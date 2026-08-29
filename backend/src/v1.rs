use axum::{
    extract::{Json, Path, State},
    http::{Response, StatusCode},
};

use serde::Deserialize;
use uuid::Uuid;

use crate::domain::{Account, AccountCodeType, LedgerType, User};
use cn_tigerbeetle as tb;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pg_connections: PgPool,
    pub tb_client: Arc<tb::Client>,
}

#[derive(Deserialize)]
pub struct UserPayload {
    name: String,
}

#[derive(Deserialize)]
pub struct AccountPayload {
    name: String,
    ledger_type: LedgerType,
    code_type: AccountCodeType,
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
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AccountPayload>,
) -> Result<(StatusCode, Json<Account>), StatusCode> {
    let account_id = Uuid::from_u128(tb::id());

    let mut tx = state
        .pg_connections
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let account = sqlx::query_as::<_, Account>(
        "INSERT INTO accounts (account_id, account_name, account_ledger_type, account_code_type, account_user_id) VALUES ($1, $2, $3, $4, $5) RETURNING * ",
    )
    .bind(account_id)
    .bind(payload.name)
    .bind(payload.ledger_type)
    .bind(payload.code_type)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e{
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    sqlx::query(
        "INSERT INTO tb_outbox(agrregate_id, ledger, code, user_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(account.account_id)
    .bind(account.account_ledger_type)
    .bind(account.account_code_type)
    .bind(account.account_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(account)))
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

pub async fn list_ledgers() -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn fetch_ledger(Path(_symbol): Path<String>) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn create_ledger(Path(_symbol): Path<String>) -> Result<Response<String>, StatusCode> {
    todo!();
}

pub async fn delete_ledger(Path(_symbol): Path<String>) -> Result<Response<String>, StatusCode> {
    todo!();
}
