use crate::consumer::Config;
use proto::EventItem;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep, sleep_until};
use tokio_util::sync::CancellationToken;
use tonic::Status;

// Waits between attempts to resubscribe: doubles up to a cap. The caller passes the jitter
// (0.0..=1.0) so tests are deterministic; a delay is shortened by at most half.
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

// What the router sends: a source of events. A trait so tests can stand in for the router.
pub trait EventSource {
    fn next(&mut self) -> impl Future<Output = Result<Option<EventItem>, Status>> + Send;
}

impl EventSource for tonic::Streaming<EventItem> {
    fn next(&mut self) -> impl Future<Output = Result<Option<EventItem>, Status>> + Send {
        self.message()
    }
}

pub enum BatchRead {
    // a batch that closed because it was full or because the wait ran out; the stream goes on
    Items(Vec<EventItem>),
    // the stream ended cleanly / broke: the items read so far, then resubscribe
    Ended(Vec<EventItem>),
    Failed(Vec<EventItem>, Status),
}

// Reads events until the batch has `max_items`, or `max_wait` has passed since its first item,
// whichever comes first. Waits for the first item without a deadline, so it never returns an
// empty `Items`. Dropping the call (on shutdown) drops the partial batch, which a replay brings
// back.
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

#[derive(Default)]
pub struct Stats {
    pub reconnects: AtomicU64,
}

pub struct SourceSettings {
    pub max_items: usize,
    pub max_wait: Duration,
    pub backoff_min: Duration,
    pub backoff_max: Duration,
    pub jitter: fn() -> f64,
}

fn clock_jitter() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or_default();
    (nanos % 1000) as f64 / 1000.0
}

impl SourceSettings {
    pub fn from_config(cfg: &Config) -> Self {
        SourceSettings {
            max_items: cfg.batch_max_items,
            max_wait: cfg.batch_max_wait,
            backoff_min: Duration::from_millis(200),
            backoff_max: Duration::from_secs(30),
            jitter: clock_jitter,
        }
    }
}

// false if shutdown came first
async fn pause(delay: Duration, cancel: &CancellationToken) -> bool {
    tokio::select! {
        _ = cancel.cancelled() => false,
        _ = sleep(delay) => true,
    }
}

// Subscribes, batches, and hands the batches to the pipeline. When the subscription fails, ends or
// breaks it flushes what it has, waits (backoff) and subscribes again, forever: the router sends
// the whole retained history again and everything downstream is safe to repeat. Returns only on
// shutdown or when the pipeline has gone away.
pub async fn run_source<S, F, Fut>(
    mut connect: F,
    settings: SourceSettings,
    out: mpsc::Sender<Vec<EventItem>>,
    cancel: CancellationToken,
    stats: Arc<Stats>,
) where
    S: EventSource + Send,
    F: FnMut() -> Fut + Send,
    Fut: Future<Output = Result<S, Status>> + Send,
{
    let mut backoff = Backoff::new(settings.backoff_min, settings.backoff_max);
    loop {
        let connected = tokio::select! {
            _ = cancel.cancelled() => return,
            c = connect() => c,
        };
        match connected {
            Err(status) => {
                eprintln!("CONSUMER: could not subscribe to the event stream: {status}");
            }
            Ok(mut source) => loop {
                let read = tokio::select! {
                    _ = cancel.cancelled() => return,
                    r = collect_batch(&mut source, settings.max_items.max(1), settings.max_wait) => r,
                };
                let (batch, stop) = match read {
                    BatchRead::Items(b) => (b, None),
                    BatchRead::Ended(b) => (b, Some("the event stream ended".to_string())),
                    BatchRead::Failed(b, status) => {
                        (b, Some(format!("the event stream broke: {status}")))
                    }
                };
                if !batch.is_empty() {
                    backoff.reset();
                    if out.send(batch).await.is_err() {
                        return;
                    }
                }
                if let Some(why) = stop {
                    eprintln!("CONSUMER: {why}");
                    break;
                }
            },
        }
        stats.reconnects.fetch_add(1, Ordering::Relaxed);
        if !pause(backoff.next_delay((settings.jitter)()), &cancel).await {
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

    // --- batching and resubscribing, with a scripted fake in place of the router ---------------

    use proto::EventItem;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
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

    fn settings() -> SourceSettings {
        SourceSettings {
            max_items: 10,
            max_wait: ms(50),
            backoff_min: ms(100),
            backoff_max: ms(1000),
            jitter: || 0.0,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn run_source_resubscribes_after_end_error_and_failed_connect() {
        let connections: Arc<Mutex<VecDeque<Result<Fake, Status>>>> = Arc::new(Mutex::new(
            vec![
                Ok(fake(vec![Step::Item(1), Step::End])),
                Err(Status::unavailable("no router yet")),
                Ok(fake(vec![Step::Item(2), Step::Fail])),
                Ok(fake(vec![Step::Item(3)])), // then idle
            ]
            .into(),
        ));
        let calls = Arc::new(Mutex::new(0u32));
        let stats = Arc::new(Stats::default());
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();

        let (conns, n) = (connections.clone(), calls.clone());
        let task = tokio::spawn(run_source(
            move || {
                *n.lock().unwrap() += 1;
                let next = conns
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("no more scripted connections");
                async move { next }
            },
            settings(),
            tx,
            cancel.clone(),
            stats.clone(),
        ));

        let start = Instant::now();
        let mut got = vec![];
        for _ in 0..3 {
            got.extend(seqs(&rx.recv().await.unwrap()));
        }
        assert_eq!(got, vec![1, 2, 3]);
        // waits: 100ms after the first end (backoff was reset by its batch), 200ms after the
        // failed connect, 100ms after the failed stream (reset by its batch)
        let waited = start.elapsed();
        assert!(waited >= ms(400) && waited < ms(500), "{waited:?}");
        assert_eq!(*calls.lock().unwrap(), 4);
        assert_eq!(
            stats.reconnects.load(std::sync::atomic::Ordering::Relaxed),
            3
        );

        cancel.cancel();
        task.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn run_source_stops_when_the_pipeline_is_gone() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let task = tokio::spawn(run_source(
            || async { Ok(fake(vec![Step::Item(1)])) },
            settings(),
            tx,
            CancellationToken::new(),
            Arc::new(Stats::default()),
        ));
        task.await.unwrap(); // returns instead of looping forever
    }
}
