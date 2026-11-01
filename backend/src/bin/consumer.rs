use backend::service::EngineClient;
use backon::{BackoffBuilder, ExponentialBuilder};
use cn_tigerbeetle as tb;
use proto::SubscribeEventsRequest;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

//builds the backoff for retries
fn new_backoff() -> impl Iterator<Item = Duration> {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_millis(500))
        .with_max_delay(Duration::from_secs(30))
        .with_jitter()
        .without_max_times()
        .build()
}

const HEALTHY_AFTER: Duration = Duration::from_secs(30);

async fn run_consumer(client: &EngineClient, tb: &tb::Client, pg: &Pool<Postgres>) -> ! {
    let mut delays = new_backoff();

    loop {
        let connected_at = Instant::now();

        match client.subscribe_events(SubscribeEventsRequest {}).await {
            Err(e) => eprintln!("Consumer: subscribe failed: {e}"),
            Ok(mut stream) => loop {
                match stream.message().await {
                    Ok(Some(event)) => match event {},
                    Ok(None) => {
                        eprintln!("Consumer: engine closed the stream");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Consumer: stream error: {e}");
                        break;
                    }
                }
            },
        }
        // if its not going  down constantly, i reset backoff so it doesnt grow
        if connected_at.elapsed() >= HEALTHY_AFTER {
            delays = new_backoff(); // it was fine for a while so its started with the smallest delay
        }

        let wait = delays.next().unwrap_or(Duration::from_secs(30));
        eprintln!("Consumer: reconnecting in {wait:?}");
        sleep(wait).await;
    }
}
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
        env::var("ENGINE_URL").unwrap_or_else(|_| "http://tradingengine:50051".to_string()); // this would be the url of the router
    let engine_client = EngineClient::new(&engine_url).expect("Invalid ENGINE_URL");
}
