/// The id of an incoming order, given by the backend.
///
/// It is unique per resting order. The [`Book`](crate::book::Book) rejects a duplicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(u128);

impl OrderId {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u128 {
        self.0
    }
}

/// The id of a trade, created by the engine.
///
/// When the book matches resting orders, this id gives the engine a log of each match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeId(u128);

impl TradeId {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u128 {
        self.0
    }
}

/// The id of a user, sent by the backend. It is needed for self-trade prevention.
///
/// It is a `u128` because the backend's UUIDs are 128 bits; a `u64` would truncate one.
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

/// The unique id of a command. The backend sends it with every request, so that the
/// same command isn't run twice.
///
/// It used to be a string, but the contract makes it a `u128`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(u128);
impl CommandId {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u128 {
        self.0
    }
}
