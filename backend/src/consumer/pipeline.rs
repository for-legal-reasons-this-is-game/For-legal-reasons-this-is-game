// The three stages of the consumer, joined by bounded queues so that while one batch is being
// saved in Postgres the next one is already moving money in TigerBeetle and the one after is
// being prepared:
//
//   source -> [prepare] -> [ledger] -> [commit]
//
// Every stage handles one batch at a time and the queues are first-in first-out, so batches keep
// the order the engine sent them in. A full queue makes the stage before it wait, and that wait
// travels back to the gRPC stream, so a slow stage slows the stream instead of piling up memory.
//
// Shutdown and failure need no special messages: when the source ends, each stage drains its queue
// and ends, which closes the next queue. When a stage fails it cancels the token (stopping the
// source) and ends; the stages before it find their output closed and stop, and the stages after
// it still finish what was already handed to them, so money that moved always gets recorded.
use crate::consumer::ConsumerError;
use crate::consumer::plan::BatchPlan;
use proto::EventItem;
use std::future::Future;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

async fn run_stage<I, O, F, Fut>(
    name: &'static str,
    mut input: mpsc::Receiver<I>,
    output: Option<mpsc::Sender<O>>,
    mut work: F,
    cancel: CancellationToken,
) -> Result<(), ConsumerError>
where
    F: FnMut(I) -> Fut,
    Fut: Future<Output = Result<O, ConsumerError>>,
{
    while let Some(item) = input.recv().await {
        match work(item).await {
            Ok(out) => {
                if let Some(next) = &output
                    && next.send(out).await.is_err()
                {
                    // the next stage is gone; its own error is the one to report
                    return Ok(());
                }
            }
            Err(e) => {
                eprintln!("CONSUMER: the {name} stage failed: {e}");
                cancel.cancel();
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Runs the source and the three stages until they all end.
///
/// Returns the first stage error, if any.
pub async fn run_pipeline<Src, SrcFut, P, PFut, L, LFut, C, CFut>(
    source: Src,
    prepare: P,
    ledger: L,
    commit: C,
    queue_depth: usize,
    cancel: CancellationToken,
) -> Result<(), ConsumerError>
where
    Src: FnOnce(mpsc::Sender<Vec<EventItem>>) -> SrcFut,
    SrcFut: Future<Output = ()>,
    P: FnMut(Vec<EventItem>) -> PFut,
    PFut: Future<Output = Result<BatchPlan, ConsumerError>>,
    L: FnMut(BatchPlan) -> LFut,
    LFut: Future<Output = Result<BatchPlan, ConsumerError>>,
    C: FnMut(BatchPlan) -> CFut,
    CFut: Future<Output = Result<(), ConsumerError>>,
{
    let depth = queue_depth.max(1);
    let (events_tx, events_rx) = mpsc::channel(depth);
    let (prepared_tx, prepared_rx) = mpsc::channel(depth);
    let (settled_tx, settled_rx) = mpsc::channel(depth);

    let (_, prepared, settled, committed) = tokio::join!(
        source(events_tx),
        run_stage(
            "prepare",
            events_rx,
            Some(prepared_tx),
            prepare,
            cancel.clone()
        ),
        run_stage(
            "ledger",
            prepared_rx,
            Some(settled_tx),
            ledger,
            cancel.clone()
        ),
        run_stage(
            "commit",
            settled_rx,
            None::<mpsc::Sender<()>>,
            commit,
            cancel.clone()
        ),
    );
    prepared.and(settled).and(committed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumer::plan::BatchPlan;
    use proto::EventItem;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    fn batch(n: u64) -> Vec<EventItem> {
        vec![EventItem {
            sequence: n,
            ..Default::default()
        }]
    }

    /// A source that sends `count` batches and then ends, like a stream that was cancelled.
    async fn send_batches(tx: mpsc::Sender<Vec<EventItem>>, count: u64) {
        for n in 0..count {
            if tx.send(batch(n)).await.is_err() {
                return;
            }
        }
    }

    #[tokio::test]
    async fn queued_batches_all_reach_commit_when_the_source_ends() {
        let committed = Arc::new(AtomicUsize::new(0));
        let c = committed.clone();
        let result = run_pipeline(
            |tx| send_batches(tx, 5),
            |_| async { Ok(BatchPlan::default()) },
            |plan| async { Ok(plan) },
            move |_| {
                let c = c.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(10)).await; // a slow commit lets the queues fill
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            2,
            CancellationToken::new(),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(committed.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn a_failing_stage_cancels_the_source_and_returns_its_error() {
        let cancel = CancellationToken::new();
        let source_stopped = Arc::new(AtomicBool::new(false));
        let (flag, token) = (source_stopped.clone(), cancel.clone());
        let ledger_calls = Arc::new(AtomicUsize::new(0));
        let calls = ledger_calls.clone();

        let result = run_pipeline(
            move |tx| async move {
                // sends forever, until the pipeline tells it to stop
                let mut n = 0;
                loop {
                    tokio::select! {
                        _ = token.cancelled() => break,
                        sent = tx.send(batch(n)) => if sent.is_err() { break },
                    }
                    n += 1;
                }
                flag.store(true, Ordering::SeqCst);
            },
            |_| async { Ok(BatchPlan::default()) },
            move |plan| {
                let calls = calls.clone();
                async move {
                    if calls.fetch_add(1, Ordering::SeqCst) == 2 {
                        return Err("tigerbeetle said no".into());
                    }
                    Ok(plan)
                }
            },
            |_| async { Ok(()) },
            2,
            cancel.clone(),
        )
        .await;

        let err = result.unwrap_err();
        assert!(err.to_string().contains("tigerbeetle said no"), "{err}");
        assert!(
            source_stopped.load(Ordering::SeqCst),
            "the source kept running"
        );
        assert!(cancel.is_cancelled());
    }

    #[tokio::test]
    async fn batches_that_passed_the_ledger_are_committed_even_if_a_later_one_fails() {
        let committed = Arc::new(AtomicUsize::new(0));
        let c = committed.clone();
        let ledger_calls = Arc::new(AtomicUsize::new(0));
        let calls = ledger_calls.clone();
        let result = run_pipeline(
            |tx| send_batches(tx, 10),
            |_| async { Ok(BatchPlan::default()) },
            move |plan| {
                let calls = calls.clone();
                async move {
                    if calls.fetch_add(1, Ordering::SeqCst) == 2 {
                        return Err("third batch failed".into());
                    }
                    Ok(plan)
                }
            },
            move |_| {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
            2,
            CancellationToken::new(),
        )
        .await;
        assert!(result.is_err());
        // the money of the first two already moved, so their records must be written
        assert_eq!(committed.load(Ordering::SeqCst), 2);
    }
}
