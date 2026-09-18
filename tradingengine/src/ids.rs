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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdempotencyKey {
    bytes: [u8; Self::MAX_LEN],
    len: u8,
}

impl IdempotencyKey {
    // Fits a 36-character UUID.
    pub const MAX_LEN: usize = 64;

    pub fn new(value: &str) -> Result<Self> {
        // len() is bytes, not characters. Safe to compare against MAX_LEN only
        // because the charset rule below rejects everything multi-byte.
        let length_ok = (1..=Self::MAX_LEN).contains(&value.len());
        let charset_ok = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');

        if length_ok && charset_ok {
            let mut bytes = [0u8; Self::MAX_LEN];
            bytes[..value.len()].copy_from_slice(value.as_bytes());
            Ok(Self {
                bytes,
                len: value.len() as u8,
            })
        } else {
            Err(EngineError::IdempotencyKeyInvalid)
        }
    }

    pub fn as_str(&self) -> &str {
        // The constructor only admits ASCII, so this cannot fail.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)])
            .expect("key bytes are validated ASCII at construction")
    }
}

impl std::fmt::Debug for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("IdempotencyKey")
            .field(&self.as_str())
            .finish()
    }
}
