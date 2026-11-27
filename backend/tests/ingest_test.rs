//! Tests for the ingest side of the consumer: engines push events over gRPC, the handler puts
//! them on one shared channel, and `ChannelSource` reads them back out. No database needed.

mod common;

use backend::consumer::ingest::{self, ChannelSource, IngestService};
use backend::consumer::stream::EventSource;
use common::relay;
use proto::event_consumer_client::EventConsumerClient;
use proto::event_consumer_server::EventConsumerServer;
use proto::{EventItem, PublishEventsResponse};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tonic::codegen::tokio_stream::wrappers::{ReceiverStream, TcpListenerStream};
use tonic::{Code, Response, Status};

// --- helpers ----------------------------------------------------------------

/// Starts the ingest server on a free port. Returns its URL, the reading end of the shared
/// channel, the token that shuts it down, and the server task.
async fn serve(
    capacity: usize,
) -> (
    String,
    ChannelSource,
    CancellationToken,
    JoinHandle<Result<(), tonic::transport::Error>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, source) = ingest::channel(capacity);
    let cancel = CancellationToken::new();
    let server = tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(EventConsumerServer::new(IngestService::new(
                tx,
                cancel.clone(),
            )))
            .serve_with_incoming_shutdown(
                TcpListenerStream::new(listener),
                cancel.clone().cancelled_owned(),
            ),
    );
    (url, source, cancel, server)
}

type Call = JoinHandle<Result<Response<PublishEventsResponse>, Status>>;

/// Plays an engine: opens one `PublishEvents` stream. The test sends items through the returned
/// sender and ends the stream by dropping it. (To break a stream, see [`relay`].)
fn open(url: &str) -> (mpsc::Sender<EventItem>, Call) {
    let (tx, rx) = mpsc::channel(1);
    let url = url.to_string();
    let call = tokio::spawn(async move {
        let mut client = EventConsumerClient::connect(url).await.expect("connect");
        client.publish_events(ReceiverStream::new(rx)).await
    });
    (tx, call)
}

fn item(sequence: u64) -> EventItem {
    EventItem {
        sequence,
        ..Default::default()
    }
}

async fn send_all(tx: &mpsc::Sender<EventItem>, sequences: impl IntoIterator<Item = u64>) {
    for n in sequences {
        tx.send(item(n)).await.expect("the stream is still open");
    }
}

/// The next item from the source, failing the test instead of hanging.
async fn next_item(source: &mut ChannelSource) -> EventItem {
    timeout(Duration::from_secs(5), source.next())
        .await
        .expect("an item within 5s")
        .expect("no error")
        .expect("the channel is still open")
}

async fn next_seqs(source: &mut ChannelSource, count: usize) -> Vec<u64> {
    let mut got = Vec::with_capacity(count);
    for _ in 0..count {
        got.push(next_item(source).await.sequence);
    }
    got
}

async fn finished(call: Call) -> Result<Response<PublishEventsResponse>, Status> {
    timeout(Duration::from_secs(5), call)
        .await
        .expect("the call returns within 5s")
        .expect("the client task did not panic")
}

// --- tests ------------------------------------------------------------------

/// Shuts the server down and checks it stops cleanly within 5 s, and that the call still open on
/// it ends with `Unavailable` (the engine is told to come back later).
async fn shutdown_cuts_the_call(
    cancel: CancellationToken,
    server: JoinHandle<Result<(), tonic::transport::Error>>,
    call: Call,
) {
    cancel.cancel();
    timeout(Duration::from_secs(5), server)
        .await
        .expect("the server stops within 5s")
        .unwrap()
        .expect("the server stopped cleanly");
    let status = finished(call).await.expect_err("a cut stream is an error");
    assert_eq!(status.code(), Code::Unavailable, "{status}");
}

#[tokio::test]
async fn forwards_items_in_order() {
    let (url, mut source, _cancel, _server) = serve(16).await;
    let (tx, call) = open(&url);
    send_all(&tx, 1..=5).await;
    drop(tx);

    assert_eq!(next_seqs(&mut source, 5).await, vec![1, 2, 3, 4, 5]);
    finished(call).await.expect("a clean end is Ok");
}

#[tokio::test]
async fn empty_stream_gets_an_empty_response() {
    let (url, mut source, _cancel, _server) = serve(16).await;
    let (tx, call) = open(&url);
    drop(tx);
    finished(call).await.expect("an empty stream is Ok");

    // the server is still taking streams afterwards
    let (tx, call) = open(&url);
    send_all(&tx, [7]).await;
    drop(tx);
    assert_eq!(next_item(&mut source).await.sequence, 7);
    finished(call).await.expect("a clean end is Ok");
}

#[tokio::test]
async fn two_streams_both_arrive_and_each_keeps_its_own_order() {
    let (url, mut source, _cancel, _server) = serve(4).await;
    let (tx_a, call_a) = open(&url);
    let (tx_b, call_b) = open(&url);
    let send_a = tokio::spawn(async move { send_all(&tx_a, 1..=50).await });
    let send_b = tokio::spawn(async move { send_all(&tx_b, 101..=150).await });

    let got = next_seqs(&mut source, 100).await;
    let a: Vec<u64> = got.iter().copied().filter(|n| *n <= 50).collect();
    let b: Vec<u64> = got.iter().copied().filter(|n| *n > 100).collect();
    assert_eq!(a, (1..=50).collect::<Vec<_>>());
    assert_eq!(b, (101..=150).collect::<Vec<_>>());

    send_a.await.unwrap();
    send_b.await.unwrap();
    finished(call_a).await.expect("a clean end is Ok");
    finished(call_b).await.expect("a clean end is Ok");
}

#[tokio::test]
async fn a_broken_stream_does_not_affect_another() {
    let (url, mut source, _cancel, _server) = serve(16).await;
    let (relay_url, relay_task) = relay(&url).await;
    let (tx_a, call_a) = open(&relay_url);
    let (tx_b, call_b) = open(&url);

    // both streams are open and have delivered something
    send_all(&tx_a, [1]).await;
    send_all(&tx_b, [101]).await;
    let mut first = next_seqs(&mut source, 2).await;
    first.sort();
    assert_eq!(first, vec![1, 101]);

    // the first engine's connection dies in the middle of its stream (tx_a stays alive, so the
    // stream never ends cleanly)
    relay_task.abort();
    let _ = relay_task.await;
    finished(call_a)
        .await
        .expect_err("the cut stream is an error for the engine");

    send_all(&tx_b, 102..=150).await;
    drop(tx_b);
    assert_eq!(
        next_seqs(&mut source, 49).await,
        (102..=150).collect::<Vec<_>>()
    );
    finished(call_b).await.expect("a clean end is Ok");
}

#[tokio::test]
async fn a_full_channel_stops_reading_and_shutdown_still_completes() {
    let (url, _source, cancel, server) = serve(1).await;
    let (tx, call) = open(&url);

    // big items, so the HTTP/2 buffers fill up long before 1000 of them
    let big = "X".repeat(16 * 1024);
    let mut accepted = 0;
    for n in 1..=1000u64 {
        let item = EventItem {
            sequence: n,
            market_symbol: big.clone(),
            ..Default::default()
        };
        match timeout(Duration::from_millis(500), tx.send(item)).await {
            Ok(sent) => {
                sent.expect("the stream is still open");
                accepted += 1;
            }
            Err(_) => break, // stalled: nobody reads the source, so the server stopped reading
        }
    }
    assert!(accepted < 1000, "the client never stalled");

    shutdown_cuts_the_call(cancel, server, call).await;
}

#[tokio::test]
async fn shutdown_completes_with_a_quiet_stream_still_open() {
    let (url, _source, cancel, server) = serve(16).await;
    let (tx, call) = open(&url);
    // the stream is open (one item went through) and then goes quiet, like a calm market
    send_all(&tx, [1]).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    shutdown_cuts_the_call(cancel, server, call).await;
    drop(tx); // kept alive until here, so the stream never ended by itself
}

#[tokio::test]
async fn an_item_without_a_body_is_forwarded_untouched() {
    let (url, mut source, _cancel, _server) = serve(16).await;
    let (tx, call) = open(&url);
    tx.send(EventItem::default()).await.unwrap();
    drop(tx);

    assert_eq!(next_item(&mut source).await, EventItem::default());
    finished(call).await.expect("a clean end is Ok");
}

#[tokio::test]
async fn channel_source_ends_when_all_senders_are_gone() {
    let (tx, mut source) = ingest::channel(4);
    tx.send(item(1)).await.unwrap();
    tx.send(item(2)).await.unwrap();
    drop(tx);

    assert_eq!(next_seqs(&mut source, 2).await, vec![1, 2]);
    assert_eq!(source.next().await.unwrap(), None);
}
