use backend::consumer::{self, Config};
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;

// Settles trades from the router's event stream. See src/consumer/ for how.
#[tokio::main]
async fn main() -> Result<(), consumer::ConsumerError> {
    // ROUTER_URL (or ENGINE_URL), DATABASE_URL, TB_CLUSTER_ID and TB_IP_ADRESSES come from the
    // process environment, like the backend's.
    let config = Config::from_env()?;

    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
        println!("CONSUMER: stopping");
        stop.cancel();
    });

    // an Err exits non-zero, so Kubernetes restarts the pod and the replay catches up
    consumer::run(config, cancel).await?;
    Ok(())
}
