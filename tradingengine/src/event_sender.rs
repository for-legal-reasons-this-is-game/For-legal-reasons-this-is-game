//! sends the engines events to the consumer, see `proto/consumer.proto`
//!
//! the engine is the client. it keeps one `PublishEvents` stream open to the consumer.
//! on every connect it sends all the events it kept, oldest first, then the new ones
//! as the engine makes them. when the stream breaks it waits a bit, connects again and
//! sends everything again. the consumer skips what it already has

use std::collections::VecDeque;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use proto::event_consumer_client::EventConsumerClient;
use proto::{EventItem, PublishEventsResponse};
use tokio::sync::mpsc;
use tokio::task::JoinError;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Channel, Endpoint};
use tonic::{Response, Status};

use crate::engine::EVENT_HISTORY_CAPACITY;

/// the consumer pings every 20s and waits 10s for the answer (`proto/devops.md`).
/// the engine does the same so a dead connection is noticed from both sides
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// the wait before connecting again. it doubles after every try that sent nothing,
/// up to the max
const FIRST_RETRY_WAIT: Duration = Duration::from_millis(100);
const MAX_RETRY_WAIT: Duration = Duration::from_secs(10);
/// how many events can wait in the stream before a send waits for the consumer
const STREAM_BUFFER: usize = 64;

/// where the engine hands its new events to the [`EventSender`]. a send never waits,
/// so a slow or missing consumer can't slow down matching
pub type NewEvents = mpsc::UnboundedSender<Vec<EventItem>>;

pub struct EventSender {
    endpoint: Endpoint,
    /// the same events the engine keeps, oldest first
    history: VecDeque<EventItem>,
    new_events: mpsc::UnboundedReceiver<Vec<EventItem>>,
}

/// how one connection ended
enum Ended {
    /// the engine dropped its [`NewEvents`], nothing more will come
    EngineStopped,
    /// the stream broke or the consumer closed it. `sent` is how many events went out
    Stream { reason: String, sent: usize },
}

impl EventSender {
    /// `history` is what the engine kept so far, it goes out first on every connect
    pub fn new(
        consumer_url: &str,
        history: impl IntoIterator<Item = EventItem>,
    ) -> Result<(EventSender, NewEvents), tonic::transport::Error> {
        let endpoint = Endpoint::from_shared(consumer_url.to_string())?
            .connect_timeout(CONNECT_TIMEOUT)
            .http2_keep_alive_interval(KEEPALIVE_INTERVAL)
            .keep_alive_timeout(KEEPALIVE_TIMEOUT)
            .keep_alive_while_idle(true);
        let (new_events, receiver) = mpsc::unbounded_channel();
        let mut sender = EventSender {
            endpoint,
            history: VecDeque::new(),
            new_events: receiver,
        };
        sender.keep(history.into_iter().collect(), &mut 0);
        Ok((sender, new_events))
    }

    /// connects and sends until the engine drops its [`NewEvents`]
    pub async fn run(mut self) {
        let mut retry_wait = FIRST_RETRY_WAIT;
        loop {
            match self.endpoint.connect().await {
                Ok(channel) => match self.publish(EventConsumerClient::new(channel)).await {
                    Ended::EngineStopped => return,
                    Ended::Stream { reason, sent } => {
                        eprintln!("event stream to the consumer ended, connecting again: {reason}");
                        if sent > 0 {
                            retry_wait = FIRST_RETRY_WAIT;
                        }
                    }
                },
                Err(error) => eprintln!("can't connect to the consumer, trying again: {error}"),
            }

            if !self.wait(retry_wait).await {
                return;
            }
            retry_wait = (retry_wait * 2).min(MAX_RETRY_WAIT);
        }
    }

    /// one connection. sends the whole history, then every new event as it comes
    async fn publish(&mut self, mut client: EventConsumerClient<Channel>) -> Ended {
        let (stream, items) = mpsc::channel(STREAM_BUFFER);
        let mut call =
            tokio::spawn(async move { client.publish_events(ReceiverStream::new(items)).await });

        let mut next = 0; // the history position of the next event to send
        let mut sent = 0;
        loop {
            while let Some(event) = self.history.get(next) {
                // a send fails when the stream is gone, the call says why
                if stream.send(event.clone()).await.is_err() {
                    let reason = why_it_ended(call.await);
                    return Ended::Stream { reason, sent };
                }
                next += 1;
                sent += 1;
            }

            tokio::select! {
                events = self.new_events.recv() => match events {
                    Some(events) => self.keep(events, &mut next),
                    None => return Ended::EngineStopped,
                },
                result = &mut call => {
                    let reason = why_it_ended(result);
                    return Ended::Stream { reason, sent };
                }
            }
        }
    }

    /// waits before the next connect, but keeps taking new events so they don't pile
    /// up. false when the engine stopped
    async fn wait(&mut self, duration: Duration) -> bool {
        let sleep = tokio::time::sleep(duration);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                () = &mut sleep => return true,
                events = self.new_events.recv() => match events {
                    Some(events) => self.keep(events, &mut 0),
                    None => return false,
                },
            }
        }
    }

    /// adds new events to the history and drops the oldest past the capacity, like the
    /// engine does. `next` moves with them so it still points at the same event
    fn keep(&mut self, events: Vec<EventItem>, next: &mut usize) {
        self.history.extend(events);
        let extra = self.history.len().saturating_sub(EVENT_HISTORY_CAPACITY);
        self.history.drain(..extra);
        *next = next.saturating_sub(extra);
    }
}

fn why_it_ended(
    result: Result<Result<Response<PublishEventsResponse>, Status>, JoinError>,
) -> String {
    match result {
        Ok(Ok(_)) => "the consumer closed the stream".to_string(),
        Ok(Err(status)) => status.to_string(),
        Err(error) => error.to_string(),
    }
}

// adds up to half of `wait` on top, so engines that lost the consumer at the same
// time don't all connect again at the same moment (not used currently)
//fn with_jitter(wait: Duration) -> Duration {
//    let nanos = SystemTime::now()
//        .duration_since(UNIX_EPOCH)
//        .map_or(0, |time| time.subsec_nanos());
//    wait + wait.mul_f64(f64::from(nanos % 1000) / 2000.0)
//}
