//! Helpers shared by the integration tests that talk to the consumer's gRPC server. Each test
//! file pulls this in with `mod common;`. Cargo does not build `tests/common/` as a test of its
//! own, because it is a folder.

use tokio::task::JoinHandle;

/// A TCP relay in front of a server, for one connection. Aborting the returned task drops both
/// sockets, which is what the server sees when an engine dies: the connection is gone in the
/// middle of a stream. (Aborting the client task is not enough: its request body just ends
/// cleanly.)
pub async fn relay(server_url: &str) -> (String, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let target = server_url.trim_start_matches("http://").to_string();
    let task = tokio::spawn(async move {
        let (mut engine_side, _) = listener.accept().await.unwrap();
        let mut server_side = tokio::net::TcpStream::connect(target).await.unwrap();
        let _ = tokio::io::copy_bidirectional(&mut engine_side, &mut server_side).await;
    });
    (url, task)
}
