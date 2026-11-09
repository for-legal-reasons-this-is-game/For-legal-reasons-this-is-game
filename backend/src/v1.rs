use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};

use num_traits::FromPrimitive;
use num_traits::ToPrimitive;
use primitive_types::U256;
use proto::{CancelOrdersRequest, PlaceOrderRequest, PlaceOrderResponse, RejectReason};
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::time::{Duration, timeout};
// #[derive(TS)] + #[ts(export)] generate TypeScript types for the frontend
// (`cargo test export_bindings` writes them to bindings/)
use ts_rs::TS;
use uuid::Uuid;

use crate::consumer::ids::{refund_transfer_id, reservation_transfer_id};
use crate::consumer::plan::fee_cap;
use crate::consumer::stream::{Backoff, clock_jitter};
use crate::domain::RejectReason as DomainRejectReason;
use crate::domain::{
    Account, AccountCodeType, AccountStatus, CancelReason, Ledger, Market, MarketRow, MarketStatus,
    Order, OrderPage, OrderRow, OrderStatus, OrderType, Position, Side, Submission, Ticker,
    TickerStats, TimeInForce, Trade, TradePage, TradeRow, Transfer, TransferDirection,
    TransferPage, TriggerDirection, User, to_decimal,
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
    /// Auto-reconnecting, cheaply cloneable Redis handle.
    pub redis: redis::aio::ConnectionManager,
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

pub async fn load_markets(pool: &PgPool) -> sqlx::Result<Vec<MarketRow>> {
    sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets ORDER BY market_id",
    )
    .fetch_all(pool)
    .await
}

pub async fn fetch_market_by_symbol(
    pool: &PgPool,
    symbol: &str,
) -> sqlx::Result<Option<MarketRow>> {
    sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets WHERE symbol = $1",
    )
    .bind(symbol)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_market_by_id(pool: &PgPool, market_id: i32) -> sqlx::Result<Option<MarketRow>> {
    sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets WHERE market_id = $1",
    )
    .bind(market_id)
    .fetch_optional(pool)
    .await
}

pub async fn fetch_markets_by_ids(
    pool: &PgPool,
    market_ids: &[i32],
) -> sqlx::Result<Vec<MarketRow>> {
    sqlx::query_as::<_, MarketRow>(
        "SELECT market_id, symbol, base_ledger_id, quote_ledger_id, status, price_decimals, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps \
         FROM markets WHERE market_id = ANY($1)",
    )
    .bind(market_ids)
    .fetch_all(pool)
    .await
}

// gives the (base, quote) ledgers of a market. Both have to exist, a market can't point to a missing ledger
pub async fn fetch_market_ledgers(
    pool: &PgPool,
    market: &MarketRow,
) -> Result<(Ledger, Ledger), StatusCode> {
    let base = fetch_ledger_by_id(pool, market.base_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let quote = fetch_ledger_by_id(pool, market.quote_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((base, quote))
}

// limit of a page: default 50, max 100, 0 makes no sense
fn page_limit(limit: Option<u32>) -> Result<u32, StatusCode> {
    match limit.unwrap_or(50).min(100) {
        0 => Err(StatusCode::BAD_REQUEST),
        limit => Ok(limit),
    }
}

// splits the cursor the client sends back ("<timestamp>_<uuid>") into its two parts
fn parse_cursor_pg(cursor: Option<String>) -> Result<(Option<String>, Option<Uuid>), StatusCode> {
    match cursor {
        None => Ok((None, None)),
        Some(c) => {
            let (time, id) = c.split_once('_').ok_or(StatusCode::BAD_REQUEST)?;
            let id = id.parse::<Uuid>().map_err(|_| StatusCode::BAD_REQUEST)?;
            Ok((Some(time.to_string()), Some(id)))
        }
    }
}

// gives (status to filter by, only open orders) from what the client asked for
fn order_status_filter(filter: Option<OrderStatusFilter>) -> (Option<i16>, bool) {
    match filter {
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
    }
}

// postgres rejecting the cursor's timestamp is the client's fault, anything else is ours
fn page_query_error(e: sqlx::Error) -> StatusCode {
    match e.as_database_error().and_then(|d| d.code()).as_deref() {
        Some("22007") | Some("22008") => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// the rows were asked for with limit + 1: cuts the extra one off and, if there was one,
// gives the cursor of the next page (built from the last row that stays)
fn next_page_cursor<T>(
    rows: &mut Vec<T>,
    limit: u32,
    cursor_of: impl Fn(&T) -> String,
) -> Option<String> {
    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    if has_more {
        rows.last().map(cursor_of)
    } else {
        None
    }
}

// gives the ledger an account lives in. It has to exist, an account can't point to a missing ledger
pub async fn fetch_account_ledger(pool: &PgPool, account: &Account) -> Result<Ledger, StatusCode> {
    fetch_ledger_by_id(pool, account.account_ledger_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
}

// looks for the (base, quote) ledgers of a market in an already loaded list of ledgers
fn find_market_ledgers<'a>(
    ledgers: &'a [Ledger],
    market: &MarketRow,
) -> Result<(&'a Ledger, &'a Ledger), StatusCode> {
    let find = |ledger_id| {
        ledgers
            .iter()
            .find(|l| l.ledger_id == ledger_id)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    };
    Ok((find(market.base_ledger_id)?, find(market.quote_ledger_id)?))
}

#[derive(Deserialize, TS)]
#[ts(export)]
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

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct LedgerEnabledPayload {
    enabled: bool,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct MarketPayload {
    symbol: String,
    base_ledger_symbol: String,
    quote_ledger_symbol: String,
    price_decimals: i16,
    #[ts(type = "string")]
    min_base_quantity: Decimal,
    #[ts(type = "string")]
    min_quote_quantity: Decimal,
    #[ts(type = "string | null")]
    quantity_step: Option<Decimal>, // none means 1
    #[ts(type = "string | null")]
    price_step: Option<Decimal>, // same
    maker_fee_bps: i32,
    taker_fee_bps: i32,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct DepositPayload {
    idempotency_key: Uuid, // frontend-generated IN UUIDv7!
    #[ts(type = "string")]
    amount: Decimal, // in practice, this is sent as a string, important because of no rounding should happen
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct TransferQuery {
    limit: Option<u32>, // amount of transfers to return requested by the frontend (if none there's default of 50)
    cursor: Option<String>, // where the page starts, nothing == first ones, Some == the one with the timestamp + the other limit - 1 amount
}

#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrderStatusFilter {
    Open,
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Expired,
    PendingTrigger,
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct OrderQuery {
    market: Option<String>,            // symbol of the market, none == all of them
    status: Option<OrderStatusFilter>, // status of the order, none == all of them
    limit: Option<u32>,                // same as in transferquery: default 50, max 100
    cursor: Option<String>, // the next_cursor of the previous page, nothing == first page
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct MarketOrderQuery {
    status: Option<OrderStatusFilter>, // status of the order, none == all of them
    limit: Option<u32>,                // same as in transferquery: default 50, max 100
    cursor: Option<String>, // the next_cursor of the previous page, nothing == first page
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct MarketTradeQuery {
    limit: Option<u32>,     // same as in transferquery: default 50, max 100
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
        Ok(TransferCodeType::Refund) => "refund",
        Ok(TransferCodeType::CloseAccount) | Err(_) => "unknown",
    }
}

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct OrderPayload {
    idempotency_key: Uuid, // frontend-generated
    market: String,
    base_account_id: Uuid,
    quote_account_id: Uuid,
    side: Side,
    order_type: OrderType,
    time_in_force: TimeInForce,
    // base_quantity for everything except market/stop_market buys, those use quote_quantity
    #[ts(type = "string | null")]
    base_quantity: Option<Decimal>,
    #[ts(type = "string | null")]
    quote_quantity: Option<Decimal>,
    #[ts(type = "string | null")]
    limit_price: Option<Decimal>, // limit and stop_limit only
    #[ts(type = "string | null")]
    trigger_price: Option<Decimal>, // stop_limit and stop_market only
    trigger_direction: Option<TriggerDirection>, // same as trigger_price
    #[ts(type = "string | null")]
    protection_price: Option<Decimal>, // optional cap/floor, not for limit orders
    expire_time: Option<String>,                 //gtd only
}

// converts an amount in human units to the smallest unit of an asset with `decimals` decimals
fn to_units(amount: Decimal, decimals: u32) -> Result<u128, StatusCode> {
    if amount <= Decimal::ZERO {
        return Err(StatusCode::BAD_REQUEST);
    }
    let factor = 10u64.checked_pow(decimals).ok_or(StatusCode::BAD_REQUEST)?;
    let scaled = amount
        .checked_mul(Decimal::from(factor))
        .ok_or(StatusCode::BAD_REQUEST)?;
    if !scaled.fract().is_zero() {
        return Err(StatusCode::BAD_REQUEST);
    }
    scaled.to_u128().ok_or(StatusCode::BAD_REQUEST)
}

// quote = price * base / 10^(price_decimals + base_decimals - quote_decimals),
// rounded down. None if the exponent is negative/doesn't fit or on overflow
fn quote_for(
    price: u128,
    base: u128,
    price_decimals: u32,
    base_decimals: u32,
    quote_decimals: u32,
) -> Option<u128> {
    let exponent = price_decimals
        .checked_add(base_decimals)?
        .checked_sub(quote_decimals)?;
    if exponent > 38 {
        return None;
    }
    let quote = U256::from(price) * U256::from(base) / U256::from(10u128.pow(exponent));
    u128::try_from(quote).ok()
}

//calculates what an order has to reserve. sell reserves base quantity, buy reserves the quote amount
// plus a fee at the higher of the two rates (rounded up)
fn reservation_amount(
    side: Side,
    order_type: OrderType,
    base: Option<u128>,
    quote: Option<u128>,
    limit_price: Option<u128>,
    scale: (u32, u32, u32),
    fees: (i32, i32),
) -> Result<u128, StatusCode> {
    if side == Side::Sell {
        return base.ok_or(StatusCode::BAD_REQUEST);
    }
    let amount = match order_type {
        OrderType::Market | OrderType::StopMarket => quote.ok_or(StatusCode::BAD_REQUEST)?,
        OrderType::Limit | OrderType::StopLimit => quote_for(
            limit_price.ok_or(StatusCode::BAD_REQUEST)?,
            base.ok_or(StatusCode::BAD_REQUEST)?,
            scale.0,
            scale.1,
            scale.2,
        )
        .ok_or(StatusCode::BAD_REQUEST)?,
    };
    if amount == 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    amount
        .checked_add(fee_cap(amount, fees.0.max(fees.1)))
        .ok_or(StatusCode::BAD_REQUEST)
}

// checks that the fields that are set are the ones the order type needs.
fn validate_shape(p: &OrderPayload) -> Result<(), StatusCode> {
    let has_limit = matches!(p.order_type, OrderType::Limit | OrderType::StopLimit);
    let is_stop = matches!(p.order_type, OrderType::StopLimit | OrderType::StopMarket);
    // market and stop_market buys are sized in quote, every other order in base
    let quote_sized =
        p.side == Side::Buy && matches!(p.order_type, OrderType::Market | OrderType::StopMarket);
    let one_quantity = if quote_sized {
        p.quote_quantity.is_some() && p.base_quantity.is_none()
    } else {
        p.base_quantity.is_some() && p.quote_quantity.is_none()
    };
    let ok = one_quantity
        && p.limit_price.is_some() == has_limit
        && p.trigger_price.is_some() == is_stop
        && p.trigger_direction.is_some() == is_stop
        && p.expire_time.is_some() == (p.time_in_force == TimeInForce::Gtd)
        && !(has_limit && p.protection_price.is_some());
    if ok {
        Ok(())
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

// checks the market rules the engine doesn't check
fn check_market_rules(
    m: &MarketRow,
    base: Option<u128>,
    quote_value: Option<u128>,
    prices: &[u128],
) -> Result<(), StatusCode> {
    let config = |d: Decimal| d.to_u128().ok_or(StatusCode::INTERNAL_SERVER_ERROR);
    let (min_base, min_quote) = (config(m.min_base_quantity)?, config(m.min_quote_quantity)?);
    let (quantity_step, price_step) = (config(m.quantity_step)?, config(m.price_step)?);
    if quantity_step == 0 || price_step == 0 {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    if base.is_some_and(|b| b < min_base || b % quantity_step != 0) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if quote_value.is_some_and(|q| q < min_quote) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if prices.iter().any(|p| p % price_step != 0) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct PlaceRow {
    order_id: Uuid,
    command_id: Uuid,
    user_id: Uuid,
    market_symbol: String,
    base_account_id: Uuid,
    quote_account_id: Uuid,
    side: Side,
    order_type: OrderType,
    time_in_force: TimeInForce,
    base_quantity: Option<u128>,
    quote_quantity: Option<u128>,
    limit_price: Option<u128>,
    trigger_price: Option<u128>,
    trigger_direction: Option<TriggerDirection>,
    protection_price: Option<u128>,
    expire_micros: Option<i64>,
    created_micros: i64,
    maker_fee_bps: u32,
    taker_fee_bps: u32,
}

// engine's answer to an order, only for the reply of placeorder
#[derive(Debug, Clone, PartialEq)]
pub struct EngineAnswer {
    pub status: Option<OrderStatus>,
    pub cancel_reason: Option<CancelReason>,
    pub filled_base: Option<u128>,
    pub filled_quote: Option<u128>,
}

// what a PlaceOrder attempt means for the order
#[derive(Debug, PartialEq)]
enum PlaceVerdict {
    // the engine took the order
    Accepted(EngineAnswer),
    // the engine refused
    Rejected(DomainRejectReason),
    //timeout/error occurred so send the same command again
    Retry,
    // an answer that doesn't fit, in this case we just log (because something is very very wrong so shouldnt happen usually)
    Anomaly,
}

fn timestamp_from_micros(micros: i64) -> proto::Timestamp {
    proto::Timestamp {
        seconds: micros.div_euclid(1_000_000),
        nanos: (micros.rem_euclid(1_000_000) * 1_000) as i32,
    }
}

// builds the PlaceOrderRequest for an order from its stored row
fn build_place_request(row: &PlaceRow) -> PlaceOrderRequest {
    let uint = |v: u128| proto::Uint128::from(v);
    let amount = match (row.quote_quantity, row.base_quantity) {
        (Some(q), _) => Some(proto::place_order_request::Amount::QuoteQuantity(uint(q))),
        (None, Some(b)) => Some(proto::place_order_request::Amount::BaseQuantity(uint(b))),
        (None, None) => None,
    };
    PlaceOrderRequest {
        command_id: Some(row.command_id.as_u128().into()),
        order_id: Some(row.order_id.as_u128().into()),
        market_symbol: row.market_symbol.clone(),
        account_holder: Some(row.user_id.as_u128().into()),
        base_account: Some(row.base_account_id.as_u128().into()),
        quote_account: Some(row.quote_account_id.as_u128().into()),
        side: proto::Side::from(row.side) as i32,
        r#type: proto::OrderType::from(row.order_type) as i32,
        time_in_force: proto::TimeInForce::from(row.time_in_force) as i32,
        amount,
        limit_price: row.limit_price.map(uint),
        trigger_price: row.trigger_price.map(uint),
        trigger_direction: row
            .trigger_direction
            .map(|t| proto::TriggerDirection::from(t) as i32)
            .unwrap_or_default(),
        protection_price: row.protection_price.map(uint),
        expire_time: row.expire_micros.map(timestamp_from_micros),
        sent_at: Some(timestamp_from_micros(row.created_micros)),
        maker_fee_bps: row.maker_fee_bps,
        taker_fee_bps: row.taker_fee_bps,
    }
}

// reads what a PlaceOrder attempt means and gives a verdict
fn classify_place(
    result: Result<Result<PlaceOrderResponse, tonic::Status>, tokio::time::error::Elapsed>,
) -> PlaceVerdict {
    let response = match result {
        Ok(Ok(response)) => response,
        Ok(Err(_)) | Err(_) => return PlaceVerdict::Retry,
    };
    if response.reject_reason == 0 {
        return PlaceVerdict::Accepted(EngineAnswer {
            status: OrderStatus::from_i32(response.status),
            cancel_reason: CancelReason::from_i32(response.cancel_reason),
            filled_base: response.filled_base_quantity.map(u128::from),
            filled_quote: response.filled_quote_quantity.map(u128::from),
        });
    }
    match DomainRejectReason::from_i32(response.reject_reason) {
        Some(DomainRejectReason::DuplicateCommand) | None => PlaceVerdict::Anomaly,
        Some(reason) => PlaceVerdict::Rejected(reason),
    }
}

// timeout for tb
const ORDER_STEP_TIMEOUT: Duration = Duration::from_secs(2);

fn parse_u128(s: &str) -> Result<u128, StatusCode> {
    s.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn parse_u128_opt(s: &Option<String>) -> Result<Option<u128>, StatusCode> {
    s.as_deref().map(parse_u128).transpose()
}

// an order as the steps of advance_order need it.
#[derive(sqlx::FromRow)]
struct AdvanceRow {
    order_id: Uuid,
    command_id: Uuid,
    user_id: Uuid,
    symbol: String,
    base_account_id: Uuid,
    quote_account_id: Uuid,
    side: Side,
    order_type: OrderType,
    time_in_force: TimeInForce,
    base_quantity: Option<String>,
    quote_quantity: Option<String>,
    limit_price: Option<String>,
    trigger_price: Option<String>,
    trigger_direction: Option<TriggerDirection>,
    protection_price: Option<String>,
    expire_micros: Option<i64>,
    created_micros: i64,
    maker_fee_bps: i32,
    taker_fee_bps: i32,
    reserved_amount: String,
    submission: Submission,
    reject_reason: Option<i16>,
    base_ledger_id: i32,
    quote_ledger_id: i32,
}

impl AdvanceRow {
    //gives account and ledger id of the correct reserved asset
    fn reserved_asset(&self) -> (Uuid, i32) {
        match self.side {
            Side::Buy => (self.quote_account_id, self.quote_ledger_id),
            Side::Sell => (self.base_account_id, self.base_ledger_id),
        }
    }
}

// Where the order is at
#[derive(Debug)]
pub struct Advance {
    pub submission: Submission,
    pub finished: bool, // is something else needed?
    pub answer: Option<EngineAnswer>,
}

// Does the next step of an order (as many as possible in a row): hold account + reservation
// (reserving), PlaceOrder (sent), the refund of a rejection. A step that can't finish just stops and the order
// stays where it is, the relay picks up
pub async fn advance_order(
    state: &AppState,
    order_id: Uuid,
    engine_attempts: u32,
) -> Result<Advance, StatusCode> {
    let mut answer = None;
    let mut last = Submission::Reserving;
    // each pass does the step the stored state asks for, then reads the state again. its only 3 steps and it should stop after those 3 are done,
    // but i put 4 for an extra retry.
    for _ in 0..4 {
        let row = sqlx::query_as::<_, AdvanceRow>(
            r#"SELECT o.order_id, o.command_id, o.user_id, m.symbol, o.base_account_id,
                      o.quote_account_id, o.side, o.order_type, o.time_in_force,
                      o.base_quantity::text AS base_quantity,
                      o.quote_quantity::text AS quote_quantity,
                      o.limit_price::text AS limit_price, o.trigger_price::text AS trigger_price,
                      o.trigger_direction, o.protection_price::text AS protection_price,
                      (extract(epoch from o.expire_time) * 1000000)::bigint AS expire_micros,
                      (extract(epoch from o.created_at) * 1000000)::bigint AS created_micros,
                      o.maker_fee_bps, o.taker_fee_bps, o.reserved_amount::text AS reserved_amount,
                      o.submission, o.reject_reason, m.base_ledger_id, m.quote_ledger_id
               FROM orders o JOIN markets m ON m.market_id = o.market_id
               WHERE o.order_id = $1"#,
        )
        .bind(order_id)
        .fetch_optional(&state.pg_connections)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
        last = row.submission;
        match row.submission {
            Submission::Accepted => {
                return Ok(Advance {
                    submission: last,
                    finished: true,
                    answer,
                });
            }
            Submission::Rejected => {
                let finished = row.reject_reason.is_none() || refund(state, &row).await?;
                return Ok(Advance {
                    submission: last,
                    finished,
                    answer,
                });
            }
            Submission::Reserving => {
                if !reserve(state, &row).await? {
                    break;
                }
            }
            Submission::Sent => match send(state, &row, engine_attempts).await? {
                Sent::Moved(engine_answer) => answer = engine_answer.or(answer),
                Sent::Stuck => break,
            },
        }
    }
    Ok(Advance {
        submission: last,
        finished: false,
        answer,
    })
}

// creates the hold account, moves the money into it and marks the order `sent` or marks it
// `rejected` when there's no funds. false if something couldn't be done right now
async fn reserve(state: &AppState, row: &AdvanceRow) -> Result<bool, StatusCode> {
    let hold_id = row.order_id.as_u128();
    let (user_account, ledger) = row.reserved_asset();

    let hold = tb::Account {
        id: hold_id,
        ledger: ledger as u32,
        code: AccountCodeType::Hold as u16,
        user_data_128: row.user_id.as_u128(),
        flags: AccountCodeType::Hold.tb_flags(),
        ..Default::default()
    };
    match timeout(ORDER_STEP_TIMEOUT, state.tb_client.create_accounts(&[hold])).await {
        Ok(Ok(results)) => {
            let fine = results.first().is_none_or(|r| {
                matches!(
                    r.result,
                    tb::CreateAccountResult::Ok | tb::CreateAccountResult::Exists
                )
            });
            if !fine {
                println!(
                    "ORDER: tigerbeetle rejected the hold account of order {}: {:?}",
                    row.order_id,
                    results.first().map(|r| &r.result)
                );
                return Ok(false);
            }
        }
        Ok(Err(e)) => {
            println!(
                "ORDER: connection to tigerbeetle failed, order {} will be retried: {e}",
                row.order_id
            );
            return Ok(false);
        }
        Err(_) => {
            println!(
                "ORDER: tigerbeetle timed out, order {} will be retried",
                row.order_id
            );
            return Ok(false);
        }
    }

    let reservation = tb::Transfer {
        id: reservation_transfer_id(hold_id),
        debit_account_id: user_account.as_u128(),
        credit_account_id: hold_id,
        amount: parse_u128(&row.reserved_amount)?,
        ledger: ledger as u32,
        code: TransferCodeType::Hold as u16,
        ..Default::default()
    };
    let results = match timeout(
        ORDER_STEP_TIMEOUT,
        state.tb_client.create_transfers(&[reservation]),
    )
    .await
    {
        Ok(Ok(results)) => results,
        Ok(Err(e)) => {
            println!(
                "ORDER: connection to tigerbeetle failed, order {} will be retried: {e}",
                row.order_id
            );
            return Ok(false);
        }
        Err(_) => {
            println!(
                "ORDER: tigerbeetle timed out, order {} will be retried",
                row.order_id
            );
            return Ok(false);
        }
    };
    let detail = match results.first().map(|r| &r.result) {
        None | Some(CreateTransferResult::Ok) | Some(CreateTransferResult::Exists) => None,
        Some(CreateTransferResult::ExceedsCredits) => Some("insufficient funds"),
        Some(CreateTransferResult::DebitAccountAlreadyClosed) => {
            Some("the account is closed, so nothing could be reserved")
        }
        Some(other) => {
            println!(
                "ORDER: unexpected tigerbeetle result for order {}: {other:?}",
                row.order_id
            );
            return Ok(false);
        }
    };

    let moved = match detail {
        None => {
            sqlx::query(
                "UPDATE orders SET submission = 'sent' WHERE order_id = $1 AND submission = 'reserving'",
            )
            .bind(row.order_id)
            .execute(&state.pg_connections)
            .await
        }
        Some(detail) => {
            sqlx::query(
                "UPDATE orders SET submission = 'rejected', reject_detail = $2 \
                 WHERE order_id = $1 AND submission = 'reserving'",
            )
            .bind(row.order_id)
            .bind(detail)
            .execute(&state.pg_connections)
            .await
        }
    };
    moved.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(true)
}

enum Sent {
    // the order's state in postgres changed
    Moved(Option<EngineAnswer>),
    // no usable answer, same state, try again later
    Stuck,
}

// sends PlaceOrder, up to `attempts` and records what the engine says
async fn send(state: &AppState, row: &AdvanceRow, attempts: u32) -> Result<Sent, StatusCode> {
    let request = build_place_request(&PlaceRow {
        order_id: row.order_id,
        command_id: row.command_id,
        user_id: row.user_id,
        market_symbol: row.symbol.clone(),
        base_account_id: row.base_account_id,
        quote_account_id: row.quote_account_id,
        side: row.side,
        order_type: row.order_type,
        time_in_force: row.time_in_force,
        base_quantity: parse_u128_opt(&row.base_quantity)?,
        quote_quantity: parse_u128_opt(&row.quote_quantity)?,
        limit_price: parse_u128_opt(&row.limit_price)?,
        trigger_price: parse_u128_opt(&row.trigger_price)?,
        trigger_direction: row.trigger_direction,
        protection_price: parse_u128_opt(&row.protection_price)?,
        expire_micros: row.expire_micros,
        created_micros: row.created_micros,
        maker_fee_bps: u32::try_from(row.maker_fee_bps)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        taker_fee_bps: u32::try_from(row.taker_fee_bps)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    });
    let mut backoff = Backoff::new(Duration::from_millis(100), Duration::from_secs(1));
    let mut verdict = PlaceVerdict::Retry;
    for attempt in 0..attempts.max(1) {
        if attempt > 0 {
            tokio::time::sleep(backoff.next_delay(clock_jitter())).await;
        }
        verdict = classify_place(
            timeout(
                ORDER_STEP_TIMEOUT,
                state.engine_client.place_order(request.clone()),
            )
            .await,
        );
        if verdict != PlaceVerdict::Retry {
            break;
        }
    }
    match verdict {
        PlaceVerdict::Accepted(answer) => {
            sqlx::query(
                "UPDATE orders SET submission = 'accepted' WHERE order_id = $1 AND submission = 'sent'",
            )
            .bind(row.order_id)
            .execute(&state.pg_connections)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Sent::Moved(Some(answer)))
        }
        PlaceVerdict::Rejected(reason) => {
            sqlx::query(
                "UPDATE orders SET submission = 'rejected', reject_reason = $2, reject_detail = $3 \
                 WHERE order_id = $1 AND submission = 'sent'",
            )
            .bind(row.order_id)
            .bind(reason as i16)
            .bind("rejected by the engine")
            .execute(&state.pg_connections)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Sent::Moved(None))
        }
        PlaceVerdict::Retry => {
            println!(
                "ORDER: no answer from the engine for order {}, will retry",
                row.order_id
            );
            Ok(Sent::Stuck)
        }
        PlaceVerdict::Anomaly => {
            println!(
                "ORDER: unexpected answer from the engine for order {}, left as it is",
                row.order_id
            );
            Ok(Sent::Stuck)
        }
    }
}

// gives the rest of the hold account back to the user. true when it is done
async fn refund(state: &AppState, row: &AdvanceRow) -> Result<bool, StatusCode> {
    let (user_account, ledger) = row.reserved_asset();
    let transfer = tb::Transfer {
        id: refund_transfer_id(row.order_id.as_u128()),
        debit_account_id: row.order_id.as_u128(),
        credit_account_id: user_account.as_u128(),
        amount: u128::MAX,
        ledger: ledger as u32,
        code: TransferCodeType::Refund as u16,
        flags: tb::TransferFlags::BalancingDebit,
        ..Default::default()
    };
    match timeout(
        ORDER_STEP_TIMEOUT,
        state.tb_client.create_transfers(&[transfer]),
    )
    .await
    {
        Ok(Ok(results)) => match results.first().map(|r| &r.result) {
            None | Some(CreateTransferResult::Ok) | Some(CreateTransferResult::Exists) => Ok(true),
            Some(other) => {
                println!(
                    "ORDER: refund of order {} was refused: {other:?}",
                    row.order_id
                );
                Ok(false)
            }
        },
        Ok(Err(e)) => {
            println!(
                "ORDER: connection to tigerbeetle failed, refund of order {} will be retried: {e}",
                row.order_id
            );
            Ok(false)
        }
        Err(_) => {
            println!(
                "ORDER: tigerbeetle timed out, refund of order {} will be retried",
                row.order_id
            );
            Ok(false)
        }
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

    let ledger = fetch_account_ledger(&state.pg_connections, &account).await?;
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

    let ledger = fetch_account_ledger(&state.pg_connections, &account).await?;
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
    let limit = page_limit(query.limit)?;

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

    let decimals = fetch_account_ledger(&state.pg_connections, &account)
        .await?
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

// creates a market. It starts halted but its activated once the engine confirms it, which can fail,
// so there's a relay to pick that up too.
// The fee account is the quote ledger's system fee account
pub async fn create_market(
    State(state): State<AppState>,
    Json(payload): Json<MarketPayload>,
) -> Result<(StatusCode, Json<Market>), StatusCode> {
    if payload.symbol.trim().is_empty()
        || !(0..=18).contains(&payload.price_decimals)
        || !(0..=10_000).contains(&payload.maker_fee_bps)
        || !(0..=10_000).contains(&payload.taker_fee_bps)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut ledgers = Vec::new();
    for symbol in [&payload.base_ledger_symbol, &payload.quote_ledger_symbol] {
        match fetch_ledger_by_symbol(&state.pg_connections, symbol)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            None => return Err(StatusCode::NOT_FOUND),
            Some(l) if !l.enabled => return Err(StatusCode::BAD_REQUEST),
            Some(l) => ledgers.push(l),
        }
    }
    let (base, quote) = (ledgers.remove(0), ledgers.remove(0));
    if base.ledger_id == quote.ledger_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    let exponent = payload.price_decimals as i32 + base.decimals as i32 - quote.decimals as i32;
    if !(0..=38).contains(&exponent) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let min_base = to_units(payload.min_base_quantity, base.decimals as u32)?;
    let min_quote = to_units(payload.min_quote_quantity, quote.decimals as u32)?;
    let quantity_step = match payload.quantity_step {
        Some(step) => to_units(step, base.decimals as u32)?,
        None => 1,
    };
    let price_step = match payload.price_step {
        Some(step) => to_units(step, payload.price_decimals as u32)?,
        None => 1,
    };

    let fee_account = sqlx::query_scalar::<_, Uuid>(
        "SELECT account_id FROM accounts WHERE account_ledger_id = $1 AND account_code_type = 2",
    )
    .bind(quote.ledger_id)
    .fetch_optional(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut tx = state
        .pg_connections
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let market_id = sqlx::query_scalar::<_, i32>(
        "INSERT INTO markets (symbol, base_ledger_id, quote_ledger_id, price_decimals, fee_account_id, \
                min_base_quantity, min_quote_quantity, quantity_step, price_step, \
                maker_fee_bps, taker_fee_bps) \
         VALUES ($1, $2, $3, $4, $5, $6::numeric, $7::numeric, $8::numeric, $9::numeric, $10, $11) \
         RETURNING market_id",
    )
    .bind(payload.symbol.trim())
    .bind(base.ledger_id)
    .bind(quote.ledger_id)
    .bind(payload.price_decimals)
    .bind(fee_account)
    .bind(min_base.to_string())
    .bind(min_quote.to_string())
    .bind(quantity_step.to_string())
    .bind(price_step.to_string())
    .bind(payload.maker_fee_bps)
    .bind(payload.taker_fee_bps)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => StatusCode::CONFLICT,
        sqlx::Error::Database(db) if db.is_check_violation() => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    sqlx::query("INSERT INTO market_outbox (market_id) VALUES ($1)")
        .bind(market_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let created = match advance_market(&state, market_id).await? {
        MarketVerdict::Accepted => match activate_market(&state.pg_connections, market_id).await {
            Ok(()) => true,
            Err(e) => {
                println!("MARKET: couldn't activate market {market_id}: {e}");
                false
            }
        },
        MarketVerdict::Refused => {
            let mut tx = state
                .pg_connections
                .begin()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            remove_market(&mut tx, market_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            tx.commit()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Err(StatusCode::BAD_REQUEST);
        }
        MarketVerdict::Retry | MarketVerdict::Anomaly => false,
    };

    let row = fetch_market_by_id(&state.pg_connections, market_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::ACCEPTED
    };
    Ok((status, Json(Market::from_row(row, base, quote)?)))
}

// what the engine said to a SetMarket
#[derive(Debug, PartialEq)]
pub enum MarketVerdict {
    Accepted,
    Retry,   // needs to be retried
    Refused, // BAD_DEFINITION: the market can't exist
    Anomaly, // an answer a retry can't fix, its logged only (something very wrong in our code)
}

// One attempt at telling the engine about a market
//repeating it is safe
pub async fn advance_market(state: &AppState, market_id: i32) -> Result<MarketVerdict, StatusCode> {
    let (symbol, price_decimals, base_decimals, quote_decimals, fee_account) =
        sqlx::query_as::<_, (String, i16, i16, i16, Uuid)>(
            "SELECT m.symbol, m.price_decimals, b.decimals, q.decimals, m.fee_account_id \
             FROM markets m \
             JOIN ledgers b ON b.ledger_id = m.base_ledger_id \
             JOIN ledgers q ON q.ledger_id = m.quote_ledger_id \
             WHERE m.market_id = $1",
        )
        .bind(market_id)
        .fetch_optional(&state.pg_connections)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let request = proto::SetMarketRequest {
        market: Some(proto::Market {
            symbol,
            base_decimals: base_decimals as u32,
            quote_decimals: quote_decimals as u32,
            price_decimals: price_decimals as u32,
            fee_account: Some(fee_account.as_u128().into()),
        }),
    };
    match timeout(ORDER_STEP_TIMEOUT, state.engine_client.set_market(request)).await {
        Err(_) => {
            println!("MARKET: the engine didn't answer SetMarket for market {market_id} in time");
            Ok(MarketVerdict::Retry)
        }
        Ok(Err(e)) => {
            println!("MARKET: SetMarket for market {market_id} failed: {e}");
            Ok(MarketVerdict::Retry)
        }
        Ok(Ok(response)) => match response.reject_reason {
            0 => Ok(MarketVerdict::Accepted),
            1 => {
                println!(
                    "MARKET: the engine refused the definition of market {market_id}: {}",
                    response.reject_detail
                );
                Ok(MarketVerdict::Refused)
            }
            other => {
                println!(
                    "MARKET: unexpected SetMarket answer {other} for market {market_id}: {}",
                    response.reject_detail
                );
                Ok(MarketVerdict::Anomaly)
            }
        },
    }
}

// takes a market out
pub async fn remove_market(
    conn: &mut sqlx::PgConnection,
    market_id: i32,
) -> Result<(), sqlx::Error> {
    for sql in [
        "DELETE FROM market_outbox WHERE market_id = $1",
        "DELETE FROM markets WHERE market_id = $1",
    ] {
        sqlx::query(sql).bind(market_id).execute(&mut *conn).await?;
    }
    Ok(())
}

// stamps the outbox row and turns it on, only happens once either directly in the backend or via the relay
pub async fn activate_market(
    executor: impl sqlx::PgExecutor<'_>,
    market_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "WITH done AS ( \
             UPDATE market_outbox SET processed_at = now() \
             WHERE market_id = $1 AND processed_at IS NULL \
             RETURNING market_id \
         ) \
         UPDATE markets SET status = 'trading' \
         WHERE market_id IN (SELECT market_id FROM done)",
    )
    .bind(market_id)
    .execute(executor)
    .await?;
    Ok(())
}

// lists all markets
pub async fn list_markets(State(state): State<AppState>) -> Result<Json<Vec<Market>>, StatusCode> {
    let market_rows = load_markets(&state.pg_connections)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let ledgers = load_ledgers(&state.pg_connections)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let markets = market_rows
        .into_iter()
        .map(|m| {
            let (base, quote) = find_market_ledgers(&ledgers, &m)?;
            Market::from_row(m, base.clone(), quote.clone())
        })
        .collect::<Result<Vec<_>, StatusCode>>()?;
    Ok(Json(markets))
}

// fetches a specific market by its symbol
pub async fn fetch_market(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Market>, StatusCode> {
    let m = fetch_market_by_symbol(&state.pg_connections, &symbol)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let (base, quote) = fetch_market_ledgers(&state.pg_connections, &m).await?;

    Ok(Json(Market::from_row(m, base, quote)?))
}

// how many times the backend itself sends PlaceOrder, after that the relay keeps trying
const HANDLER_ENGINE_ATTEMPTS: u32 = 2;

// figures out the HTTP answer for PlaceOrder and gives it back.
fn order_outcome(
    submission: Submission,
    reject_reason: Option<DomainRejectReason>,
    fresh: bool,
) -> Result<StatusCode, StatusCode> {
    match submission {
        Submission::Accepted if fresh => Ok(StatusCode::CREATED),
        Submission::Accepted => Ok(StatusCode::OK),
        Submission::Reserving | Submission::Sent => Ok(StatusCode::ACCEPTED),
        Submission::Rejected => Err(match reject_reason {
            // not enough funds, closed account
            None => StatusCode::CONFLICT,
            Some(DomainRejectReason::UnknownMarket) => StatusCode::NOT_FOUND,
            Some(
                DomainRejectReason::BadQuantity
                | DomainRejectReason::BadPrice
                | DomainRejectReason::BadOrderParams
                | DomainRejectReason::Unsupported,
            ) => StatusCode::BAD_REQUEST,
            Some(_) => StatusCode::SERVICE_UNAVAILABLE,
        }),
    }
}

// checks that this user can trade with the account on this ledger
async fn check_order_account(
    pool: &PgPool,
    account_id: Uuid,
    user_id: Uuid,
    ledger_id: i32,
) -> Result<(), StatusCode> {
    let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    if account.account_user_id != user_id || account.account_code_type != AccountCodeType::User {
        return Err(StatusCode::FORBIDDEN);
    }
    if account.account_status != AccountStatus::Active {
        return Err(StatusCode::CONFLICT);
    }
    if account.account_ledger_id != ledger_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

// a database error about the text the client sent (a bad format, bad characters: class 22) is the
// client's fault, anything else is ours
fn bad_text_or_500(e: sqlx::Error) -> StatusCode {
    match e.as_database_error().and_then(|d| d.code()).as_deref() {
        Some(code) if code.starts_with("22") => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// the quantities and prices of an order as text
struct OrderTexts {
    base: Option<String>,
    quote: Option<String>,
    limit: Option<String>,
    trigger: Option<String>,
    protection: Option<String>,
}

// looks for the order an idempotency_key was used for. Some((order_id, same)): same says whether its the same order or simething else
async fn find_order_by_key(
    pool: &PgPool,
    user_id: Uuid,
    market_id: i32,
    payload: &OrderPayload,
    t: &OrderTexts,
) -> Result<Option<(Uuid, bool)>, StatusCode> {
    sqlx::query_as::<_, (Uuid, bool)>(
        "SELECT order_id, (user_id = $2 AND market_id = $3 AND base_account_id = $4 \
         AND quote_account_id = $5 AND side = $6 AND order_type = $7 \
         AND time_in_force = $8 \
         AND base_quantity IS NOT DISTINCT FROM $9::numeric \
         AND quote_quantity IS NOT DISTINCT FROM $10::numeric \
         AND limit_price IS NOT DISTINCT FROM $11::numeric \
         AND trigger_price IS NOT DISTINCT FROM $12::numeric \
         AND trigger_direction IS NOT DISTINCT FROM $13 \
         AND protection_price IS NOT DISTINCT FROM $14::numeric \
         AND expire_time IS NOT DISTINCT FROM $15::text::timestamptz) AS same \
         FROM orders WHERE idempotency_key = $1",
    )
    .bind(payload.idempotency_key)
    .bind(user_id)
    .bind(market_id)
    .bind(payload.base_account_id)
    .bind(payload.quote_account_id)
    .bind(payload.side)
    .bind(payload.order_type)
    .bind(payload.time_in_force)
    .bind(&t.base)
    .bind(&t.quote)
    .bind(&t.limit)
    .bind(&t.trigger)
    .bind(payload.trigger_direction)
    .bind(&t.protection)
    .bind(&payload.expire_time)
    .fetch_optional(pool)
    .await
    .map_err(bad_text_or_500)
}

// Places an order, doing checks in the way: first that all the fields are properly set in the request,
// then if the user/market exists, then the units and then if the idempotency key was used before (and in that case we give the order back so retrying is safe).
// it saves the order and tries to complete it, whatever isnt finished is completed by the order relay.
pub async fn create_order(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<OrderPayload>,
) -> Result<(StatusCode, Json<Order>), StatusCode> {
    validate_shape(&payload)?;

    let user_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE user_id = $1)")
            .bind(user_id)
            .fetch_one(&state.pg_connections)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !user_exists {
        return Err(StatusCode::NOT_FOUND);
    }

    let market = fetch_market_by_symbol(&state.pg_connections, &payload.market)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let (base, quote) = fetch_market_ledgers(&state.pg_connections, &market).await?;

    // everything from here on is in the smallest unit of each asset
    let (base_dec, quote_dec, price_dec) = (
        base.decimals as u32,
        quote.decimals as u32,
        market.price_decimals as u32,
    );
    let base_quantity = payload
        .base_quantity
        .map(|d| to_units(d, base_dec))
        .transpose()?;
    let quote_quantity = payload
        .quote_quantity
        .map(|d| to_units(d, quote_dec))
        .transpose()?;
    let limit_price = payload
        .limit_price
        .map(|d| to_units(d, price_dec))
        .transpose()?;
    let trigger_price = payload
        .trigger_price
        .map(|d| to_units(d, price_dec))
        .transpose()?;
    let protection_price = payload
        .protection_price
        .map(|d| to_units(d, price_dec))
        .transpose()?;
    let text = |v: Option<u128>| v.map(|v| v.to_string());
    let texts = OrderTexts {
        base: text(base_quantity),
        quote: text(quote_quantity),
        limit: text(limit_price),
        trigger: text(trigger_price),
        protection: text(protection_price),
    };

    let expires_in_the_future = match &payload.expire_time {
        None => true,
        Some(expire) => {
            let (usable, in_the_future) = sqlx::query_as::<_, (bool, bool)>(
                "SELECT isfinite(t) AND t < '10000-01-01'::timestamptz, t > now() \
                 FROM (SELECT $1::text::timestamptz AS t) x",
            )
            .bind(expire)
            .fetch_one(&state.pg_connections)
            .await
            .map_err(bad_text_or_500)?;
            if !usable {
                return Err(StatusCode::BAD_REQUEST);
            }
            in_the_future
        }
    };

    let found = find_order_by_key(
        &state.pg_connections,
        user_id,
        market.market_id,
        &payload,
        &texts,
    )
    .await?;
    let (order_id, fresh) = match found {
        Some((existing, true)) => (existing, false),
        Some((_, false)) => return Err(StatusCode::CONFLICT),
        None => {
            if market.status != MarketStatus::Trading {
                return Err(StatusCode::CONFLICT);
            }
            if !base.enabled || !quote.enabled {
                return Err(StatusCode::BAD_REQUEST);
            }
            check_order_account(
                &state.pg_connections,
                payload.base_account_id,
                user_id,
                market.base_ledger_id,
            )
            .await?;
            check_order_account(
                &state.pg_connections,
                payload.quote_account_id,
                user_id,
                market.quote_ledger_id,
            )
            .await?;

            let quote_value = quote_quantity.or_else(|| {
                limit_price
                    .zip(base_quantity)
                    .and_then(|(p, b)| quote_for(p, b, price_dec, base_dec, quote_dec))
            });
            let prices: Vec<u128> = [limit_price, trigger_price, protection_price]
                .into_iter()
                .flatten()
                .collect();
            check_market_rules(&market, base_quantity, quote_value, &prices)?;

            let (maker_fee_bps, taker_fee_bps) = (market.maker_fee_bps, market.taker_fee_bps);
            if maker_fee_bps < 0 || taker_fee_bps < 0 {
                println!("HANDLER: market {} has a negative fee rate", market.symbol);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            let reserved = reservation_amount(
                payload.side,
                payload.order_type,
                base_quantity,
                quote_quantity,
                limit_price,
                (price_dec, base_dec, quote_dec),
                (maker_fee_bps, taker_fee_bps),
            )?;
            // the order is read back with a Decimal, which holds less than the u128 the column does
            if Decimal::from_u128(reserved).is_none() {
                return Err(StatusCode::BAD_REQUEST);
            }
            if !expires_in_the_future {
                return Err(StatusCode::BAD_REQUEST);
            }

            // the order and its outbox row are saved together, so the relay always knows about an order
            let order_id = Uuid::from_u128(tb::id());
            let command_id = Uuid::from_u128(tb::id());
            let mut tx = state
                .pg_connections
                .begin()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let inserted = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO orders (order_id, idempotency_key, command_id, user_id, market_id, \
                 base_account_id, quote_account_id, side, order_type, time_in_force, base_quantity, \
                 quote_quantity, limit_price, trigger_price, trigger_direction, protection_price, \
                 expire_time, maker_fee_bps, taker_fee_bps, reserved_amount) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::numeric, $12::numeric, \
                 $13::numeric, $14::numeric, $15, $16::numeric, $17::text::timestamptz, $18, $19, \
                 $20::numeric) \
                 ON CONFLICT (idempotency_key) DO NOTHING RETURNING order_id",
            )
            .bind(order_id)
            .bind(payload.idempotency_key)
            .bind(command_id)
            .bind(user_id)
            .bind(market.market_id)
            .bind(payload.base_account_id)
            .bind(payload.quote_account_id)
            .bind(payload.side)
            .bind(payload.order_type)
            .bind(payload.time_in_force)
            .bind(&texts.base)
            .bind(&texts.quote)
            .bind(&texts.limit)
            .bind(&texts.trigger)
            .bind(payload.trigger_direction)
            .bind(&texts.protection)
            .bind(&payload.expire_time)
            .bind(maker_fee_bps)
            .bind(taker_fee_bps)
            .bind(reserved.to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match inserted {
                Some(order_id) => {
                    sqlx::query("INSERT INTO order_outbox (order_id) VALUES ($1)")
                        .bind(order_id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    tx.commit()
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    (order_id, true)
                }
                // another request with this key got in first (they were at the same time)
                None => {
                    drop(tx);
                    let found = find_order_by_key(
                        &state.pg_connections,
                        user_id,
                        market.market_id,
                        &payload,
                        &texts,
                    )
                    .await?;
                    match found {
                        Some((existing, true)) => (existing, false),
                        Some((_, false)) => return Err(StatusCode::CONFLICT),
                        None => return Err(StatusCode::INTERNAL_SERVER_ERROR),
                    }
                }
            }
        }
    };

    let mut engine_answer = None;
    if fresh {
        let advance = advance_order(&state, order_id, HANDLER_ENGINE_ATTEMPTS).await?;
        if advance.finished {
            // if this fails the relay stamps it on its next pass, the order itself is already done
            if let Err(e) = sqlx::query(
                "UPDATE order_outbox SET processed_at = now() \
                 WHERE order_id = $1 AND processed_at IS NULL",
            )
            .bind(order_id)
            .execute(&state.pg_connections)
            .await
            {
                println!("HANDLER: couldn't stamp the outbox row of order {order_id}: {e}");
            }
        }
        engine_answer = advance.answer;
    }

    let Json(mut order) = fetch_order(State(state.clone()), Path(order_id)).await?;
    // what the engine answered goes in the reply only: status and fills are written by the consumer
    if let Some(answer) = engine_answer {
        order.status = answer.status;
        order.cancel_reason = answer.cancel_reason;
        if let Some(filled) = answer.filled_base {
            let raw = i128::try_from(filled).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            order.filled_base_quantity = to_decimal(raw, base.decimals)?;
        }
        if let Some(filled) = answer.filled_quote {
            let raw = i128::try_from(filled).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            order.filled_quote_quantity = to_decimal(raw, quote.decimals)?;
        }
    }

    order_outcome(order.submission, order.reject_reason, fresh).map(|code| (code, Json(order)))
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

    let m = fetch_market_by_id(&state.pg_connections, o.market_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let (base, quote) = fetch_market_ledgers(&state.pg_connections, &m).await?;

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
    let limit = page_limit(query.limit)?;

    let (cursor_time, cursor_id) = parse_cursor_pg(query.cursor)?;

    let (status, open_only) = order_status_filter(query.status);

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
    .map_err(page_query_error)?;

    let next_cursor = next_page_cursor(&mut rows, limit, |o| {
        format!("{}_{}", o.created_at, o.order_id)
    });

    let mut market_ids: Vec<i32> = rows.iter().map(|o| o.market_id).collect();
    market_ids.sort_unstable();
    market_ids.dedup();
    let markets = fetch_markets_by_ids(&state.pg_connections, &market_ids)
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
            let (base, quote) = find_market_ledgers(&ledgers, m)?;
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
    let limit = page_limit(query.limit)?;

    let (cursor_time, cursor_id) = parse_cursor_pg(query.cursor)?;

    let (status, open_only) = order_status_filter(query.status);

    let m = fetch_market_by_symbol(&state.pg_connections, &symbol)
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
    .map_err(page_query_error)?;

    let next_cursor = next_page_cursor(&mut rows, limit, |o| {
        format!("{}_{}", o.created_at, o.order_id)
    });

    let (base, quote) = fetch_market_ledgers(&state.pg_connections, &m).await?;

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

    let m = fetch_market_by_id(&state.pg_connections, market_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let (base, quote) = fetch_market_ledgers(&state.pg_connections, &m).await?;

    let rows = sqlx::query_as::<_, TradeRow>(
        r#"SELECT trade_id, market_id, buyer_order_id, seller_order_id, price, base_quantity,
                  quote_quantity, buyer_fee, seller_fee, taker_side,
                  to_char(engine_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS engine_time,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS created_at
           FROM trades
           WHERE buyer_order_id = $1 OR seller_order_id = $1
           ORDER BY trades.engine_time ASC, trades.trade_id ASC"#,
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

// gives the trades of a market (by symbol), newest first
// returns the page + next cursor
pub async fn list_market_trades(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(query): Query<MarketTradeQuery>,
) -> Result<Json<TradePage>, StatusCode> {
    let limit = page_limit(query.limit)?;

    let (cursor_time, cursor_id) = parse_cursor_pg(query.cursor)?;

    let m = fetch_market_by_symbol(&state.pg_connections, &symbol)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut rows = sqlx::query_as::<_, TradeRow>(
        r#"SELECT trade_id, market_id, buyer_order_id, seller_order_id, price, base_quantity,
                  quote_quantity, buyer_fee, seller_fee, taker_side,
                  to_char(engine_time AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS engine_time,
                  to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                      AS created_at
           FROM trades
           WHERE market_id = $1
             AND ($2::text IS NULL
                  OR (engine_time, trade_id) < ($2::text::timestamptz, $3::uuid))
           ORDER BY trades.engine_time DESC, trades.trade_id DESC
           LIMIT $4"#,
    )
    .bind(m.market_id)
    .bind(cursor_time)
    .bind(cursor_id)
    .bind(i64::from(limit) + 1)
    .fetch_all(&state.pg_connections)
    .await
    .map_err(page_query_error)?;

    let next_cursor = next_page_cursor(&mut rows, limit, |t| {
        format!("{}_{}", t.engine_time, t.trade_id)
    });

    let (base, quote) = fetch_market_ledgers(&state.pg_connections, &m).await?;

    let items = rows
        .into_iter()
        .map(|t| Trade::from_row(t, &m, &base, &quote))
        .collect::<Result<Vec<_>, StatusCode>>()?;

    Ok(Json(TradePage { items, next_cursor }))
}

// 24 hour summary of a market (by symbol), computed from its trades
pub async fn fetch_market_ticker(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<Ticker>, StatusCode> {
    let m = fetch_market_by_symbol(&state.pg_connections, &symbol)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let t = sqlx::query_as::<_, TickerStats>(
        r#"SELECT
               (SELECT price FROM trades WHERE market_id = $1
                ORDER BY engine_time DESC, trade_id DESC LIMIT 1) AS last_price,
               w.open, w.high, w.low, w.base_volume, w.quote_volume, w.trade_count
           FROM (
               SELECT (array_agg(price ORDER BY engine_time ASC, trade_id ASC))[1] AS open,
                      max(price) AS high,
                      min(price) AS low,
                      COALESCE(sum(base_quantity), 0) AS base_volume,
                      COALESCE(sum(quote_quantity), 0) AS quote_volume,
                      count(*) AS trade_count
               FROM trades
               WHERE market_id = $1 AND engine_time >= now() - interval '24 hours'
           ) w"#,
    )
    .bind(m.market_id)
    .fetch_one(&state.pg_connections)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (base, quote) = fetch_market_ledgers(&state.pg_connections, &m).await?;

    Ok(Json(Ticker::from_row(t, &m, &base, &quote)?))
}

#[cfg(test)]
mod order_tests {
    use super::*;

    fn dec(mantissa: i64, scale: u32) -> Decimal {
        Decimal::new(mantissa, scale)
    }

    // a valid limit buy, each test changes the fields it is about
    fn payload() -> OrderPayload {
        OrderPayload {
            idempotency_key: Uuid::from_u128(1),
            market: "BTC-USD".to_string(),
            base_account_id: Uuid::from_u128(2),
            quote_account_id: Uuid::from_u128(3),
            side: Side::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Gtc,
            base_quantity: Some(dec(1, 0)),
            quote_quantity: None,
            limit_price: Some(dec(1, 0)),
            trigger_price: None,
            trigger_direction: None,
            protection_price: None,
            expire_time: None,
        }
    }

    fn market() -> MarketRow {
        MarketRow {
            market_id: 1,
            symbol: "BTC-USD".to_string(),
            base_ledger_id: 1,
            quote_ledger_id: 2,
            status: crate::domain::MarketStatus::Trading,
            price_decimals: 2,
            min_base_quantity: Decimal::from(10),
            min_quote_quantity: Decimal::from(100),
            quantity_step: Decimal::from(5),
            price_step: Decimal::from(10),
            maker_fee_bps: 10,
            taker_fee_bps: 30,
        }
    }

    const BAD: Result<(), StatusCode> = Err(StatusCode::BAD_REQUEST);

    // --- units ---------------------------------------------------------------------------

    #[test]
    fn to_units_scales_by_the_decimals() {
        assert_eq!(to_units(dec(15, 1), 2), Ok(150)); // 1.5 with 2 decimals
        assert_eq!(to_units(dec(1, 2), 2), Ok(1)); // 0.01
        assert_eq!(to_units(dec(7, 0), 0), Ok(7));
    }

    #[test]
    fn to_units_rejects_fractions_zero_negative_and_overflow() {
        assert_eq!(to_units(dec(1505, 3), 2), Err(StatusCode::BAD_REQUEST)); // 1.505 needs 3 decimals
        assert_eq!(to_units(Decimal::ZERO, 2), Err(StatusCode::BAD_REQUEST));
        assert_eq!(to_units(dec(-1, 0), 2), Err(StatusCode::BAD_REQUEST));
        assert_eq!(to_units(Decimal::MAX, 18), Err(StatusCode::BAD_REQUEST));
        assert_eq!(to_units(dec(1, 0), 40), Err(StatusCode::BAD_REQUEST));
    }

    // --- wide math -----------------------------------------------------------------------

    #[test]
    fn quote_for_matches_the_contract_formula() {
        // price 1000 and base 1000, all with 2 decimals: 1000 * 1000 / 10^(2+2-2)
        assert_eq!(quote_for(1000, 1000, 2, 2, 2), Some(10_000));
        // always down
        assert_eq!(quote_for(1, 1, 2, 2, 2), Some(0));
        assert_eq!(quote_for(7, 15, 1, 0, 0), Some(10)); // 105 / 10
        // the product needs more than 128 bits, the result doesn't
        assert_eq!(quote_for(u128::MAX, 100, 2, 2, 2), Some(u128::MAX));
    }

    #[test]
    fn quote_for_rejects_what_cannot_be_computed() {
        assert_eq!(quote_for(1, 1, 0, 0, 1), None); // quote has more decimals than price + base
        assert_eq!(quote_for(1, 1, 30, 10, 0), None); // 10^40 doesn't fit a u128
        assert_eq!(quote_for(u128::MAX, u128::MAX, 2, 2, 2), None); // the result doesn't fit
    }

    // --- reservation ---------------------------------------------------------------------

    const SCALE: (u32, u32, u32) = (2, 2, 2);

    #[test]
    fn buy_reservation_adds_rounded_up_fee_of_the_higher_rate() {
        // market buy: the quote quantity is the amount. 1_000_000 + ceil(1_000_000 * 30 / 10_000)
        let r = reservation_amount(
            Side::Buy,
            OrderType::Market,
            None,
            Some(1_000_000),
            None,
            SCALE,
            (10, 30),
        );
        assert_eq!(r, Ok(1_003_000));
        // the higher rate wins whichever side it is on
        let r = reservation_amount(
            Side::Buy,
            OrderType::Market,
            None,
            Some(1_000_000),
            None,
            SCALE,
            (30, 10),
        );
        assert_eq!(r, Ok(1_003_000));
        // the fee rounds up
        let r = reservation_amount(
            Side::Buy,
            OrderType::Market,
            None,
            Some(1),
            None,
            SCALE,
            (1, 1),
        );
        assert_eq!(r, Ok(2));
    }

    #[test]
    fn limit_buy_reserves_the_quote_value_at_the_limit_price_plus_fee() {
        // 1000 base at limit 1000 is worth 10_000 quote, fee ceil(10_000 * 30 / 10_000) = 30
        let r = reservation_amount(
            Side::Buy,
            OrderType::Limit,
            Some(1000),
            None,
            Some(1000),
            SCALE,
            (10, 30),
        );
        assert_eq!(r, Ok(10_030));
    }

    #[test]
    fn sell_reservation_is_the_base_quantity_without_fee() {
        for t in [OrderType::Limit, OrderType::Market] {
            let r =
                reservation_amount(Side::Sell, t, Some(5000), None, Some(1000), SCALE, (10, 30));
            assert_eq!(r, Ok(5000));
        }
    }

    #[test]
    fn reservation_overflow_or_zero_value_is_an_error() {
        let bad = Err(StatusCode::BAD_REQUEST);
        let r = reservation_amount(
            Side::Buy,
            OrderType::Market,
            None,
            Some(u128::MAX),
            None,
            SCALE,
            (10, 30),
        );
        assert_eq!(r, bad);
        // 1 base at price 1 is worth 0.0001 of the smallest quote unit: it rounds down to 0
        let r = reservation_amount(
            Side::Buy,
            OrderType::Limit,
            Some(1),
            None,
            Some(1),
            SCALE,
            (10, 30),
        );
        assert_eq!(r, bad);
    }

    // --- shape ---------------------------------------------------------------------------

    #[test]
    fn shape_valid_orders_pass() {
        assert_eq!(validate_shape(&payload()), Ok(()));

        let mut market_buy = payload();
        market_buy.order_type = OrderType::Market;
        market_buy.base_quantity = None;
        market_buy.quote_quantity = Some(dec(5, 0));
        market_buy.limit_price = None;
        market_buy.protection_price = Some(dec(9, 0));
        assert_eq!(validate_shape(&market_buy), Ok(()));

        let mut market_sell = market_buy;
        market_sell.side = Side::Sell;
        market_sell.base_quantity = Some(dec(5, 0));
        market_sell.quote_quantity = None;
        assert_eq!(validate_shape(&market_sell), Ok(()));

        let mut stop_limit = payload();
        stop_limit.order_type = OrderType::StopLimit;
        stop_limit.trigger_price = Some(dec(2, 0));
        stop_limit.trigger_direction = Some(TriggerDirection::LastAtOrAbove);
        assert_eq!(validate_shape(&stop_limit), Ok(()));

        let mut gtd = payload();
        gtd.time_in_force = TimeInForce::Gtd;
        gtd.expire_time = Some("2999-01-01T00:00:00Z".to_string());
        assert_eq!(validate_shape(&gtd), Ok(()));
    }

    #[test]
    fn shape_market_buy_needs_the_quote_arm_and_nothing_else_does() {
        let mut p = payload();
        p.order_type = OrderType::Market;
        p.limit_price = None;
        // a market buy with a base quantity
        assert_eq!(validate_shape(&p), BAD);
        // a limit buy with a quote quantity instead of a base one
        let mut p = payload();
        p.base_quantity = None;
        p.quote_quantity = Some(dec(5, 0));
        assert_eq!(validate_shape(&p), BAD);
        // both at once
        let mut p = payload();
        p.quote_quantity = Some(dec(5, 0));
        assert_eq!(validate_shape(&p), BAD);
        // a sell with a quote quantity
        let mut p = payload();
        p.side = Side::Sell;
        p.base_quantity = None;
        p.quote_quantity = Some(dec(5, 0));
        assert_eq!(validate_shape(&p), BAD);
    }

    #[test]
    fn shape_limit_prices_only_on_limit_orders() {
        let mut p = payload();
        p.limit_price = None;
        assert_eq!(validate_shape(&p), BAD); // limit without a limit price
        let mut p = payload();
        p.order_type = OrderType::Market;
        p.base_quantity = None;
        p.quote_quantity = Some(dec(5, 0));
        assert_eq!(validate_shape(&p), BAD); // market with a limit price
    }

    #[test]
    fn shape_stop_orders_need_trigger_price_and_direction_together() {
        let mut p = payload();
        p.order_type = OrderType::StopLimit;
        assert_eq!(validate_shape(&p), BAD); // neither
        p.trigger_price = Some(dec(2, 0));
        assert_eq!(validate_shape(&p), BAD); // no direction
        p.trigger_price = None;
        p.trigger_direction = Some(TriggerDirection::LastAtOrBelow);
        assert_eq!(validate_shape(&p), BAD); // no price
        let mut p = payload();
        p.trigger_price = Some(dec(2, 0));
        p.trigger_direction = Some(TriggerDirection::LastAtOrBelow);
        assert_eq!(validate_shape(&p), BAD); // a trigger on a plain limit order
    }

    #[test]
    fn shape_expiry_goes_with_gtd_and_only_with_gtd() {
        let mut p = payload();
        p.time_in_force = TimeInForce::Gtd;
        assert_eq!(validate_shape(&p), BAD);
        let mut p = payload();
        p.expire_time = Some("2999-01-01T00:00:00Z".to_string());
        assert_eq!(validate_shape(&p), BAD);
    }

    #[test]
    fn shape_protection_price_is_not_for_limit_orders() {
        let mut p = payload();
        p.protection_price = Some(dec(2, 0));
        assert_eq!(validate_shape(&p), BAD);
        p.order_type = OrderType::StopLimit;
        p.trigger_price = Some(dec(2, 0));
        p.trigger_direction = Some(TriggerDirection::LastAtOrAbove);
        assert_eq!(validate_shape(&p), BAD);
    }

    // --- market rules --------------------------------------------------------------------

    #[test]
    fn market_rules_accept_a_conforming_order() {
        assert_eq!(
            check_market_rules(&market(), Some(10), Some(100), &[10, 20]),
            Ok(())
        );
    }

    #[test]
    fn market_rules_reject_below_the_minimums() {
        let m = market();
        assert_eq!(check_market_rules(&m, Some(5), Some(100), &[]), BAD);
        assert_eq!(check_market_rules(&m, Some(10), Some(99), &[]), BAD);
    }

    #[test]
    fn market_rules_reject_off_step_quantities_and_prices() {
        let m = market();
        assert_eq!(check_market_rules(&m, Some(12), Some(100), &[]), BAD); // step 5
        assert_eq!(check_market_rules(&m, Some(10), Some(100), &[15]), BAD); // step 10
    }

    #[test]
    fn market_rules_only_check_the_quantity_the_request_states() {
        let m = market();
        // market buy: only a quote quantity
        assert_eq!(check_market_rules(&m, None, Some(100), &[]), Ok(()));
        // market sell: only a base quantity
        assert_eq!(check_market_rules(&m, Some(10), None, &[]), Ok(()));
    }

    #[test]
    fn market_rules_never_divide_by_a_zero_step() {
        let mut m = market();
        m.quantity_step = Decimal::ZERO;
        assert!(check_market_rules(&m, Some(10), Some(100), &[10]).is_err());
        let mut m = market();
        m.price_step = Decimal::ZERO;
        assert!(check_market_rules(&m, Some(10), Some(100), &[10]).is_err());
    }

    // --- the HTTP answer for an order ----------------------------------------------------

    #[test]
    fn outcome_of_a_good_order_depends_on_whether_it_is_new() {
        assert_eq!(
            order_outcome(Submission::Accepted, None, true),
            Ok(StatusCode::CREATED)
        );
        assert_eq!(
            order_outcome(Submission::Accepted, None, false),
            Ok(StatusCode::OK)
        );
    }

    #[test]
    fn outcome_of_an_unfinished_order_is_accepted_for_processing() {
        for s in [Submission::Reserving, Submission::Sent] {
            assert_eq!(order_outcome(s, None, true), Ok(StatusCode::ACCEPTED));
            assert_eq!(order_outcome(s, None, false), Ok(StatusCode::ACCEPTED));
        }
    }

    #[test]
    fn outcome_of_a_rejected_order_is_an_error_code_for_the_reason() {
        let rejected = |r| order_outcome(Submission::Rejected, r, true);
        // refused by the backend itself (not enough funds, closed account)
        assert_eq!(rejected(None), Err(StatusCode::CONFLICT));
        assert_eq!(
            rejected(Some(DomainRejectReason::UnknownMarket)),
            Err(StatusCode::NOT_FOUND)
        );
        for r in [
            DomainRejectReason::BadQuantity,
            DomainRejectReason::BadPrice,
            DomainRejectReason::BadOrderParams,
            DomainRejectReason::Unsupported,
        ] {
            assert_eq!(rejected(Some(r)), Err(StatusCode::BAD_REQUEST));
        }
        assert_eq!(
            rejected(Some(DomainRejectReason::InternalError)),
            Err(StatusCode::SERVICE_UNAVAILABLE)
        );
    }

    // --- the PlaceOrder request and its answer -------------------------------------------

    fn place_row() -> PlaceRow {
        PlaceRow {
            order_id: Uuid::from_u128(10),
            command_id: Uuid::from_u128(11),
            user_id: Uuid::from_u128(12),
            market_symbol: "BTC-USD".to_string(),
            base_account_id: Uuid::from_u128(13),
            quote_account_id: Uuid::from_u128(14),
            side: Side::Buy,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::Gtc,
            base_quantity: Some(1000),
            quote_quantity: None,
            limit_price: Some(1000),
            trigger_price: None,
            trigger_direction: None,
            protection_price: None,
            expire_micros: None,
            created_micros: 1_700_000_000_123_456,
            maker_fee_bps: 10,
            taker_fee_bps: 30,
        }
    }

    #[test]
    fn request_built_twice_from_one_row_is_equal() {
        let row = place_row();
        assert_eq!(build_place_request(&row), build_place_request(&row));
    }

    #[test]
    fn request_carries_ids_accounts_and_the_stored_fee_rates() {
        let r = build_place_request(&place_row());
        assert_eq!(r.command_id, Some(proto::Id::from(11)));
        assert_eq!(r.order_id, Some(proto::Id::from(10)));
        assert_eq!(r.account_holder, Some(proto::Id::from(12)));
        assert_eq!(r.base_account, Some(proto::Id::from(13)));
        assert_eq!(r.quote_account, Some(proto::Id::from(14)));
        assert_eq!(r.market_symbol, "BTC-USD");
        assert_eq!(r.side, proto::Side::Buy as i32);
        assert_eq!(r.r#type, proto::OrderType::Limit as i32);
        assert_eq!(r.time_in_force, proto::TimeInForce::Gtc as i32);
        assert_eq!(r.limit_price, Some(proto::Uint128::from(1000)));
        assert_eq!((r.maker_fee_bps, r.taker_fee_bps), (10, 30));
    }

    #[test]
    fn request_uses_quote_arm_only_for_market_buys() {
        use proto::place_order_request::Amount;
        let limit_buy = build_place_request(&place_row());
        assert_eq!(limit_buy.amount, Some(Amount::BaseQuantity(1000.into())));

        let mut market_buy = place_row();
        market_buy.order_type = OrderType::Market;
        market_buy.base_quantity = None;
        market_buy.quote_quantity = Some(5000);
        market_buy.limit_price = None;
        let r = build_place_request(&market_buy);
        assert_eq!(r.amount, Some(Amount::QuoteQuantity(5000.into())));
        assert_eq!(r.limit_price, None);

        let mut market_sell = place_row();
        market_sell.side = Side::Sell;
        market_sell.order_type = OrderType::Market;
        market_sell.limit_price = None;
        let r = build_place_request(&market_sell);
        assert_eq!(r.amount, Some(Amount::BaseQuantity(1000.into())));
    }

    #[test]
    fn request_sent_at_is_the_row_creation_time_and_expiry_comes_from_the_row() {
        let mut row = place_row();
        row.expire_micros = Some(1_800_000_000_000_001);
        let r = build_place_request(&row);
        let sent = r.sent_at.expect("sent_at");
        assert_eq!((sent.seconds, sent.nanos), (1_700_000_000, 123_456_000));
        let expire = r.expire_time.expect("expire_time");
        assert_eq!((expire.seconds, expire.nanos), (1_800_000_000, 1_000));
        assert_eq!(build_place_request(&place_row()).expire_time, None);
    }

    #[test]
    fn request_stop_orders_carry_trigger_price_and_direction() {
        let mut row = place_row();
        row.order_type = OrderType::StopLimit;
        row.trigger_price = Some(900);
        row.trigger_direction = Some(TriggerDirection::LastAtOrBelow);
        let r = build_place_request(&row);
        assert_eq!(r.trigger_price, Some(proto::Uint128::from(900)));
        assert_eq!(
            r.trigger_direction,
            proto::TriggerDirection::LastAtOrBelow as i32
        );
    }

    fn response(reject_reason: i32) -> proto::PlaceOrderResponse {
        proto::PlaceOrderResponse {
            reject_reason,
            ..Default::default()
        }
    }

    #[tokio::test(start_paused = true)]
    async fn classify_timeout_and_every_status_is_retry() {
        let elapsed = tokio::time::timeout(
            std::time::Duration::from_millis(1),
            std::future::pending::<()>(),
        )
        .await
        .unwrap_err();
        assert_eq!(classify_place(Err(elapsed)), PlaceVerdict::Retry);
        for code in [
            tonic::Code::Unavailable,
            tonic::Code::DeadlineExceeded,
            tonic::Code::Internal,
            tonic::Code::Unimplemented,
            tonic::Code::InvalidArgument,
        ] {
            let verdict = classify_place(Ok(Err(tonic::Status::new(code, "x"))));
            assert_eq!(verdict, PlaceVerdict::Retry, "{code:?}");
        }
    }

    #[test]
    fn classify_zero_reject_is_accepted_with_amounts() {
        let mut r = response(0);
        r.status = proto::OrderStatus::Cancelled as i32;
        r.cancel_reason = proto::CancelReason::IocNoFill as i32;
        r.filled_base_quantity = Some(5u128.into());
        r.filled_quote_quantity = Some(7u128.into());
        assert_eq!(
            classify_place(Ok(Ok(r))),
            PlaceVerdict::Accepted(EngineAnswer {
                status: Some(OrderStatus::Cancelled),
                cancel_reason: Some(CancelReason::IocNoFill),
                filled_base: Some(5),
                filled_quote: Some(7),
            })
        );
        // an accepted order with nothing else in the answer: absent amounts stay absent
        assert_eq!(
            classify_place(Ok(Ok(response(0)))),
            PlaceVerdict::Accepted(EngineAnswer {
                status: None,
                cancel_reason: None,
                filled_base: None,
                filled_quote: None,
            })
        );
    }

    #[test]
    fn classify_unknown_reject_number_is_anomaly_not_accepted() {
        for n in [99, 11, -1] {
            assert_eq!(
                classify_place(Ok(Ok(response(n)))),
                PlaceVerdict::Anomaly,
                "{n}"
            );
        }
    }

    #[test]
    fn classify_duplicate_command_is_anomaly() {
        assert_eq!(classify_place(Ok(Ok(response(8)))), PlaceVerdict::Anomaly);
    }

    #[test]
    fn classify_internal_error_and_unknown_market_are_rejected() {
        assert_eq!(
            classify_place(Ok(Ok(response(10)))),
            PlaceVerdict::Rejected(DomainRejectReason::InternalError)
        );
        assert_eq!(
            classify_place(Ok(Ok(response(1)))),
            PlaceVerdict::Rejected(DomainRejectReason::UnknownMarket)
        );
    }
}
