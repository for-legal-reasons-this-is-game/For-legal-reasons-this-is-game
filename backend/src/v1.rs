use axum::{
    extract::{Json, Path, State},
    http::{Response, StatusCode},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use cn_tigerbeetle as tb;
use sqlx::{FromRow, PgPool};
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

#[derive(Serialize, FromRow)]
pub struct User {
    user_id: Uuid,
    user_name: String,
}

#[derive(Deserialize)]
pub struct AccountPayload {
    name: String,
    ledger_type: LedgerType,
    code_type: AccountCodeType,
    user_id: Uuid,
}

#[derive(Serialize, FromRow)]
pub struct Account {
    account_id: Uuid,
    account_name: String,
    account_ledger_type: LedgerType,
    account_code_type: AccountCodeType,
    account_user_id: Uuid,
}

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
#[serde(try_from = "i16", into = "i16")]
enum LedgerType {
    Usd = 0,
    Eur = 1,
    Bitcoin = 2, // extensible
}

impl TryFrom<i16> for LedgerType {
    type Error = String;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        LedgerType::from_i16(value).ok_or_else(|| format!("invalid LedgerType{}", value))
    }
}

impl From<LedgerType> for i16 {
    fn from(value: LedgerType) -> Self {
        value.to_i16().expect("enum variants always fit in i16")
    }
}

impl From<LedgerType> for u32 {
    fn from(value: LedgerType) -> Self {
        value.to_u32().expect("enum variants always fit in u32")
    }
}

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
#[serde(try_from = "i16", into = "i16")]
enum AccountCodeType {
    Cash = 0,
    Crypto = 1,
}

impl TryFrom<i16> for AccountCodeType {
    type Error = String;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        AccountCodeType::from_i16(value).ok_or_else(|| format!("invalid AccountCodeType{}", value))
    }
}

impl From<AccountCodeType> for u16 {
    fn from(value: AccountCodeType) -> Self {
        value.to_u16().expect("enum variants always fit in u16")
    }
}

impl From<AccountCodeType> for i16 {
    fn from(value: AccountCodeType) -> Self {
        value.to_i16().expect("enum variants always fit in i16")
    }
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
    Json(payload): Json<AccountPayload>,
) -> Result<(StatusCode, Json<Account>), StatusCode> {
    let q = sqlx::query_as::<_, Account>(
        "INSERT INTO accounts (account_name, account_ledger_type, account_code_type, account_user_id) VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(payload.name)
    .bind(payload.ledger_type)
    .bind(payload.code_type)
    .bind(payload.user_id)
    .fetch_one(&state.pg_connections)
    .await;

    if let Err(e) = &q {
        match e.to_owned() {
            sqlx::Error::InvalidArgument(_) => return Err(StatusCode::BAD_REQUEST),
            _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    let account = q.unwrap(); // proven by previous statement

    let tb_account = tb::Account {
        id: account.account_id.as_u128(),
        code: account.account_code_type as u16,
        ..Default::default()
    };

    let tb_result = state.tb_client.create_accounts(&[tb_account]).await;

    if let Err(_e) = &tb_result {
        //delete in the relational database
        todo!()
    }
    // iterate over CreateAccountsResult and find if error
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
