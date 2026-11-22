use proto::{trading, trading_engine_server::TradingEngineServer};
use std::env;
use tonic::transport::Server;

mod events_handeling;
mod redis_input;

use events_handeling::EventHandler;
use redis_input::Redis;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL is not set");
    let client = redis::Client::open(redis_url)?;
    let conn = client.get_connection()?;
    let handler = EventHandler::new(Redis::new(conn));

    // health reporter lets the backend wait until the router is ready
    let (health_reporter, health_server) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<TradingEngineServer<EventHandler>>()
        .await;

    // reflection, so grpcurl/grpcui can list the services
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(trading::v1::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    let addr = env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()?;

    Server::builder()
        .add_service(reflection)
        .add_service(health_server)
        .add_service(TradingEngineServer::new(handler))
        .serve(addr)
        .await?;

    Ok(())
}
