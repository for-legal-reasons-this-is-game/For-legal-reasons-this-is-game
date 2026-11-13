use std::time::Duration;

use proto::{trading, trading_engine_server::TradingEngineServer};
use tonic::transport::Server;
use tradingengine::engine::Engine;
use tradingengine::service::EngineService;
use tradingengine::store::{RedisStore, StoreError};

/// How many times the engine tries to reach Redis at startup, a second apart, before it gives
/// up.
const REDIS_STARTUP_ATTEMPTS: u32 = 30;

/// Starts the engine's gRPC server on port 50051.
///
/// With `REDIS_URL` set, the engine loads its state from that Redis before it answers anything,
/// and saves each change there before it answers the command that made it. `ENGINE_INSTANCE`
/// names the instance the state belongs to, for example the StatefulSet pod name. Without
/// `REDIS_URL`, the state lives only in memory and is lost when the engine stops.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // health reporter is for setting status of server (at the moment only docker checks it, so dont change it), health server is a service for advertising
    // health so we can actually start the backend after this is ready.
    let (health_reporter, health_server) = tonic_health::server::health_reporter();

    //grpc server
    let addr = "0.0.0.0:50051".parse()?;

    // loaded before anything is served: the contract says not to answer any command until
    // the load is complete
    let tradingengine = engine_service().await?;

    // reflections, so that we can see the services available on clients or via grpccurl/grpcui
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(trading::v1::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    health_reporter
        .set_serving::<TradingEngineServer<EngineService>>()
        .await;

    // on SIGTERM or Ctrl-C the server stops taking requests and finishes the ones in
    // progress. Every command that was answered is already saved, so nothing else is written
    Server::builder()
        .add_service(reflection)
        .add_service(TradingEngineServer::new(tradingengine))
        .add_service(health_server)
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;
    println!("ENGINE: stopped");
    Ok(())
}

/// The service, with its state loaded from Redis when `REDIS_URL` is set.
async fn engine_service() -> Result<EngineService, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("REDIS_URL") else {
        println!(
            "ENGINE: REDIS_URL isn't set, so the state lives only in memory and is lost when the engine stops"
        );
        return Ok(EngineService::default());
    };
    let instance = std::env::var("ENGINE_INSTANCE")
        .ok()
        .filter(|instance| !instance.is_empty())
        .ok_or("REDIS_URL is set, so ENGINE_INSTANCE must name this engine instance")?;

    let mut attempt = 1;
    loop {
        match load(&url, &instance).await {
            Ok((engine, store)) => {
                println!("ENGINE: loaded the state of instance {instance} from redis");
                return Ok(EngineService::with_store(engine, store));
            }
            // redis may still be starting, but a saved state that doesn't load won't load later
            Err(StoreError::Redis(error)) if attempt < REDIS_STARTUP_ATTEMPTS => {
                println!("ENGINE: can't reach redis yet, trying again: {error}");
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

async fn load(url: &str, instance: &str) -> Result<(Engine, RedisStore), StoreError> {
    let mut store = RedisStore::connect(url, instance).await?;
    let engine = store.load().await?;
    Ok((engine, store))
}

/// Waits for SIGTERM, which Kubernetes sends to stop a pod, or for Ctrl-C.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("listen for Ctrl-C");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    println!("ENGINE: stopping, finishing the commands in progress");
}
