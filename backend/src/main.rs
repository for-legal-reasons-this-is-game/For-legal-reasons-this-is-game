use axum::{
    Router,
    routing::{delete, get, post},
};

use backend::v1::{self, AppState};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

use cn_tigerbeetle as tb;

use std::env;

// how to stricture api /api/{version: String}/*
#[tokio::main]
async fn main() {
    // TigerBeetle connection details come from the environment (Infisical at
    // runtime); see entrypoint.sh / secret_managment.
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

    let state = AppState {
        pg_connections,
        tb_client,
    };

    sqlx::migrate!()
        .run(&state.pg_connections)
        .await
        .expect("Migration failed");

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
