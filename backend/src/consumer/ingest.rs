//! The receiving side: engines open `PublishEvents` streams to the consumer and push their events.
//!
//! Every stream feeds one shared bounded channel, and the batching reads the other end through
//! [`ChannelSource`].

use crate::consumer::stream::EventSource;
use proto::event_consumer_server::EventConsumer;
use proto::{EventItem, PublishEventsResponse};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status, Streaming};

/// The shared channel: the handlers get the sender (one clone each), the batching gets the source.
pub fn channel(capacity: usize) -> (mpsc::Sender<EventItem>, ChannelSource) {
    let (tx, rx) = mpsc::channel(capacity);
    (tx, ChannelSource { rx })
}

/// The reading end of the shared channel, as an [`EventSource`] so `collect_batch` can use it.
pub struct ChannelSource {
    rx: mpsc::Receiver<EventItem>,
}

impl EventSource for ChannelSource {
    /// `Ok(None)` once every sender is gone and the channel is empty. It never fails: a broken
    /// stream is the handler's business and never reaches here.
    async fn next(&mut self) -> Result<Option<EventItem>, Status> {
        Ok(self.rx.recv().await)
    }
}

/// The gRPC service the engines talk to.
pub struct IngestService {
    tx: mpsc::Sender<EventItem>,
    cancel: CancellationToken,
}

impl IngestService {
    pub fn new(tx: mpsc::Sender<EventItem>, cancel: CancellationToken) -> IngestService {
        IngestService { tx, cancel }
    }
}

#[tonic::async_trait]
impl EventConsumer for IngestService {
    /// Reads one stream item by item and puts each on the shared channel, so that stream keeps
    /// its order.
    ///
    /// When the channel is full the handler waits and stops reading, and HTTP/2 flow control slows
    /// the engine down. A stream that ends or breaks only ends this call, never the consumer or
    /// the other streams.
    async fn publish_events(
        &self,
        mut request: Request<Streaming<EventItem>>,
    ) -> Result<Response<PublishEventsResponse>, Status> {
        let peer = request
            .remote_addr()
            .map_or_else(|| "an unknown peer".to_string(), |addr| addr.to_string());
        println!("CONSUMER: stream from {peer} opened");
        let stream = request.get_mut();
        loop {
            match stream.message().await {
                Ok(Some(item)) => {
                    // waits on the token too: a handler stuck on a full channel would otherwise
                    // keep tonic's graceful shutdown waiting forever
                    tokio::select! {
                        _ = self.cancel.cancelled() => {
                            println!("CONSUMER: stream from {peer} cut: shutting down");
                            return Err(Status::unavailable("consumer is shutting down"));
                        }
                        sent = self.tx.send(item) => if sent.is_err() {
                            eprintln!("CONSUMER: stream from {peer} cut: the pipeline is gone");
                            return Err(Status::unavailable("consumer is not processing events"));
                        }
                    }
                }
                Ok(None) => {
                    println!("CONSUMER: stream from {peer} ended");
                    return Ok(Response::new(PublishEventsResponse {}));
                }
                Err(status) => {
                    eprintln!("CONSUMER: stream from {peer} broke: {status}");
                    return Err(status);
                }
            }
        }
    }
}
