//! The engine pushes its events to the consumer (`proto/consumer.proto`). These tests run a
//! fake consumer that writes down every event it reads, and which stream it came on.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use proto::event_consumer_server::{EventConsumer, EventConsumerServer};
use proto::{EventItem, PublishEventsResponse};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tonic::codegen::tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};
use tradingengine::event_sender::EventSender;

/// (which stream, starting at 0, the event's `sequence`)
type Read = (usize, u64);

struct FakeConsumer {
    read: mpsc::UnboundedSender<Read>,
    streams: Arc<AtomicUsize>,
    /// the first stream breaks after reading this many events
    break_first_after: Option<usize>,
}

#[tonic::async_trait]
impl EventConsumer for FakeConsumer {
    async fn publish_events(
        &self,
        request: Request<Streaming<EventItem>>,
    ) -> Result<Response<PublishEventsResponse>, Status> {
        let stream_number = self.streams.fetch_add(1, Ordering::SeqCst);
        let mut stream = request.into_inner();
        let mut count = 0;
        while let Some(event) = stream.message().await? {
            let _ = self.read.send((stream_number, event.sequence));
            count += 1;
            if stream_number == 0 && Some(count) == self.break_first_after {
                return Err(Status::unavailable("the test breaks the first stream"));
            }
        }
        Ok(Response::new(PublishEventsResponse {}))
    }
}

fn start_consumer(
    listener: TcpListener,
    break_first_after: Option<usize>,
) -> mpsc::UnboundedReceiver<Read> {
    let (read, reads) = mpsc::unbounded_channel();
    let consumer = FakeConsumer {
        read,
        streams: Arc::new(AtomicUsize::new(0)),
        break_first_after,
    };
    tokio::spawn(
        Server::builder()
            .add_service(EventConsumerServer::new(consumer))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );
    reads
}

async fn listener() -> TcpListener {
    TcpListener::bind("127.0.0.1:0").await.expect("bind")
}

fn url(listener: &TcpListener) -> String {
    format!("http://{}", listener.local_addr().expect("address"))
}

fn events(sequences: impl IntoIterator<Item = u64>) -> Vec<EventItem> {
    sequences
        .into_iter()
        .map(|sequence| EventItem {
            sequence,
            ..Default::default()
        })
        .collect()
}

/// Waits for the next `n` events the consumer reads.
async fn next_reads(reads: &mut mpsc::UnboundedReceiver<Read>, n: usize) -> Vec<Read> {
    let mut got = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), async {
        while got.len() < n {
            got.push(reads.recv().await.expect("the consumer is running"));
        }
    })
    .await
    .expect("the consumer got the events in time");
    got
}

#[tokio::test]
async fn the_kept_events_go_first_then_the_new_ones_in_order() {
    let listener = listener().await;
    let url = url(&listener);
    let mut reads = start_consumer(listener, None);

    let (sender, new_events) = EventSender::new(&url, events(1..=3)).expect("sender");
    tokio::spawn(sender.run());
    new_events
        .send(events(4..=5))
        .expect("the sender is running");

    let got = next_reads(&mut reads, 5).await;
    assert_eq!(got, vec![(0, 1), (0, 2), (0, 3), (0, 4), (0, 5)]);
}

#[tokio::test]
async fn new_events_go_out_on_the_open_stream() {
    let listener = listener().await;
    let url = url(&listener);
    let mut reads = start_consumer(listener, None);

    let (sender, new_events) = EventSender::new(&url, events(1..=1)).expect("sender");
    tokio::spawn(sender.run());
    assert_eq!(next_reads(&mut reads, 1).await, vec![(0, 1)]);

    new_events
        .send(events(2..=2))
        .expect("the sender is running");
    assert_eq!(next_reads(&mut reads, 1).await, vec![(0, 2)]);
}

/// The contract: when the stream breaks, the engine connects again and sends its whole
/// history again. The consumer skips what it already has.
#[tokio::test]
async fn after_a_broken_stream_everything_is_sent_again() {
    let listener = listener().await;
    let url = url(&listener);
    let mut reads = start_consumer(listener, Some(2));

    let (sender, _new_events) = EventSender::new(&url, events(1..=3)).expect("sender");
    tokio::spawn(sender.run());

    let got = next_reads(&mut reads, 5).await;
    assert_eq!(got, vec![(0, 1), (0, 2), (1, 1), (1, 2), (1, 3)]);
}

#[tokio::test]
async fn a_consumer_that_starts_later_still_gets_everything() {
    // take a free port and close it again, so the first connects fail
    let address = listener().await.local_addr().expect("address");
    let (sender, new_events) =
        EventSender::new(&format!("http://{address}"), events(1..=2)).expect("sender");
    tokio::spawn(sender.run());
    new_events
        .send(events(3..=3))
        .expect("the sender is running");

    tokio::time::sleep(Duration::from_millis(300)).await;
    let listener = TcpListener::bind(address)
        .await
        .expect("bind the same port");
    let mut reads = start_consumer(listener, None);

    let got = next_reads(&mut reads, 3).await;
    assert_eq!(got, vec![(0, 1), (0, 2), (0, 3)]);
}
