use axum::{
    Router,
    routing::{get, patch, post},
};

use backend::{
    bootstrap, relay,
    service::EngineClient,
    v1::{self, AppState},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, RwLock};

use cn_tigerbeetle as tb;

use std::env;

// how to stricture api /api/{version: String}/*
#[tokio::main]
async fn main() {
    // TB_CLUSTER_ID / TB_IP_ADRESSES come from the process environment
    // (e.g. a K8s Secret via envFrom).
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

    sqlx::migrate!()
        .run(&pg_connections)
        .await
        .expect("Migration failed");

    let ledgers = v1::load_ledgers(&pg_connections)
        .await
        .expect("Failed to load ledgers");

    let engine_url =
        env::var("ENGINE_URL").unwrap_or_else(|_| "http://tradingengine:50051".to_string());
    let engine_client = EngineClient::new(&engine_url)
        .await
        .expect("Failed trading engine connection");

    let state = AppState {
        pg_connections,
        tb_client,
        ledgers: Arc::new(RwLock::new(ledgers)),
        engine_client,
    };

    bootstrap::run(&state)
        .await
        .expect("Failed to bootstrap system accounts");

    // the task loops forever, but we will need to join it on ctrl c
    tokio::task::spawn(relay::relay_loop(state.clone()));

    let v1 = Router::new()
        .route("/users", post(v1::create_user))
        .route("/users", get(v1::list_users))
        .route("/users/{user_id}/accounts", post(v1::create_account))
        .route("/users/{user_id}", get(v1::fetch_user))
        .route("/accounts/{account_id}", get(v1::fetch_account))
        .route(
            "/accounts/{account_id}/position",
            get(v1::fetch_account_position),
        )
        .route("/ledgers", get(v1::list_ledgers))
        .route("/ledgers", post(v1::create_ledger))
        .route("/ledgers/{symbol}", patch(v1::set_ledger_enabled))
        .route("/ledgers/{symbol}", get(v1::fetch_ledger))
        .with_state(state);

    let router = Router::<()>::new().nest("/api/v1", v1);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("Server running on 0.0.0.0:8000");
    axum::serve(listener, router).await.unwrap();
}
