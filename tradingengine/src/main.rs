use tradingengine::command_cache::DEFAULT_CACHE_CAPACITY;
use tradingengine::engine::Engine;
use tradingengine::engine_state::EngineState;
use tradingengine::event_sender::{EventSender, NewEvents};
use tradingengine::redis_store::RedisStore;
use tradingengine::router_consumer::RouterConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut consumer = create_consumer().await?;

    spawn_health_service().await?;

    loop {
        consumer.handle_next().await?
    }
}

async fn spawn_health_service() -> Result<(), Box<dyn std::error::Error>> {
    // health reporter is for setting status of server (at the moment only docker checks it, so dont change it), health server is a service for advertising
    // health so we can actually start the backend after this is ready.
    let (health_reporter, health_server) = tonic_health::server::health_reporter();
    let grpc_addr = "0.0.0.0:50051".parse()?;
    health_reporter
        .set_service_status(
            "trading.v1.TradingEngine",
            tonic_health::ServingStatus::Serving,
        )
        .await;
    tokio::spawn(
        tonic::transport::Server::builder()
            .add_service(health_server)
            .serve(grpc_addr),
    );
    Ok(())
}

async fn create_consumer() -> Result<RouterConsumer, Box<dyn std::error::Error>> {
    let router_url = std::env::var("ROUTER_REDIS_URL")
        .ok()
        .filter(|router_url| !router_url.is_empty())
        .ok_or("ROUTER_REDIS_URL is not set!")?;
    let instance = std::env::var("ENGINE_INSTANCE")
        .ok()
        .filter(|instance| !instance.is_empty())
        .ok_or("ENGINE_INSTANCE is not set")?;
    let store_url = std::env::var("REDIS_URL")
        .ok()
        .filter(|store_url| !store_url.is_empty())
        .ok_or("REDIS_URL is not set or empty!")?;
    let mut store = RedisStore::connect(&store_url, &instance).await?;
    let engine = match store.load_state().await? {
        // `store.load()` loads from redis and either returns the bytes of the
        // state or `None` if it's first start, and in that case we construct
        // `Engine` with default (fresh start without a saved state)
        None => Engine::default(),
        Some(state_bytes) => Engine::new_from_state(
            EngineState::from_bytes(&state_bytes)?,
            DEFAULT_CACHE_CAPACITY,
        )?,
    };
    let new_events = spawn_event_sender(&engine)?;
    let mut consumer = RouterConsumer::new(engine, Some(store), &router_url, &instance).await?;
    consumer.send_events_to(new_events);
    Ok(consumer)
}

/// starts the task that pushes the events to the consumer. it begins with the history
/// loaded from redis, so a restart sends the kept events again
fn spawn_event_sender(engine: &Engine) -> Result<NewEvents, Box<dyn std::error::Error>> {
    let consumer_url = std::env::var("CONSUMER_URL")
        .ok()
        .filter(|consumer_url| !consumer_url.is_empty())
        .expect("CONSUMER_URL is not set or empty!");
    let (sender, new_events) =
        EventSender::new(&consumer_url, engine.event_history().iter().cloned())?;
    tokio::spawn(sender.run());
    Ok(new_events)
}
