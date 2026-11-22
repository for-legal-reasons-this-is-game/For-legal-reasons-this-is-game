use proto::{trading, trading_engine_server::TradingEngineServer};
use tonic::transport::Server;
use tradingengine::command_cache::DEFAULT_CACHE_CAPACITY;
use tradingengine::engine::Engine;
use tradingengine::engine_state::EngineState;
use tradingengine::redis_store::RedisStore;
use tradingengine::service::EngineService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // health reporter is for setting status of server (at the moment only docker checks it, so dont change it), health server is a service for advertising
    // health so we can actually start the backend after this is ready.
    let (health_reporter, health_server) = tonic_health::server::health_reporter();

    //grpc server
    let addr = "0.0.0.0:50051".parse()?;

    let tradingengine = engine_service().await?;

    // reflections, so that we can see the services available on clients or via grpccurl/grpcui
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(trading::v1::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    health_reporter
        .set_serving::<TradingEngineServer<EngineService>>()
        .await;

    Server::builder()
        .add_service(reflection)
        .add_service(TradingEngineServer::new(tradingengine))
        .add_service(health_server)
        .serve(addr)
        .await?;
    Ok(())
}

async fn engine_service() -> Result<EngineService, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("REDIS_URL") else {
        println!("REDIS_URL is not set. using in-memory state that will be lost on restart");
        return Ok(EngineService::default());
    };
    let instance = std::env::var("ENGINE_INSTANCE")
        .ok()
        .filter(|instance| !instance.is_empty())
        .ok_or("REDIS_URL is set, but ENGINE_INSTANCE is not")?;
    let mut store = RedisStore::connect(&url, &instance).await?;
    let engine = match store.load().await? {
        // `store.load()` loads from redis and either returns the bytes of the
        // state or `None` if it's first start, and in that case we construct
        // `Engine` with default (fresh start without a saved state)
        None => Engine::default(),
        Some(state_bytes) => Engine::load(
            EngineState::from_bytes(&state_bytes)?,
            DEFAULT_CACHE_CAPACITY,
        )?,
    };
    Ok(EngineService::new(engine, Some(store)))
}
