use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};

use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

use crate::domain::{Account, AccountCodeType, AccountStatus, Ledger, Position, User};
use cn_tigerbeetle as tb;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub pg_connections: PgPool,
    pub tb_client: Arc<tb::Client>,
    pub ledgers: Arc<RwLock<HashMap<String, Ledger>>>,
}

/// Loads every ledger into a `symbol/ledger` map. Call at startup, and again
/// after any write to the ledgers table to refresh the cache.
pub async fn load_ledgers(pool: &PgPool) -> sqlx::Result<HashMap<String, Ledger>> {
    let rows = sqlx::query_as::<_, Ledger>(
        "SELECT ledger_id, symbol, name, decimals, enabled FROM ledgers",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|l| (l.symbol.clone(), l)).collect())
}

#[derive(Deserialize)]
pub struct UserPayload {
    name: String,
}

#[derive(Deserialize)]
pub struct AccountPayload {
    name: String,
    ledger_symbol: String, // e.g. "USD"
    code_type: AccountCodeType,
}

#[derive(Deserialize)]
pub struct LedgerPayload {
    symbol: String,
    name: String,
    decimals: i16,
}

#[derive(Deserialize)]
pub struct LedgerEnabledPayload {
    enabled: bool,
}

#[derive(Default)]
struct RawBalance {
    debits_posted: i128,
    credits_posted: i128,
    debits_pending: i128,
    credits_pending: i128,
}

// from i128 to Decimal type
fn to_decimal(raw: i128, decimals: i16) -> Result<Decimal, StatusCode> {
    Decimal::try_from_i128_with_scale(raw, decimals as u32)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//create user and write it to postgress. returns the user details, or an error if it fails.
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

//fetches all users; returns the list, or an error.
pub async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<User>>, StatusCode> {
    sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(&state.pg_connections)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

//create account with unique ID and writes it to postgres. It also creates an entry into the outbox,
//which is monitored by the relay. after that, it attempts to create the tigerbeetle account by
//itself and mark the outbox entry as done, while also marking the account as active.
//If that fails, the relay will use the outbox to create the account. returns CREATED when
// both writes succeed, ACCEPTED if only postgres succeeds. In any other case, returns error.
//
// this is probably the most critical endpoint of the API. change it with care and extensive
// testing.
pub async fn create_account(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AccountPayload>,
) -> Result<(StatusCode, Json<Account>), StatusCode> {
    let account_id = Uuid::from_u128(tb::id());

    let ledger_id: i32 = {
        let cache = state
            .ledgers
            .read()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        match cache.get(&payload.ledger_symbol) {
            None => return Err(StatusCode::NOT_FOUND),
            Some(l) if !l.enabled => return Err(StatusCode::BAD_REQUEST),
            Some(l) => l.ledger_id,
        }
    };

    let mut tx = state
        .pg_connections
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut account = sqlx::query_as::<_, Account>(
        "INSERT INTO accounts (account_id, account_name, account_ledger_id, account_code_type, account_user_id) VALUES ($1, $2, $3, $4, $5) RETURNING * ",
    )
    .bind(account_id)
    .bind(payload.name)
    .bind(ledger_id)
    .bind(payload.code_type)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e{
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    sqlx::query(
        "INSERT INTO tb_outbox(aggregate_id, ledger, code, user_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(account.account_id)
    .bind(account.account_ledger_id)
    .bind(account.account_code_type)
    .bind(account.account_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tb_account = tb::Account {
        id: account.account_id.as_u128(),
        ledger: account.account_ledger_id as u32,
        code: account.account_code_type as u16,
        user_data_128: account.account_user_id.as_u128(),
        ..Default::default()
    };

    // timeout is needed for tb calls. the tb client does not implement timeouts
    match timeout(
        Duration::from_millis(500),
        state.tb_client.create_accounts(&[tb_account]),
    )
    .await
    {
        Ok(Ok(results)) => {
            let ok =
                results.is_empty() || matches!(results[0].result, tb::CreateAccountResult::Exists);

            if ok {
                match sqlx::query(
                    "WITH marked AS ( \
                         UPDATE tb_outbox SET processed_at = now() \
                         WHERE aggregate_id = $1 AND processed_at IS NULL \
                         RETURNING aggregate_id \
                     ) \
                     UPDATE accounts SET account_status = 'active' WHERE account_id = $1",
                )
                .bind(account.account_id)
                .execute(&state.pg_connections)
                .await
                {
                    Ok(_) => {
                        account.account_status = AccountStatus::Active;
                        return Ok((StatusCode::CREATED, Json(account)));
                    }
                    Err(e) => println!(
                        "HANDLER: self report succeeded but couldn't write the outbox stamp and the account . Relay will pick it up: {e}"
                    ),
                }
            } else {
                println!(
                    "HANDLER: tigerbeetle rejected self report. Relay will pick it up: {:?}",
                    results[0].result
                );
            }
        }
        Ok(Err(e)) => {
            println!("HANDLER: connection to tigerbeetle failed. Relay will attempt it: {e}");
        }
        Err(_) => {
            println!("HANDLER: Tiger beetle timed out. Relay will pick it up.");
        }
    }

    Ok((StatusCode::ACCEPTED, Json(account)))
}

// returns information about the user
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

//returns with information about the account. use this endpoint to poll the account's status.
pub async fn fetch_account(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
) -> Result<Json<Account>, StatusCode> {
    sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(&state.pg_connections)
        .await
        .map(Json)
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
}

//returns with information about the account's position
pub async fn fetch_account_position(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
) -> Result<Json<Position>, StatusCode> {
    let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(&state.pg_connections)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let (symbol, decimals) = {
        let cache = state
            .ledgers
            .read()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let ledger = cache
            .values()
            .find(|l| l.ledger_id == account.account_ledger_id)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        (ledger.symbol.clone(), ledger.decimals)
    };
    let balance: RawBalance = if account.account_status == AccountStatus::Processing {
        RawBalance::default()
    } else {
        match timeout(
            Duration::from_millis(500),
            state
                .tb_client
                .lookup_accounts(&[account.account_id.as_u128()]),
        )
        .await
        {
            Ok(Ok(result)) => {
                let first = result.first().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
                RawBalance {
                    debits_posted: first.debits_posted as i128,
                    credits_posted: first.credits_posted as i128,
                    debits_pending: first.debits_pending as i128,
                    credits_pending: first.credits_pending as i128,
                }
            }
            Ok(Err(_)) => {
                return Err(StatusCode::BAD_GATEWAY);
            }
            Err(_) => {
                return Err(StatusCode::GATEWAY_TIMEOUT);
            }
        }
    };

    let position = Position {
        account_id: account.account_id,
        symbol,
        decimals,
        debits_posted: to_decimal(balance.debits_posted, decimals)?,
        credits_posted: to_decimal(balance.credits_posted, decimals)?,
        debits_pending: to_decimal(balance.debits_pending, decimals)?,
        credits_pending: to_decimal(balance.credits_pending, decimals)?,
        net_posted: to_decimal(balance.credits_posted - balance.debits_posted, decimals)?,
        account_status: account.account_status,
    };

    Ok(Json(position))
}

//deleting an account is ommited as tb uses transfers to do so, which will come with v2.

//lists all ledgers from cache, it's read-only so no need to look at db again
pub async fn list_ledgers(State(state): State<AppState>) -> Result<Json<Vec<Ledger>>, StatusCode> {
    let cache = state
        .ledgers
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ledgers: Vec<Ledger> = cache.values().cloned().collect();
    Ok(Json(ledgers))
}

// fetches a ledger by its symbol from the cache
pub async fn fetch_ledger(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Ledger>, StatusCode> {
    let cache = state
        .ledgers
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ledger = cache.get(&symbol).cloned();

    if ledger.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(ledger.unwrap()))
}

// creates a new ledger, also updating the cache. It also enforces the db requirements
pub async fn create_ledger(
    State(state): State<AppState>,
    Json(payload): Json<LedgerPayload>,
) -> Result<(StatusCode, Json<Ledger>), StatusCode> {
    let new = sqlx::query_as::<_, Ledger>(
        "INSERT INTO ledgers (symbol, name, decimals) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(payload.symbol)
    .bind(payload.name)
    .bind(payload.decimals)
    .fetch_one(&state.pg_connections)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => StatusCode::CONFLICT,
        sqlx::Error::Database(db) if db.is_check_violation() => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    {
        let mut cache = state
            .ledgers
            .write()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        cache.insert(new.symbol.clone(), new.clone());
    }

    Ok((StatusCode::CREATED, Json(new)))
}

//enable/disable the ledger. payload must contain information on if it's on or off. The cache is
//also updated
pub async fn set_ledger_enabled(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Json(payload): Json<LedgerEnabledPayload>,
) -> Result<Json<Ledger>, StatusCode> {
    let updated = sqlx::query_as::<_, Ledger>(
        "UPDATE ledgers SET enabled = $1 WHERE symbol = $2 RETURNING *",
    )
    .bind(payload.enabled)
    .bind(&symbol)
    .fetch_one(&state.pg_connections)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    {
        let mut cache = state
            .ledgers
            .write()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        cache.insert(updated.symbol.clone(), updated.clone());
    }

    Ok(Json(updated))
}
