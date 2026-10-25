use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};

use num_traits::ToPrimitive;
use proto::{CancelOrdersRequest, RejectReason};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::time::{Duration, timeout};
// #[derive(TS)] + #[ts(export)] generate TypeScript types for the frontend
// (`cargo test export_bindings` writes them to bindings/)
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::{
    Account, AccountCodeType, AccountStatus, Ledger, Market, MarketRow, Order, OrderPage, OrderRow,
    OrderStatus, Position, Trade, TradeRow, Transfer, TransferDirection, TransferPage, User,
    to_decimal,
};
use crate::service::EngineClient;
use crate::{bootstrap, domain::TransferCodeType};
use cn_tigerbeetle::{self as tb, CreateTransferResult, TransferFlags};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pg_connections: PgPool,
    pub tb_client: Arc<tb::Client>,
    pub engine_client: EngineClient,
}

pub async fn load_ledgers(pool: &PgPool) -> sqlx::Result<Vec<Ledger>> {
    sqlx::query_as::<_, Ledger>("SELECT ledger_id, symbol, name, decimals, enabled FROM ledgers")
        .fetch_all(pool)
        .await
}

pub async fn fetch_ledger_by_symbol(pool: &PgPool, symbol: &str) -> sqlx::Result<Option<Ledger>> {
    sqlx::query_as::<_, Ledger>(
        "SELECT ledger_id, symbol, name, decimals, enabled FROM ledgers WHERE symbol = $1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_ledger_by_id(pool: &PgPool, ledger_id: i32) -> sqlx::Result<Option<Ledger>> {
    sqlx::query_as::<_, Ledger>(
        "SELECT ledger_id, symbol, name, decimals, enabled FROM ledgers WHERE ledger_id = $1",
    )
    .bind(ledger_id)
    .fetch_optional(pool)
    .await
}

#[derive(Deserialize)]
pub struct UserPayload {
    name: String,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct AccountPayload {
    name: String,
    ledger_symbol: String, // e.g. "USD"
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct LedgerPayload {
    symbol: String,
    name: String,
    decimals: i16,
}

#[derive(Deserialize)]
pub struct LedgerEnabledPayload {
    enabled: bool,
}

#[derive(Deserialize)]
pub struct DepositPayload {
    idempotency_key: Uuid, // frontend-generated IN UUIDv7!
    amount: Decimal, // in practice, this is sent as a string, important because of no rounding should happen
}

#[derive(Deserialize)]
pub struct TransferQuery {
    limit: Option<u32>, // amount of transfers to return requested by the frontend (if none there's default of 50)
    cursor: Option<String>, // where the page starts, nothing == first ones, Some == the one with the timestamp + the other limit - 1 amount
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatusFilter {
    Open,
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Expired,
    PendingTrigger,
}

#[derive(Deserialize)]
pub struct OrderQuery {
    market: Option<String>,            // symbol of the market, none == all of them
    status: Option<OrderStatusFilter>, // status of the order, none == all of them
    limit: Option<u32>,                // same as in transferquery: default 50, max 100
    cursor: Option<String>, // the next_cursor of the previous page, nothing == first page
}

#[derive(Deserialize)]
pub struct MarketOrderQuery {
    status: Option<OrderStatusFilter>, // status of the order, none == all of them
    limit: Option<u32>,                // same as in transferquery: default 50, max 100
    cursor: Option<String>, // the next_cursor of the previous page, nothing == first page
}

#[derive(Default)]
struct RawBalance {
    debits_posted: i128,
    credits_posted: i128,
    debits_pending: i128,
    credits_pending: i128,
}

// if a idempotency key from the front end was already used with a different deposit/transfer(so if the other fields conflict) TB will reject it
fn is_deposit_conflict(result: &CreateTransferResult) -> bool {
    matches!(
        result,
        CreateTransferResult::ExistsWithDifferentFlags
            | CreateTransferResult::ExistsWithDifferentPendingId
            | CreateTransferResult::ExistsWithDifferentTimeout
            | CreateTransferResult::ExistsWithDifferentDebitAccountId
            | CreateTransferResult::ExistsWithDifferentCreditAccountId
            | CreateTransferResult::ExistsWithDifferentAmount
            | CreateTransferResult::ExistsWithDifferentUserData128
            | CreateTransferResult::ExistsWithDifferentUserData64
            | CreateTransferResult::ExistsWithDifferentUserData32
            | CreateTransferResult::ExistsWithDifferentLedger
            | CreateTransferResult::ExistsWithDifferentCode
            | CreateTransferResult::IdAlreadyFailed
            | CreateTransferResult::CreditAccountAlreadyClosed
    )
}

// a type name for the transfer response for each transfer so the frontend displays it
fn transfer_kind(t: &tb::Transfer) -> &'static str {
    match TransferCodeType::try_from(t.code as i16) {
        Ok(TransferCodeType::Trade) => "trade",
        Ok(TransferCodeType::FeeCollection) => "fee",
        Ok(TransferCodeType::ProvisioningFromSource) => "deposit",
        Ok(TransferCodeType::Hold) => "hold",
        Ok(TransferCodeType::CloseAccount) | Err(_) => "unknown",
    }
}

//                                                           API ENDPOINTS
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

    let ledger_id: i32 = match fetch_ledger_by_symbol(&state.pg_connections, &payload.ledger_symbol)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        None => return Err(StatusCode::NOT_FOUND),
        Some(l) if !l.enabled => return Err(StatusCode::BAD_REQUEST),
        Some(l) => l.ledger_id,
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
    .bind(AccountCodeType::User)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e{
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => StatusCode::NOT_FOUND,
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
        flags: account.account_code_type.tb_flags(),
        ..Default::default()
    };

    // timeout is needed for tb calls. the tb client does not implement timeouts
    match timeout(
        Duration::from_secs(2),
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

// closes an account and returns no content if it was able to do it/was already closed. It doesnt
// need timeouts with tb because retrying is 100% safe.

pub async fn close_account(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let account = state
        .tb_client
        .lookup_accounts(&[account_id.as_u128()])
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .into_iter()
        .next()
        .ok_or(StatusCode::NOT_FOUND)?;
    if account.code != AccountCodeType::User as u16 {
        return Err(StatusCode::FORBIDDEN);
    }

    // this sends a cancelordersrequest for all the account's orders in all markets.
    // a problem to solve later is that some orders might be in flight, we probably would want to
    // credit the buyer/seller and forfeit the money exchanged or a different solution, we'll
    // discuss

    let symbols: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT m.symbol \
         FROM orders o \
         JOIN markets m ON m.market_id = o.market_id \
         WHERE (o.base_account_id = $1 OR o.quote_account_id = $1) \
           AND o.submission <> 'rejected' \
           AND (o.status IS NULL OR o.status IN (1, 2, 6))",
    )
    .bind(account_id)
    .fetch_all(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for symbol in symbols {
        let resp = state
            .engine_client
            .cancel_orders(CancelOrdersRequest {
                command_id: Some(tb::id().into()),
                market_symbol: symbol,
                account_holder: Some(account.user_data_128.into()),
            })
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;

        if resp.reject_reason() != RejectReason::Unspecified {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // searches for the source account for the transfer, shouldn't fail as it was created at the
    // ledger creation
    let source_acc = state
        .tb_client
        .query_accounts(tb::QueryFilter {
            ledger: account.ledger,
            code: AccountCodeType::Source as u16,
            limit: 1,
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .into_iter()
        .next()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    match state
        .tb_client
        .create_transfers(&[tb::Transfer {
            id: tb::id(),
            credit_account_id: source_acc.id,
            debit_account_id: account.id,
            amount: 0,
            ledger: account.ledger,
            code: TransferCodeType::CloseAccount as u16,
            flags: TransferFlags::Pending | tb::TransferFlags::ClosingDebit,
            ..Default::default()
        }])
        .await
    {
        Err(_) => Err(StatusCode::BAD_GATEWAY),
        Ok(v) => match v.first() {
            Some(r) => {
                if r.result == CreateTransferResult::CreditAccountAlreadyClosed {
                    println!("close_account: The source account is closed!");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                } else {
                    Ok(StatusCode::NO_CONTENT)
                }
            }
            None => Ok(StatusCode::NO_CONTENT),
        },
    }
}

// reopens a closed account.Returns NO CONTENT for already open accounts and when it succeeds at reopening an account
pub async fn reopen_account(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let account = state
        .tb_client
        .lookup_accounts(&[account_id.as_u128()])
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .into_iter()
        .next()
        .ok_or(StatusCode::NOT_FOUND)?;

    if account.code != AccountCodeType::User as u16 {
        return Err(StatusCode::FORBIDDEN);
    }

    if !account.flags.contains(tb::AccountFlags::Closed) {
        return Ok(StatusCode::NO_CONTENT);
    }

    let close_transfer = state
        .tb_client
        .get_account_transfers(tb::AccountFilter {
            account_id: account.id,
            code: TransferCodeType::CloseAccount as u16,
            limit: 1,
            flags: tb::AccountFilterFlags::Debits | tb::AccountFilterFlags::Reversed,
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .into_iter()
        .next()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    match state
        .tb_client
        .create_transfers(&[tb::Transfer {
            id: tb::id(),
            pending_id: close_transfer.id,
            flags: TransferFlags::VoidPendingTransfer,
            ..Default::default()
        }])
        .await
    {
        Err(_) => Err(StatusCode::BAD_GATEWAY),
        Ok(v) => match v.first() {
            Some(r) if r.result == CreateTransferResult::PendingTransferAlreadyVoided => {
                Ok(StatusCode::NO_CONTENT)
            }
            Some(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            None => Ok(StatusCode::NO_CONTENT),
        },
    }
}

// credits an account from the ledger's source account. The idempotency key comes from frontend as UUIDv7 and
// its there so we dont duplicate deposits on retries, returns CREATED with the transfer when we succeed at making the deposit,
// OK with the same transfer if its a retry of an already successful deposit and  CONFLICT if there is a conflict (for example that its same key but different amount)
pub async fn create_deposit(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
    Json(payload): Json<DepositPayload>,
) -> Result<(StatusCode, Json<Transfer>), StatusCode> {
    if payload.idempotency_key.is_nil() || payload.idempotency_key.as_u128() == u128::MAX {
        return Err(StatusCode::BAD_REQUEST);
    }

    let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(&state.pg_connections)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    // we can only fund user accounts that are active
    if account.account_code_type != AccountCodeType::User {
        return Err(StatusCode::FORBIDDEN);
    }
    if account.account_status != AccountStatus::Active {
        return Err(StatusCode::CONFLICT);
    }

    let ledger = fetch_ledger_by_id(&state.pg_connections, account.account_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if !ledger.enabled {
        return Err(StatusCode::BAD_REQUEST);
    }
    let decimals = ledger.decimals;

    // Decimal types get converted into integers with no decimals for TB.
    if payload.amount <= Decimal::ZERO {
        return Err(StatusCode::BAD_REQUEST);
    }
    let smallest_units = payload
        .amount
        .checked_mul(Decimal::from(10u64.pow(decimals as u32)))
        .ok_or(StatusCode::BAD_REQUEST)?;
    if !smallest_units.fract().is_zero() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let smallest_units = smallest_units.to_u128().ok_or(StatusCode::BAD_REQUEST)?;

    let source_id: Uuid = sqlx::query_scalar(
        "SELECT account_id FROM accounts WHERE account_ledger_id = $1 AND account_code_type = $2",
    )
    .bind(account.account_ledger_id)
    .bind(AccountCodeType::Source)
    .fetch_one(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let results = match timeout(
        Duration::from_secs(2),
        state.tb_client.create_transfers(&[tb::Transfer {
            id: payload.idempotency_key.as_u128(),
            debit_account_id: source_id.as_u128(),
            credit_account_id: account.account_id.as_u128(),
            amount: smallest_units,
            ledger: account.account_ledger_id as u32,
            code: TransferCodeType::ProvisioningFromSource as u16,
            ..Default::default()
        }]),
    )
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => return Err(StatusCode::BAD_GATEWAY),
        Err(_) => return Err(StatusCode::GATEWAY_TIMEOUT),
    };

    let status = match results.first().map(|r| &r.result) {
        None | Some(CreateTransferResult::Ok) => StatusCode::CREATED,
        Some(CreateTransferResult::Exists) => StatusCode::OK,
        Some(r) if is_deposit_conflict(r) => return Err(StatusCode::CONFLICT),
        Some(
            CreateTransferResult::OverflowsCredits | CreateTransferResult::OverflowsCreditsPosted,
        ) => return Err(StatusCode::BAD_REQUEST),
        // this happens when the source account hasn't been created by the relay, so retrying should work
        Some(CreateTransferResult::DebitAccountNotFound) => {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        Some(other) => {
            println!("deposit: unexpected tigerbeetle result {other:?}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // tb doesn't give back the stored transfer (we need its timestamp), so we look it up. if this
    // fails the deposit is already done; a retry with the same idempotency key returns it as OK.
    let stored = match timeout(
        Duration::from_secs(2),
        state
            .tb_client
            .lookup_transfers(&[payload.idempotency_key.as_u128()]),
    )
    .await
    {
        Ok(Ok(r)) => r
            .into_iter()
            .next()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        Ok(Err(_)) => return Err(StatusCode::BAD_GATEWAY),
        Err(_) => return Err(StatusCode::GATEWAY_TIMEOUT),
    };

    Ok((
        status,
        Json(Transfer {
            transfer_id: Uuid::from_u128(stored.id),
            direction: TransferDirection::In,
            kind: transfer_kind(&stored),
            amount: to_decimal(
                i128::try_from(stored.amount).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                decimals,
            )?,
            timestamp: stored.timestamp.to_string(),
        }),
    ))
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

    let ledger = fetch_ledger_by_id(&state.pg_connections, account.account_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let (symbol, decimals) = (ledger.symbol, ledger.decimals);
    let balance: RawBalance = if account.account_status == AccountStatus::Processing {
        RawBalance::default()
    } else {
        match timeout(
            Duration::from_secs(2),
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

// gives am account's transfers, newest first, limited to a page per call (the limit part of the TransferQuery, which should be max 100 and is 50 if you dont set it)
// when you call the endpoint the first time you dont set the cursor so you get the first page,
// the response of that call will give you next_cursor which is a timestamp of the first element of next page,
// so you can send it back with youre query again and you get the next page. if next_cursor is none its the last page
pub async fn list_account_transfers(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
    Query(query): Query<TransferQuery>,
) -> Result<Json<TransferPage>, StatusCode> {
    let limit = query.limit.unwrap_or(50).min(100);
    if limit == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let timestamp_max = match query.cursor {
        None => 0,
        Some(c) => c.parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST)?,
    };

    let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(&state.pg_connections)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    let decimals = fetch_ledger_by_id(&state.pg_connections, account.account_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .decimals;

    // i ask tb for 1 more than the page limit because i need timestamp of limit+1 to return the cursor
    let mut transfers = match timeout(
        Duration::from_secs(2),
        state.tb_client.get_account_transfers(tb::AccountFilter {
            account_id: account.account_id.as_u128(),
            timestamp_max,
            limit: limit + 1,
            flags: tb::AccountFilterFlags::Debits
                | tb::AccountFilterFlags::Credits
                | tb::AccountFilterFlags::Reversed,
            ..Default::default()
        }),
    )
    .await
    {
        Ok(Ok(t)) => t,
        Ok(Err(_)) => return Err(StatusCode::BAD_GATEWAY),
        Err(_) => return Err(StatusCode::GATEWAY_TIMEOUT),
    };

    let next_cursor = transfers
        .get(limit as usize)
        .map(|t| t.timestamp.to_string());
    transfers.truncate(limit as usize);

    let items = transfers
        .iter()
        .filter(|t| t.code != TransferCodeType::CloseAccount as u16)
        .map(|t| {
            Ok(Transfer {
                transfer_id: Uuid::from_u128(t.id),
                direction: if t.debit_account_id == account.account_id.as_u128() {
                    TransferDirection::Out
                } else {
                    TransferDirection::In
                },
                kind: transfer_kind(t),
                amount: to_decimal(
                    i128::try_from(t.amount).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                    decimals,
                )?,
                timestamp: t.timestamp.to_string(),
            })
        })
        .collect::<Result<Vec<_>, StatusCode>>()?;

    Ok(Json(TransferPage { items, next_cursor }))
}

//lists all ledgers
pub async fn list_ledgers(State(state): State<AppState>) -> Result<Json<Vec<Ledger>>, StatusCode> {
    load_ledgers(&state.pg_connections)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// fetches a ledger by its symbol
pub async fn fetch_ledger(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Ledger>, StatusCode> {
    fetch_ledger_by_symbol(&state.pg_connections, &symbol)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// creates a new ledger. It also enforces the db requirements.
// Additionally, creates the fee account (for collecting fees) and the source account (for giving
// money to users). if those accounts cant be created, the relay picks them up
pub async fn create_ledger(
    State(state): State<AppState>,
    Json(payload): Json<LedgerPayload>,
) -> Result<(StatusCode, Json<Ledger>), StatusCode> {
    let mut tx = state
        .pg_connections
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let new = sqlx::query_as::<_, Ledger>(
        "INSERT INTO ledgers (symbol, name, decimals) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(payload.symbol)
    .bind(payload.name)
    .bind(payload.decimals)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => StatusCode::CONFLICT,
        sqlx::Error::Database(db) if db.is_check_violation() => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    bootstrap::insert_system_accounts(&mut tx, new.ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match bootstrap::activate_system_accounts(&state, new.ledger_id).await {
        Ok(true) => Ok((StatusCode::CREATED, Json(new))),
        Ok(false) => Ok((StatusCode::ACCEPTED, Json(new))), // relay handles
        Err(_) => Ok((StatusCode::ACCEPTED, Json(new))),    // relay handles
    }
}

//enable/disable the ledger. payload must contain information on if it's on or off
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

    Ok(Json(updated))
}

// lists all markets
pub async fn list_markets(State(state): State<AppState>) -> Result<Json<Vec<Market>>, StatusCode> {
    let market_rows = sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets ORDER BY market_id",
    )
    .fetch_all(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ledgers = load_ledgers(&state.pg_connections)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let markets = market_rows
        .into_iter()
        .map(|m| {
            let base = ledgers
                .iter()
                .find(|l| l.ledger_id == m.base_ledger_id)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
                .clone();
            let quote = ledgers
                .iter()
                .find(|l| l.ledger_id == m.quote_ledger_id)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
                .clone();
            Market::from_row(m, base, quote)
        })
        .collect::<Result<Vec<_>, StatusCode>>()?;
    Ok(Json(markets))
}

// fetches a specific market by its symbol
pub async fn fetch_market(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Market>, StatusCode> {
    let m = sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                   min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                   maker_fee_bps, taker_fee_bps \
            FROM markets WHERE symbol = $1",
    )
    .bind(symbol)
    .fetch_optional(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let base = fetch_ledger_by_id(&state.pg_connections, m.base_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let quote = fetch_ledger_by_id(&state.pg_connections, m.quote_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(Market::from_row(m, base, quote)?))
}

// returns an order by id. Submission comes from the side of the backend, and status from the engine
pub async fn fetch_order(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Order>, StatusCode> {
    let o = sqlx::query_as::<_, OrderRow>(
        r#"SELECT order_id, idempotency_key, user_id, market_id, base_account_id,
                  quote_account_id, side, order_type, time_in_force, base_quantity,
                  quote_quantity, limit_price, trigger_price, trigger_direction,
                  protection_price,
                  to_char(expire_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS expire_time,
                  maker_fee_bps, taker_fee_bps, reserved_amount, submission, reject_reason,
                  reject_detail, status, cancel_reason, filled_base_quantity,
                  filled_quote_quantity,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS created_at,
                  to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS updated_at
           FROM orders WHERE order_id = $1"#,
    )
    .bind(order_id)
    .fetch_optional(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let m = sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets WHERE market_id = $1",
    )
    .bind(o.market_id)
    .fetch_one(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base = fetch_ledger_by_id(&state.pg_connections, m.base_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let quote = fetch_ledger_by_id(&state.pg_connections, m.quote_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(Order::from_row(o, &m, &base, &quote)?))
}

// gives a user's orders, newest first
// market (a symbol) and status can be used to filter, status can also be "open" which is every order that can still trade
// returns the page + next cursor
pub async fn list_user_orders(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<OrderQuery>,
) -> Result<Json<OrderPage>, StatusCode> {
    let limit = query.limit.unwrap_or(50).min(100);
    if limit == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (cursor_time, cursor_id) = match query.cursor {
        None => (None, None),
        Some(c) => {
            let (time, id) = c.split_once('_').ok_or(StatusCode::BAD_REQUEST)?;
            let id = id.parse::<Uuid>().map_err(|_| StatusCode::BAD_REQUEST)?;
            (Some(time.to_string()), Some(id))
        }
    };

    let (status, open_only) = match query.status {
        None => (None, false),
        Some(OrderStatusFilter::Open) => (None, true),
        Some(OrderStatusFilter::New) => (Some(OrderStatus::New as i16), false),
        Some(OrderStatusFilter::PartiallyFilled) => {
            (Some(OrderStatus::PartiallyFilled as i16), false)
        }
        Some(OrderStatusFilter::Filled) => (Some(OrderStatus::Filled as i16), false),
        Some(OrderStatusFilter::Cancelled) => (Some(OrderStatus::Cancelled as i16), false),
        Some(OrderStatusFilter::Expired) => (Some(OrderStatus::Expired as i16), false),
        Some(OrderStatusFilter::PendingTrigger) => {
            (Some(OrderStatus::PendingTrigger as i16), false)
        }
    };

    let user_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE user_id = $1)")
            .bind(user_id)
            .fetch_one(&state.pg_connections)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !user_exists {
        return Err(StatusCode::NOT_FOUND);
    }

    let market_id = match query.market {
        None => None,
        Some(symbol) => Some(
            sqlx::query_scalar::<_, i32>("SELECT market_id FROM markets WHERE symbol = $1")
                .bind(symbol)
                .fetch_optional(&state.pg_connections)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::NOT_FOUND)?,
        ),
    };

    let mut rows = sqlx::query_as::<_, OrderRow>(
        r#"SELECT order_id, idempotency_key, user_id, market_id, base_account_id,
                  quote_account_id, side, order_type, time_in_force, base_quantity,
                  quote_quantity, limit_price, trigger_price, trigger_direction,
                  protection_price,
                  to_char(expire_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS expire_time,
                  maker_fee_bps, taker_fee_bps, reserved_amount, submission, reject_reason,
                  reject_detail, status, cancel_reason, filled_base_quantity,
                  filled_quote_quantity,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS created_at,
                  to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS updated_at
           FROM orders
           WHERE user_id = $1
             AND ($2::int IS NULL OR market_id = $2)
             AND ($3::smallint IS NULL OR status = $3)
             AND (NOT $4::bool
                  OR (submission <> 'rejected' AND (status IS NULL OR status IN (1, 2, 6))))
             AND ($5::text IS NULL
                  OR (created_at, order_id) < ($5::text::timestamptz, $6::uuid))
           ORDER BY orders.created_at DESC, orders.order_id DESC
           LIMIT $7"#,
    )
    .bind(user_id)
    .bind(market_id)
    .bind(status)
    .bind(open_only)
    .bind(cursor_time)
    .bind(cursor_id)
    .bind(i64::from(limit) + 1)
    .fetch_all(&state.pg_connections)
    .await
    .map_err(
        |e| match e.as_database_error().and_then(|d| d.code()).as_deref() {
            Some("22007") | Some("22008") => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    )?;

    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_cursor = if has_more {
        rows.last()
            .map(|o| format!("{}_{}", o.created_at, o.order_id))
    } else {
        None
    };

    let mut market_ids: Vec<i32> = rows.iter().map(|o| o.market_id).collect();
    market_ids.sort_unstable();
    market_ids.dedup();
    let markets = sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets WHERE market_id = ANY($1)",
    )
    .bind(&market_ids)
    .fetch_all(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ledgers = load_ledgers(&state.pg_connections)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items = rows
        .into_iter()
        .map(|o| {
            let m = markets
                .iter()
                .find(|m| m.market_id == o.market_id)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            let base = ledgers
                .iter()
                .find(|l| l.ledger_id == m.base_ledger_id)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            let quote = ledgers
                .iter()
                .find(|l| l.ledger_id == m.quote_ledger_id)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
            Order::from_row(o, m, base, quote)
        })
        .collect::<Result<Vec<_>, StatusCode>>()?;

    Ok(Json(OrderPage { items, next_cursor }))
}

// gives the orders of a market (by symbol), newest first
// status can be used to filter, it can also be "open" which is every order that can still trade
// returns the page + next cursor
pub async fn list_market_orders(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(query): Query<MarketOrderQuery>,
) -> Result<Json<OrderPage>, StatusCode> {
    let limit = query.limit.unwrap_or(50).min(100);
    if limit == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (cursor_time, cursor_id) = match query.cursor {
        None => (None, None),
        Some(c) => {
            let (time, id) = c.split_once('_').ok_or(StatusCode::BAD_REQUEST)?;
            let id = id.parse::<Uuid>().map_err(|_| StatusCode::BAD_REQUEST)?;
            (Some(time.to_string()), Some(id))
        }
    };

    let (status, open_only) = match query.status {
        None => (None, false),
        Some(OrderStatusFilter::Open) => (None, true),
        Some(OrderStatusFilter::New) => (Some(OrderStatus::New as i16), false),
        Some(OrderStatusFilter::PartiallyFilled) => {
            (Some(OrderStatus::PartiallyFilled as i16), false)
        }
        Some(OrderStatusFilter::Filled) => (Some(OrderStatus::Filled as i16), false),
        Some(OrderStatusFilter::Cancelled) => (Some(OrderStatus::Cancelled as i16), false),
        Some(OrderStatusFilter::Expired) => (Some(OrderStatus::Expired as i16), false),
        Some(OrderStatusFilter::PendingTrigger) => {
            (Some(OrderStatus::PendingTrigger as i16), false)
        }
    };

    let m = sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets WHERE symbol = $1",
    )
    .bind(symbol)
    .fetch_optional(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let mut rows = sqlx::query_as::<_, OrderRow>(
        r#"SELECT order_id, idempotency_key, user_id, market_id, base_account_id,
                  quote_account_id, side, order_type, time_in_force, base_quantity,
                  quote_quantity, limit_price, trigger_price, trigger_direction,
                  protection_price,
                  to_char(expire_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS expire_time,
                  maker_fee_bps, taker_fee_bps, reserved_amount, submission, reject_reason,
                  reject_detail, status, cancel_reason, filled_base_quantity,
                  filled_quote_quantity,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS created_at,
                  to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS updated_at
           FROM orders
           WHERE market_id = $1
             AND ($2::smallint IS NULL OR status = $2)
             AND (NOT $3::bool
                  OR (submission <> 'rejected' AND (status IS NULL OR status IN (1, 2, 6))))
             AND ($4::text IS NULL
                  OR (created_at, order_id) < ($4::text::timestamptz, $5::uuid))
           ORDER BY orders.created_at DESC, orders.order_id DESC
           LIMIT $6"#,
    )
    .bind(m.market_id)
    .bind(status)
    .bind(open_only)
    .bind(cursor_time)
    .bind(cursor_id)
    .bind(i64::from(limit) + 1)
    .fetch_all(&state.pg_connections)
    .await
    .map_err(
        |e| match e.as_database_error().and_then(|d| d.code()).as_deref() {
            Some("22007") | Some("22008") => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    )?;

    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_cursor = if has_more {
        rows.last()
            .map(|o| format!("{}_{}", o.created_at, o.order_id))
    } else {
        None
    };

    let base = fetch_ledger_by_id(&state.pg_connections, m.base_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let quote = fetch_ledger_by_id(&state.pg_connections, m.quote_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let items = rows
        .into_iter()
        .map(|o| Order::from_row(o, &m, &base, &quote))
        .collect::<Result<Vec<_>, StatusCode>>()?;

    Ok(Json(OrderPage { items, next_cursor }))
}

// gives every trade an order took part in (as buyer or seller), oldest first
pub async fn list_order_trades(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Vec<Trade>>, StatusCode> {
    let market_id =
        sqlx::query_scalar::<_, i32>("SELECT market_id FROM orders WHERE order_id = $1")
            .bind(order_id)
            .fetch_optional(&state.pg_connections)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

    let m = sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets WHERE market_id = $1",
    )
    .bind(market_id)
    .fetch_one(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base = fetch_ledger_by_id(&state.pg_connections, m.base_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let quote = fetch_ledger_by_id(&state.pg_connections, m.quote_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = sqlx::query_as::<_, TradeRow>(
        r#"SELECT trade_id, market_id, buyer_order_id, seller_order_id, price, base_quantity,
                  quote_quantity, buyer_fee, seller_fee, taker_side, engine_epoch,
                  engine_sequence,
                  to_char(engine_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS engine_time,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS created_at
           FROM trades
           WHERE buyer_order_id = $1 OR seller_order_id = $1
           ORDER BY trades.engine_time ASC, engine_epoch ASC, engine_sequence ASC"#,
    )
    .bind(order_id)
    .fetch_all(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let trades = rows
        .into_iter()
        .map(|t| Trade::from_row(t, &m, &base, &quote))
        .collect::<Result<Vec<_>, StatusCode>>()?;

    Ok(Json(trades))
}
