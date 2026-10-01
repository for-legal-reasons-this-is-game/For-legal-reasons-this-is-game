use proto::{trading, trading_engine_server::TradingEngineServer};
use tonic::transport::Server;
mod service;
use tonic_health;
use tradingengine::service::EngineService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // health reporter is for setting status of server (at the moment only docker checks it, so dont change it), health server is a service for advertising
    // health so we can actually start the backend after this is ready.

    let (health_reporter, health_server) = tonic_health::server::health_reporter();

    //grpc server
    let addr = "0.0.0.0:50051".parse()?;

    let tradingengine = service::EngineService::default();

    // reflections, so that we can see the services available on clients or via grpccurl/grpcui
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(trading::v1::FILE_DESCRIPTOR_SET)
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
