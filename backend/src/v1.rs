use axum::{
    extract::{Json, Path, State},
    http::{Response, StatusCode},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct UserPayload {
    name: String,
}

#[derive(Serialize, FromRow)]
pub struct User {
    id: i32,
    name: String,
}
#[derive(Clone)]
pub struct AppState {
    pub pg_connections: PgPool,
}
//create user write it to database and respond if it sucseeds and create userid
pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<UserPayload>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    sqlx::query_as::<_, User>("INSERT INTO users name VALUES $1 RETURNING * ")
        .bind(payload.name)
        .fetch_one(&state.pg_connections)
        .await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//create account with unique ID and write it to database
pub async fn create_account(
    State(_state): State<AppState>,
    Json(_payload): Json<Value>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the user
pub async fn fetch_user(Path(_user_id): Path<Uuid>) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the account
pub async fn fetch_account(Path(_account_id): Path<Uuid>) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the accoiunts position
pub async fn fetch_positions(
    Path(_account_id): Path<Uuid>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the accoiunts position
pub async fn fetch_account_position_asset(
    Path(_account_id): Path<Uuid>,
    Path(_asset_id): Path<Uuid>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

//returns with information about the accoiunts position
pub async fn delete_account(
    Path(_account_id): Path<Uuid>,
    Path(_asset_id): Path<Uuid>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn list_instruments() -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn fetch_instrument(Path(_symbol): Path<String>) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn create_instrument(
    Path(_symbol): Path<String>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn delete_instrument(
    Path(_symbol): Path<String>,
) -> Result<Response<String>, StatusCode> {
    todo!();
}
