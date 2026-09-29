use proto::{trading, trading_engine_server::TradingEngineServer};
use tonic::transport::Server;
mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //grpc server
    let addr = "0.0.0.0:50051".parse()?;

    let tradingengine = service::EngineService::default();

    // reflections, so that we can see the services available on clients or via grpccurl/grpcui
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(trading::v1::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    Server::builder()
        .add_service(reflection)
        .add_service(TradingEngineServer::new(tradingengine))
        .serve(addr)
        .await?;
    Ok(())
}
