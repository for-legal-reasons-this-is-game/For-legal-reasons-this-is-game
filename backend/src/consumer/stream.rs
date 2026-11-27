use proto::EventItem;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;
use tonic::Status;

/// Waits between attempts to resubscribe, doubling up to a cap.
///
/// The caller passes the jitter (0.0..=1.0), so tests are deterministic. A delay is shortened
/// by at most half.
pub struct Backoff {
    min: Duration,
    max: Duration,
    current: Duration,
}

impl Backoff {
    pub fn new(min: Duration, max: Duration) -> Self {
        Backoff {
            min,
            max,
            current: min,
        }
    }

    pub fn next_delay(&mut self, jitter: f64) -> Duration {
        let delay = self.current;
        self.current = (self.current * 2).min(self.max);
        delay.mul_f64(1.0 - 0.5 * jitter.clamp(0.0, 1.0))
    }

    pub fn reset(&mut self) {
        self.current = self.min;
    }
}

/// What the router sends: a source of events. It's a trait, so tests can stand in for the router.
pub trait EventSource {
    fn next(&mut self) -> impl Future<Output = Result<Option<EventItem>, Status>> + Send;
}

impl EventSource for tonic::Streaming<EventItem> {
    fn next(&mut self) -> impl Future<Output = Result<Option<EventItem>, Status>> + Send {
        self.message()
    }
}

pub fn clock_jitter() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or_default();
    (nanos % 1000) as f64 / 1000.0
}

pub enum BatchRead {
    /// A batch that closed because it was full or because the wait ran out. The stream goes on.
    Items(Vec<EventItem>),
    /// The stream ended cleanly: the items read so far, then resubscribe.
    Ended(Vec<EventItem>),
    /// The stream broke: the items read so far, then resubscribe.
    Failed(Vec<EventItem>, Status),
}

/// Reads events until the batch has `max_items`, or `max_wait` has passed since its first item,
/// whichever comes first.
///
/// Waits for the first item without a deadline, so it never returns an empty
/// [`Items`](BatchRead::Items). Dropping the call (on shutdown) drops the partial batch, which a
/// replay brings back.
pub async fn collect_batch<S: EventSource>(
    source: &mut S,
    max_items: usize,
    max_wait: Duration,
) -> BatchRead {
    let mut batch = Vec::new();
    match source.next().await {
        Ok(Some(item)) => batch.push(item),
        Ok(None) => return BatchRead::Ended(batch),
        Err(status) => return BatchRead::Failed(batch, status),
    }
    let deadline = Instant::now() + max_wait;
    while batch.len() < max_items {
        tokio::select! {
            read = source.next() => match read {
                Ok(Some(item)) => batch.push(item),
                Ok(None) => return BatchRead::Ended(batch),
                Err(status) => return BatchRead::Failed(batch, status),
            },
            _ = sleep_until(deadline) => break,
        }
    }
    BatchRead::Items(batch)
}

/// Batches the events of one source and hands the batches to the pipeline.
///
/// Returns when the source ends or breaks (after flushing what it has), on shutdown (dropping the
/// half-read batch, which a replay brings back), or when the pipeline has gone away.
pub async fn run_batcher<S: EventSource + Send>(
    mut source: S,
    max_items: usize,
    max_wait: Duration,
    out: mpsc::Sender<Vec<EventItem>>,
    cancel: CancellationToken,
) {
    loop {
        let read = tokio::select! {
            _ = cancel.cancelled() => return,
            r = collect_batch(&mut source, max_items.max(1), max_wait) => r,
        };
        let (batch, over) = match read {
            BatchRead::Items(b) => (b, false),
            BatchRead::Ended(b) | BatchRead::Failed(b, _) => (b, true),
        };
        if !batch.is_empty() {
            // A full pipeline makes the send wait, but shutdown wins: the unsent batch is dropped
            // and a replay brings it back.
            let sent = tokio::select! {
                biased;
                _ = cancel.cancelled() => return,
                s = out.send(batch) => s,
            };
            if sent.is_err() {
                return;
            }
        }
        if over {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn delay_doubles_until_cap() {
        let mut b = Backoff::new(ms(100), ms(1000));
        let got: Vec<Duration> = (0..6).map(|_| b.next_delay(0.0)).collect();
        assert_eq!(
            got,
            vec![ms(100), ms(200), ms(400), ms(800), ms(1000), ms(1000)]
        );
    }

    #[test]
    fn jitter_only_shortens() {
        let mut b = Backoff::new(ms(100), ms(1000));
        let d = b.next_delay(1.0);
        assert!(d >= ms(50) && d <= ms(100), "got {d:?}");
    }

    #[test]
    fn reset_returns_to_min() {
        let mut b = Backoff::new(ms(100), ms(1000));
        b.next_delay(0.0);
        b.next_delay(0.0);
        b.reset();
        assert_eq!(b.next_delay(0.0), ms(100));
    }

    // --- batching, with a scripted fake in place of the router ----------------------------------

    use proto::EventItem;
    use std::collections::VecDeque;
    use tokio::sync::mpsc;
    use tokio::time::Instant;
    use tokio_util::sync::CancellationToken;
    use tonic::Status;

    enum Step {
        Item(u64),
        Delay(Duration),
        End,
        Fail,
    }

    struct Fake(VecDeque<Step>);

    impl EventSource for Fake {
        async fn next(&mut self) -> Result<Option<EventItem>, Status> {
            loop {
                match self.0.front() {
                    None => std::future::pending::<()>().await, // an idle stream
                    Some(Step::Delay(d)) => {
                        let d = *d;
                        tokio::time::sleep(d).await;
                        self.0.pop_front();
                    }
                    Some(Step::Item(n)) => {
                        let item = EventItem {
                            sequence: *n,
                            ..Default::default()
                        };
                        self.0.pop_front();
                        return Ok(Some(item));
                    }
                    Some(Step::End) => {
                        self.0.pop_front();
                        return Ok(None);
                    }
                    Some(Step::Fail) => {
                        self.0.pop_front();
                        return Err(Status::unavailable("router went away"));
                    }
                }
            }
        }
    }

    fn fake(steps: Vec<Step>) -> Fake {
        Fake(steps.into())
    }

    fn seqs(items: &[EventItem]) -> Vec<u64> {
        items.iter().map(|i| i.sequence).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn batch_closes_at_max_items() {
        let mut src = fake((1..=5).map(Step::Item).collect());
        match collect_batch(&mut src, 3, ms(50)).await {
            BatchRead::Items(b) => assert_eq!(seqs(&b), vec![1, 2, 3]),
            _ => panic!("expected a full batch"),
        }
        match collect_batch(&mut src, 3, ms(50)).await {
            BatchRead::Items(b) => assert_eq!(seqs(&b), vec![4, 5]),
            _ => panic!("expected the rest after the wait"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn batch_closes_at_max_wait_with_the_items_it_has() {
        let mut src = fake(vec![
            Step::Item(1),
            Step::Item(2),
            Step::Delay(Duration::from_secs(1)),
            Step::Item(3),
        ]);
        let start = Instant::now();
        match collect_batch(&mut src, 10, ms(50)).await {
            BatchRead::Items(b) => assert_eq!(seqs(&b), vec![1, 2]),
            _ => panic!("expected a partial batch"),
        }
        let waited = start.elapsed();
        assert!(waited >= ms(50) && waited < ms(1000), "{waited:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn an_idle_stream_never_yields_an_empty_batch() {
        let mut src = fake(vec![Step::Delay(Duration::from_secs(10)), Step::Item(1)]);
        match collect_batch(&mut src, 10, ms(50)).await {
            BatchRead::Items(b) => assert_eq!(seqs(&b), vec![1]),
            _ => panic!("expected the first item"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn stream_end_and_error_return_the_partial_batch() {
        let mut src = fake(vec![Step::Item(1), Step::Item(2), Step::End]);
        assert!(
            matches!(collect_batch(&mut src, 10, ms(50)).await, BatchRead::Ended(b) if seqs(&b) == vec![1, 2])
        );
        let mut src = fake(vec![Step::Item(1), Step::Fail]);
        assert!(
            matches!(collect_batch(&mut src, 10, ms(50)).await, BatchRead::Failed(b, _) if seqs(&b) == vec![1])
        );
    }

    #[tokio::test(start_paused = true)]
    async fn batcher_sends_full_and_timed_batches() {
        let src = fake((1..=5).map(Step::Item).collect());
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let task = tokio::spawn(run_batcher(src, 3, ms(50), tx, cancel.clone()));

        assert_eq!(seqs(&rx.recv().await.unwrap()), vec![1, 2, 3]);
        assert_eq!(seqs(&rx.recv().await.unwrap()), vec![4, 5]);

        cancel.cancel();
        task.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn batcher_flushes_the_partial_batch_when_the_source_ends() {
        let src = fake(vec![Step::Item(1), Step::Item(2), Step::End]);
        let (tx, mut rx) = mpsc::channel(8);
        let task = tokio::spawn(run_batcher(src, 10, ms(50), tx, CancellationToken::new()));

        assert_eq!(seqs(&rx.recv().await.unwrap()), vec![1, 2]);
        task.await.unwrap();
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn batcher_stops_on_cancel_and_drops_the_partial_batch() {
        // one item, then idle: the batch stays open until the wait runs out
        let src = fake(vec![Step::Item(1)]);
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let task = tokio::spawn(run_batcher(
            src,
            10,
            Duration::from_secs(60),
            tx,
            cancel.clone(),
        ));

        tokio::time::sleep(ms(10)).await;
        cancel.cancel();
        task.await.unwrap();
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn batcher_stops_on_cancel_while_blocked_on_a_full_pipeline() {
        let (tx, rx) = mpsc::channel(1);
        tx.try_send(vec![]).unwrap(); // the queue is full and nobody receives
        let src = fake(vec![Step::Item(1), Step::Item(2)]);
        let cancel = CancellationToken::new();
        let task = tokio::spawn(run_batcher(src, 1, ms(50), tx, cancel.clone()));

        tokio::time::sleep(ms(10)).await; // the batcher is now stuck in send
        cancel.cancel();
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("the batcher did not stop on cancel")
            .unwrap();
        drop(rx); // kept alive until here, so the queue never drained
    }

    #[tokio::test(start_paused = true)]
    async fn batcher_stops_when_the_pipeline_is_gone() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let src = fake(vec![Step::Item(1)]);
        let task = tokio::spawn(run_batcher(src, 10, ms(50), tx, CancellationToken::new()));
        task.await.unwrap();
    }
}
