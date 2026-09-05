use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize, FromRow)]
pub struct User {
    pub user_id: Uuid,
    pub user_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, sqlx::Type)]
#[sqlx(type_name = "account_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    Active,
    Processing,
}

#[derive(Serialize, FromRow)]
pub struct Account {
    pub account_id: Uuid,
    pub account_name: String,
    pub account_ledger_type: LedgerType,
    pub account_code_type: AccountCodeType,
    pub account_status: AccountStatus,
    pub account_user_id: Uuid,
}

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive, Serialize, Deserialize, sqlx::Type)]
#[repr(i16)]
#[serde(try_from = "i16", into = "i16")]
pub enum LedgerType {
    Usd = 1,
    Eur = 2,
    Bitcoin = 3, // extensible
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
pub enum AccountCodeType {
    Cash = 1,
    Crypto = 2,
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
