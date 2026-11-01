use backend::service::EngineClient;
use cn_tigerbeetle as tb;
use proto::SubscribeEventsRequest;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;
#[tokio::main]
async fn main() {
    let tb_client = Arc::new(
        tb::Client::new(
            env::var("TB_CLUSTER_ID")
                .expect("TB_CLUSTER_ID not set")
                .parse()
                .expect("Failed to parse TB_CLUSTER_ID"),
            env::var("TB_IP_ADRESSES")
                .expect("TB_ADRESSES not set")
                .as_str(),
        )
        .expect("Tiger Beetle client couldn't be started"),
    );
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pg_connections = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    let engine_url =
        env::var("ENGINE_URL").unwrap_or_else(|_| "http://tradingengine:50051".to_string());
    let engine_client = EngineClient::new(&engine_url).expect("Invalid ENGINE_URL");
    let stream = engine_client
        .subscribe_events(SubscribeEventsRequest {})
        .await
        .expect("Can't start stream");
}
