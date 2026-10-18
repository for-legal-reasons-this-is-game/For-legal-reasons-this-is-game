use cn_tigerbeetle as tb;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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

#[derive(Serialize, FromRow, TS)]
#[ts(export)]
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
// #[serde(try_from = "i16", into = "i16")]
#[ts(export)]
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
