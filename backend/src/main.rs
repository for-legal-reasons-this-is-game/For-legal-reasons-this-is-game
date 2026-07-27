use axum::{
    Router,
    routing::{delete, get, post},
};
pub mod hmac_utils;
pub mod v1;

// how to stricture api /api/{version: String}/*
#[tokio::main]
async fn main() {
    let v1 = Router::<()>::new()
        .route("/users", post(v1::create_user))
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
        .route("/instruments/{symbol}", get(v1::list_instruments));

    let _router = Router::<()>::new().nest("/api/v1", v1);
}
