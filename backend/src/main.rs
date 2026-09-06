use axum::{
    Router,
    routing::{delete, get, post},
};

use backend::{
    relay,
    v1::{self, AppState},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::{Arc, RwLock};

use cn_tigerbeetle as tb;

use std::env;

// how to stricture api /api/{version: String}/*
#[tokio::main]
async fn main() {
    // We need to implement secrets, and hold these values there for tiger beetle.
    let tb_client = Arc::new(
        tb::Client::new(
            112369905620179595468860499864441504024,
            "172.28.0.10:3000,172.28.0.11:3000,172.28.0.12:3000",
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

    let state = AppState {
        pg_connections,
        tb_client,
        ledgers: Arc::new(RwLock::new(ledgers)),
    };

    // the task loops forever, but we will need to join it on ctrl c
    tokio::task::spawn(relay::relay_loop(state.clone()));
    let v1 = Router::new()
        .route("/users", post(v1::create_user))
        .route("/users", get(v1::list_users))
        .route("/users/{user_id}/accounts", post(v1::create_account))
        .route("/users/{user_id}", get(v1::fetch_user))
        .route("/accounts/{account_id}", get(v1::fetch_account))
        .route("/accounts/{account_id}", delete(v1::delete_account))
        .route("/accounts/{account_id}/positions", get(v1::fetch_positions))
        .route(
            "/accounts/{account_id}/positions/{asset_id}",
            get(v1::fetch_account_position_asset),
        )
        .route("/ledgers", get(v1::list_ledgers))
        .route("/ledgers", post(v1::create_ledger))
        .route("/ledgers", delete(v1::delete_ledger))
        .route("/ledgers/{symbol}", get(v1::list_ledgers))
        .with_state(state);

    let router = Router::<()>::new().nest("/api/v1", v1);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("Server running on 0.0.0.0:8000");
    axum::serve(listener, router).await.unwrap();
}
