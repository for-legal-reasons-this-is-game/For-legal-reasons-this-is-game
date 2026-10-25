use axum::http::StatusCode;
use cn_tigerbeetle as tb;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
// #[derive(TS)] + #[ts(export)] generate TypeScript types for the frontend
// (`cargo test export_bindings` writes them to bindings/)
use ts_rs::TS;
use uuid::Uuid;

#[derive(Serialize, FromRow, TS)]
#[ts(export)]
pub struct User {
    pub user_id: Uuid,
    pub user_name: String,
}

#[derive(Serialize, TS)]
#[ts(export)]
pub struct Position {
    pub account_id: Uuid,
    pub account_status: AccountStatus,
    pub symbol: String,
    pub decimals: i16,
    #[ts(type = "string")]
    pub debits_posted: Decimal,
    #[ts(type = "string")]
    pub credits_posted: Decimal,
    #[ts(type = "string")]
    pub debits_pending: Decimal,
    #[ts(type = "string")]
    pub credits_pending: Decimal,
    #[ts(type = "string")]
    pub net_posted: Decimal, // credits_posted - debits_posted
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, sqlx::Type, TS)]
#[sqlx(type_name = "account_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AccountStatus {
    Active,
    Processing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, sqlx::Type, TS)]
#[sqlx(type_name = "market_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MarketStatus {
    Trading,
    CancelOnly,
    Halted,
}

#[derive(Serialize, FromRow)]
pub struct Account {
    pub account_id: Uuid,
    pub account_name: String,
    pub account_ledger_id: i32,
    pub account_code_type: AccountCodeType,
    pub account_status: AccountStatus,
    pub account_user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, FromRow, TS)]
#[ts(export)]
pub struct Ledger {
    pub ledger_id: i32,
    pub symbol: String,
    pub name: String,
    pub decimals: i16,
    pub enabled: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct MarketRow {
    pub market_id: i32,
    pub symbol: String,
    pub base_ledger_id: i32,
    pub quote_ledger_id: i32,
    pub status: MarketStatus,
    pub price_decimals: i16,
    pub min_base_quantity: Decimal,
    pub min_quote_quantity: Decimal,
    pub quantity_step: Decimal,
    pub price_step: Decimal,
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
}

// from i128 to Decimal type
pub fn to_decimal(raw: i128, decimals: i16) -> Result<Decimal, StatusCode> {
    Decimal::try_from_i128_with_scale(raw, decimals as u32)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// scales raw decimal to the decimal stored in db
fn scale_to_decimal(raw: Decimal, decimals: i16) -> Result<Decimal, StatusCode> {
    to_decimal(
        raw.to_i128().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        decimals,
    )
}

#[derive(Debug, Clone, Serialize)]
pub struct Market {
    pub market_id: i32,
    pub symbol: String,
    pub status: MarketStatus,
    pub base_asset: Ledger,
    pub quote_asset: Ledger,
    pub price_decimals: i16,
    pub min_base_quantity: Decimal,
    pub min_quote_quantity: Decimal,
    pub quantity_step: Decimal,
    pub price_step: Decimal,
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
}

impl Market {
    // builds the market from the row in the db
    pub fn from_row(m: MarketRow, base: Ledger, quote: Ledger) -> Result<Self, StatusCode> {
        Ok(Market {
            market_id: m.market_id,
            symbol: m.symbol,
            status: m.status,
            price_decimals: m.price_decimals,
            min_base_quantity: scale_to_decimal(m.min_base_quantity, base.decimals)?,
            quantity_step: scale_to_decimal(m.quantity_step, base.decimals)?,
            min_quote_quantity: scale_to_decimal(m.min_quote_quantity, quote.decimals)?,
            price_step: scale_to_decimal(m.price_step, m.price_decimals)?,
            maker_fee_bps: m.maker_fee_bps,
            taker_fee_bps: m.taker_fee_bps,
            base_asset: base,
            quote_asset: quote,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    In,  // money came into this account
    Out, // money left this account
}

#[derive(Serialize)]
pub struct Transfer {
    pub transfer_id: Uuid,
    pub direction: TransferDirection,
    pub kind: &'static str,
    pub amount: Decimal,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct TransferPage {
    pub items: Vec<Transfer>,
    pub next_cursor: Option<String>, // None means this was the last page
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    FromPrimitive,
    ToPrimitive,
    Serialize,
    Deserialize,
    sqlx::Type,
    TS,
)]
#[repr(i16)]
#[serde(try_from = "i16", into = "i16")]
#[ts(export, as = "i16")]
pub enum AccountCodeType {
    User = 1,
    Fee = 2,
    Source = 3,
    Hold = 4, // this will be created for each order to hold the money the order might spend (not
              // currently implemented)
}

impl AccountCodeType {
    pub fn tb_flags(self) -> tb::AccountFlags {
        match self {
            AccountCodeType::Source => tb::AccountFlags::empty(),
            _ => tb::AccountFlags::DebitsMustNotExceedCredits,
        }
    }
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

#[derive(
    Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, Deserialize, sqlx::Type,
)]
#[repr(i16)]
#[serde(try_from = "i16", into = "i16")]
pub enum TransferCodeType {
    Trade = 1,
    FeeCollection = 2,
    ProvisioningFromSource = 3,
    Hold = 4,
    CloseAccount = 5,
}

impl TryFrom<i16> for TransferCodeType {
    type Error = String;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        TransferCodeType::from_i16(value)
            .ok_or_else(|| format!("invalid TransferCodeType{}", value))
    }
}

impl From<TransferCodeType> for u16 {
    fn from(value: TransferCodeType) -> Self {
        value.to_u16().expect("enum variants always fit in u16")
    }
}

impl From<TransferCodeType> for i16 {
    fn from(value: TransferCodeType) -> Self {
        value.to_i16().expect("enum variants always fit in i16")
    }
}

// enums for serializing the proto enums
#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, sqlx::Type, TS)]
#[repr(i16)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Side {
    Buy = 1,
    Sell = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, sqlx::Type, TS)]
#[repr(i16)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrderType {
    Limit = 1,
    Market = 2,
    StopLimit = 3,
    StopMarket = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, sqlx::Type, TS)]
#[repr(i16)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TimeInForce {
    Gtc = 1,
    Gtd = 2,
    Ioc = 3,
    Fok = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, sqlx::Type, TS)]
#[repr(i16)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TriggerDirection {
    LastAtOrAbove = 1,
    LastAtOrBelow = 2,
}

// the engine's view of the order
#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, sqlx::Type, TS)]
#[repr(i16)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrderStatus {
    New = 1,
    PartiallyFilled = 2,
    Filled = 3,
    Cancelled = 4,
    Expired = 5,
    PendingTrigger = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, sqlx::Type, TS)]
#[repr(i16)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CancelReason {
    UserRequest = 1,
    IocRemainder = 2,
    IocNoFill = 3,
    FokNoFullFill = 4,
    SelfTrade = 5,
    ProtectionPrice = 6,
    Admin = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive, ToPrimitive, Serialize, sqlx::Type, TS)]
#[repr(i16)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RejectReason {
    UnknownMarket = 1,
    BadQuantity = 2,
    BadPrice = 3,
    BadOrderParams = 4,
    UnknownOrder = 5,
    OrderNotWorking = 6,
    NotOrderOwner = 7,
    DuplicateCommand = 8,
    Unsupported = 9,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, sqlx::Type, TS)]
#[sqlx(type_name = "order_submission", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Submission {
    Reserving,
    Sent,
    Accepted,
    Rejected,
}

// how an order comes from sql
#[derive(Debug, sqlx::FromRow)]
pub struct OrderRow {
    pub order_id: Uuid,
    pub idempotency_key: Uuid,
    pub user_id: Uuid,
    pub market_id: i32,
    pub base_account_id: Uuid,
    pub quote_account_id: Uuid,
    pub side: Side,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub base_quantity: Option<Decimal>,
    pub quote_quantity: Option<Decimal>,
    pub limit_price: Option<Decimal>,
    pub trigger_price: Option<Decimal>,
    pub trigger_direction: Option<TriggerDirection>,
    pub protection_price: Option<Decimal>,
    pub expire_time: Option<String>,
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
    pub reserved_amount: Decimal,
    pub submission: Submission,
    pub reject_reason: Option<RejectReason>,
    pub reject_detail: Option<String>,
    pub status: Option<OrderStatus>,
    pub cancel_reason: Option<CancelReason>,
    pub filled_base_quantity: Decimal,
    pub filled_quote_quantity: Decimal,
    pub created_at: String,
    pub updated_at: String,
}

// the actual box to return
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Order {
    pub order_id: Uuid,
    pub idempotency_key: Uuid,
    pub user_id: Uuid,
    pub market: String,
    pub base_account_id: Uuid,
    pub quote_account_id: Uuid,

    pub side: Side,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,

    // exactly one of these two is set
    #[ts(type = "string | null")]
    pub base_quantity: Option<Decimal>,
    #[ts(type = "string | null")]
    pub quote_quantity: Option<Decimal>,

    #[ts(type = "string | null")]
    pub limit_price: Option<Decimal>,
    #[ts(type = "string | null")]
    pub trigger_price: Option<Decimal>,
    pub trigger_direction: Option<TriggerDirection>,
    #[ts(type = "string | null")]
    pub protection_price: Option<Decimal>,
    pub expire_time: Option<String>,

    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
    // in the quote asset for a buy, in the base asset for a sell
    #[ts(type = "string")]
    pub reserved_amount: Decimal,
    pub submission: Submission,
    pub reject_reason: Option<RejectReason>,
    pub reject_detail: Option<String>,

    pub status: Option<OrderStatus>,
    pub cancel_reason: Option<CancelReason>,
    #[ts(type = "string")]
    pub filled_base_quantity: Decimal,
    #[ts(type = "string")]
    pub filled_quote_quantity: Decimal,

    pub created_at: String,
    pub updated_at: String,
}

// same as scale_to_decimal, for the columns that can be null
fn scale_option(raw: Option<Decimal>, decimals: i16) -> Result<Option<Decimal>, StatusCode> {
    raw.map(|r| scale_to_decimal(r, decimals)).transpose()
}

impl Order {
    // builds the order from the row in the db
    pub fn from_row(
        o: OrderRow,
        market: &MarketRow,
        base: &Ledger,
        quote: &Ledger,
    ) -> Result<Self, StatusCode> {
        let reserved_decimals = match o.side {
            Side::Buy => quote.decimals,
            Side::Sell => base.decimals,
        };
        Ok(Order {
            order_id: o.order_id,
            idempotency_key: o.idempotency_key,
            user_id: o.user_id,
            market: market.symbol.clone(),
            base_account_id: o.base_account_id,
            quote_account_id: o.quote_account_id,
            side: o.side,
            order_type: o.order_type,
            time_in_force: o.time_in_force,
            base_quantity: scale_option(o.base_quantity, base.decimals)?,
            quote_quantity: scale_option(o.quote_quantity, quote.decimals)?,
            limit_price: scale_option(o.limit_price, market.price_decimals)?,
            trigger_price: scale_option(o.trigger_price, market.price_decimals)?,
            trigger_direction: o.trigger_direction,
            protection_price: scale_option(o.protection_price, market.price_decimals)?,
            expire_time: o.expire_time,
            maker_fee_bps: o.maker_fee_bps,
            taker_fee_bps: o.taker_fee_bps,
            reserved_amount: scale_to_decimal(o.reserved_amount, reserved_decimals)?,
            submission: o.submission,
            reject_reason: o.reject_reason,
            reject_detail: o.reject_detail,
            status: o.status,
            cancel_reason: o.cancel_reason,
            filled_base_quantity: scale_to_decimal(o.filled_base_quantity, base.decimals)?,
            filled_quote_quantity: scale_to_decimal(o.filled_quote_quantity, quote.decimals)?,
            created_at: o.created_at,
            updated_at: o.updated_at,
        })
    }
}
