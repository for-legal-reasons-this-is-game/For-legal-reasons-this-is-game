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
    birthday: String,
}

#[derive(Serialize, FromRow)]
pub struct User {
    user_id: Uuid,
    user_name: String,
    user_birthday: String,
}

// #[derive(Serialize, FromRow)]
// pub struct Account {
//     id: i32,
//     name: String,
//     user_id: i32,
// }

#[derive(Clone)]
pub struct AppState {
    pub pg_connections: PgPool,
}
//create user write it to database and respond if it sucseeds and create userid
pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<UserPayload>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    sqlx::query_as::<_, User>("INSERT INTO users (user_name) VALUES ($1) RETURNING * ")
        .bind(payload.name)
        .fetch_one(&state.pg_connections)
        .await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//look at users
pub async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<User>>, StatusCode> {
    sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&state.pg_connections)
        .await
        .map(Json)
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
pub async fn fetch_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, StatusCode> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&state.pg_connections)
        .await
        .map(Json)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            sqlx::Error::InvalidArgument(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
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
