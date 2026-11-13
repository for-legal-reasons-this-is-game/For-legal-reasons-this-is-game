//! Saves the engine's state to Redis, and loads it back when the engine starts.
//!
//! Each engine instance keeps its state under its own name (`ENGINE_INSTANCE`, for example the
//! StatefulSet pod name), so two instances never write the same keys:
//!
//! | Key | Type | Holds |
//! |---|---|---|
//! | `engine:{instance}:markets` | hash | each market's definition, by symbol |
//! | `engine:{instance}:book:{symbol}` | string | the orders resting in the market's book |
//! | `engine:{instance}:ended:{symbol}` | list | the market's ended orders, oldest first |
//!
//! The values are JSON, in the format of [`crate::state`]. Once the engine has a command
//! cache, an event history and an event `sequence`, they are saved here too.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::time::Duration;

use redis::aio::ConnectionManager;

use crate::engine::Engine;
use crate::ids::OrderId;
use crate::state::{self, StateError};

/// How many times a save is tried before the engine gives up on Redis.
const SAVE_ATTEMPTS: u32 = 3;

/// How long to wait before trying a failed save again.
const SAVE_RETRY_DELAY: Duration = Duration::from_millis(200);

/// Why the state couldn't be saved or loaded.
#[derive(Debug)]
pub enum StoreError {
    /// Redis couldn't be reached, or refused a command.
    Redis(redis::RedisError),
    /// What Redis holds isn't a state the engine can load.
    State(StateError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Redis(error) => write!(f, "redis: {error}"),
            Self::State(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<redis::RedisError> for StoreError {
    fn from(error: redis::RedisError) -> Self {
        Self::Redis(error)
    }
}

impl From<StateError> for StoreError {
    fn from(error: StateError) -> Self {
        Self::State(error)
    }
}

/// Where one engine instance keeps its state in Redis.
pub struct RedisStore {
    conn: ConnectionManager,
    /// `engine:{instance}`, the start of every key.
    prefix: String,
    /// For each market, the newest of its ended orders that is already in Redis, so that a
    /// save only adds the orders that ended after it.
    saved_ended: HashMap<String, Option<OrderId>>,
}

impl RedisStore {
    /// Connects to the Redis at `url`, to the state of the engine instance `instance`.
    pub async fn connect(url: &str, instance: &str) -> Result<Self, StoreError> {
        let client = redis::Client::open(url)?;
        let mut conn = ConnectionManager::new(client).await?;
        let _pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(Self {
            conn,
            prefix: format!("engine:{instance}"),
            saved_ended: HashMap::new(),
        })
    }

    /// Loads the saved state. An instance that never saved anything starts with no markets.
    pub async fn load(&mut self) -> Result<Engine, StoreError> {
        let definitions: BTreeMap<String, String> = redis::cmd("HGETALL")
            .arg(self.markets_key())
            .query_async(&mut self.conn)
            .await?;

        let mut markets = Vec::with_capacity(definitions.len());
        for (symbol, definition) in definitions {
            let book: Option<String> = redis::cmd("GET")
                .arg(self.book_key(&symbol))
                .query_async(&mut self.conn)
                .await?;
            let ended: Vec<String> = redis::cmd("LRANGE")
                .arg(self.ended_key(&symbol))
                .arg(0)
                .arg(-1)
                .query_async(&mut self.conn)
                .await?;

            let market = state::restore_market(&definition, book.as_deref(), &ended)?;
            if market.market().symbol != symbol {
                return Err(StateError::Invalid(format!(
                    "the market saved as {symbol} says it is {}",
                    market.market().symbol
                ))
                .into());
            }
            self.saved_ended.insert(symbol, market.ended().newest());
            markets.push(market);
        }
        Ok(Engine::restore(markets))
    }

    /// Saves everything a command can have changed in the market `symbol`: its definition,
    /// its book, and the orders that ended since the last save. All of it is written at once,
    /// or none of it.
    ///
    /// A save that fails is tried again a few times. If it still fails, the error is returned.
    pub async fn save_market(&mut self, engine: &Engine, symbol: &str) -> Result<(), StoreError> {
        let Some(market) = engine.market(symbol) else {
            return Ok(());
        };
        let definition = state::market_json(market.market())?;
        let book = state::book_json(market)?;
        let saved_up_to = self.saved_ended.get(symbol).copied().flatten();
        let ended = market
            .ended()
            .after(saved_up_to)
            .map(|(id, ended)| state::ended_json(id, ended))
            .collect::<Result<Vec<_>, _>>()?;

        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("HSET")
            .arg(self.markets_key())
            .arg(symbol)
            .arg(definition)
            .ignore()
            .cmd("SET")
            .arg(self.book_key(symbol))
            .arg(book)
            .ignore();
        if !ended.is_empty() {
            // the list keeps as many ended orders as the market remembers
            let ended_key = self.ended_key(symbol);
            let keep = i64::try_from(market.ended().capacity()).unwrap_or(i64::MAX);
            pipe.cmd("RPUSH")
                .arg(&ended_key)
                .arg(&ended)
                .ignore()
                .cmd("LTRIM")
                .arg(&ended_key)
                .arg(-keep)
                .arg(-1)
                .ignore();
        }

        let mut attempt = 1;
        loop {
            match pipe.query_async::<()>(&mut self.conn).await {
                Ok(()) => break,
                Err(error) if attempt < SAVE_ATTEMPTS => {
                    eprintln!(
                        "ENGINE: saving market {symbol} to redis failed, trying again: {error}"
                    );
                    attempt += 1;
                    tokio::time::sleep(SAVE_RETRY_DELAY).await;
                }
                Err(error) => return Err(error.into()),
            }
        }
        self.saved_ended
            .insert(symbol.to_string(), market.ended().newest());
        Ok(())
    }

    fn markets_key(&self) -> String {
        format!("{}:markets", self.prefix)
    }

    fn book_key(&self, symbol: &str) -> String {
        format!("{}:book:{symbol}", self.prefix)
    }

    fn ended_key(&self, symbol: &str) -> String {
        format!("{}:ended:{symbol}", self.prefix)
    }
}
