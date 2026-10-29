use std::thread;

use proto::Market;
use tokio::sync::{mpsc, oneshot};

use crate::error::Result;
use crate::market::MarketInfo;

// how many jobs can wait for one market before a sender has to wait too
const QUEUE_SIZE: usize = 1024;

// one thing to do on one market, plus where to send the answer.
// every command for a market goes through here (place, cancel and modify
// get their own variants when they are implemented)
#[derive(Debug)]
enum Job {
    Reconfigure {
        market: Market,
        reply: oneshot::Sender<Result<()>>,
    },
    Snapshot {
        reply: oneshot::Sender<MarketInfo>,
    },
}

// each market runs on its own thread and is the only owner of its book,
// so no lock is needed around it. the handle is how everyone else talks to
// that thread: it puts a job on the queue and waits for the answer.
// jobs run one at a time in the order they were queued
#[derive(Debug, Clone)]
pub struct MarketHandle {
    jobs: mpsc::Sender<Job>,
}

impl MarketHandle {
    // starts the thread for this market. the thread stops on its own when
    // every handle to it has been dropped
    pub fn spawn(info: MarketInfo) -> MarketHandle {
        let (jobs, inbox) = mpsc::channel(QUEUE_SIZE);
        let name = format!("market-{}", info.market().symbol);

        // as decided for the engine, a failure we cannot handle panics (and aborts)
        thread::Builder::new()
            .name(name)
            .spawn(move || run(info, inbox))
            .expect("could not start a market thread");

        MarketHandle { jobs }
    }

    pub async fn reconfigure(&self, market: Market) -> Result<()> {
        let (reply, answer) = oneshot::channel();
        self.send(Job::Reconfigure { market, reply }).await;
        answer.await.expect("market thread dropped the reply")
    }

    // a copy of the market as it is right now
    pub async fn snapshot(&self) -> MarketInfo {
        let (reply, answer) = oneshot::channel();
        self.send(Job::Snapshot { reply }).await;
        answer.await.expect("market thread dropped the reply")
    }

    async fn send(&self, job: Job) {
        self.jobs
            .send(job)
            .await
            .expect("market thread has stopped");
    }
}

// the market thread: take the next job, do it, answer, repeat.
// recv returns None once every handle is gone, which ends the thread.
// if the caller gave up waiting the answer is dropped, the job still counts
fn run(mut info: MarketInfo, mut inbox: mpsc::Receiver<Job>) {
    while let Some(job) = inbox.blocking_recv() {
        match job {
            Job::Reconfigure { market, reply } => {
                let _ = reply.send(info.reconfigure(market));
            }
            Job::Snapshot { reply } => {
                let _ = reply.send(info.clone());
            }
        }
    }
}
