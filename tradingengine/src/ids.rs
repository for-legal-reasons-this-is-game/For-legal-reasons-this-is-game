// given by backend for incoming orders this id is unique per resting
// order that is opened and rejecter by the book if a duplicate is sent

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

// created by the engine this makes sure after resting orders are matched
// by the book the trading engine has a log of each matched order
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

// u128 because the backend's UUIDs are 128 bits; a u64 would truncate one.
// required for self trade prevention sent by the backend
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

// sent by the backend on every request to prevent duplication of a command
// it used to be string but the contract says a u128 variable so it has been changed
// each command has a unique id
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
