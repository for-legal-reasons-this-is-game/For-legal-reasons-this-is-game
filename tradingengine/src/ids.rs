use crate::error::{EngineError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(u64);

impl OrderId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeId(u64);

impl TradeId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeqNo(u64);

impl SeqNo {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoinId(u32);

impl CoinId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u32 {
        self.0
    }
}

// u128 because the backend's UUIDs are 128 bits; a u64 would truncate one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(u128);

impl UserId {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u128 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    // Fits a 36-character UUID. Capped because the engine keeps every key it has
    // seen, so an unbounded key is an allocation the client gets to choose.
    pub const MAX_LEN: usize = 64;

    pub fn new(value: String) -> Result<Self> {
        // len() is bytes, not characters. Safe to compare against MAX_LEN only
        // because the charset rule below rejects everything multi-byte.
        let length_ok = (1..=Self::MAX_LEN).contains(&value.len());
        let charset_ok = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');

        if length_ok && charset_ok {
            Ok(Self(value))
        } else {
            Err(EngineError::IdempotencyKeyInvalid)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
