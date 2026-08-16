use axum::{
    Router,
    routing::{delete, get, post},
};

use sqlx::postgres::PgPoolOptions;
use std::env;

use crate::v1::AppState;
pub mod hmac_utils;
pub mod v1;

// how to stricture api /api/{version: String}/*
#[tokio::main]
async fn main() {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pg_connections = PgPoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB");
    let state = AppState { pg_connections };
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
        .route("/instruments", get(v1::list_instruments))
        .route("/instruments", post(v1::create_instrument))
        .route("/instruments", delete(v1::delete_instrument))
        .route("/instruments/{symbol}", get(v1::list_instruments))
        .with_state(state);

    let router = Router::<()>::new().nest("/api/v1", v1);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("Server running on 0.0.0.0:8000");
    axum::serve(listener, router).await.unwrap();
}
