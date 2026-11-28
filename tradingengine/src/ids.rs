use proto::Id;
use redis::{RedisWrite, ToRedisArgs, ToSingleRedisArg};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The id of an incoming order, given by the backend.
///
/// It is unique per resting order. The [`Book`](crate::book::Book) rejects a duplicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(u128);

impl ToRedisArgs for OrderId {
    fn write_redis_args<W: ?Sized + RedisWrite>(&self, out: &mut W) {
        self.0.write_redis_args(out);
    }
}
impl ToSingleRedisArg for OrderId {}

impl From<OrderId> for Id {
    fn from(id: OrderId) -> Self {
        Id::from(id.value())
    }
}
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

impl From<TradeId> for Id {
    fn from(id: TradeId) -> Self {
        Id::from(id.value())
    }
}
impl TradeId {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// A new UUIDv7, as the contract asks for `trade_id`.
    ///
    /// Each one is bigger than the one before it in this process, even within the same
    /// millisecond, and its random bits keep two engine instances from colliding.
    pub fn generate() -> Self {
        Self(Uuid::now_v7().as_u128())
    }
    pub const fn value(self) -> u128 {
        self.0
    }
}

/// The id of a user, sent by the backend. It is needed for self-trade prevention.
///
/// It is a `u128` because the backend's UUIDs are 128 bits; a `u64` would truncate one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(u128);

impl From<UserId> for Id {
    fn from(id: UserId) -> Self {
        Id::from(id.value())
    }
}
impl UserId {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u128 {
        self.0
    }
}

/// The id of a TigerBeetle account, sent by the backend. The engine only stores it and
/// sends it back in the `TradeSettlement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(u128);

impl From<AccountId> for Id {
    fn from(id: AccountId) -> Self {
        Id::from(id.value())
    }
}
impl AccountId {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(u128);

impl From<Id> for CommandId {
    fn from(id: Id) -> Self {
        Self(u128::from(id))
    }
}
impl ToRedisArgs for CommandId {
    fn write_redis_args<W: ?Sized + RedisWrite>(&self, out: &mut W) {
        self.0.write_redis_args(out);
    }
}
impl ToSingleRedisArg for CommandId {}

impl From<CommandId> for Id {
    fn from(id: CommandId) -> Self {
        Id::from(id.value())
    }
}
impl CommandId {
    pub const fn new(value: u128) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u128 {
        self.0
    }
}
