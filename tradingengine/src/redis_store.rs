use std::collections::HashMap;
use std::time::Duration;

use prost::Message;
use proto::{EventItem, Market};
use redis::{
    AsyncCommands,
    aio::{ConnectionManager, ConnectionManagerConfig},
};

use crate::command_cache::{Answer, Command};
use crate::engine::{Change, EVENT_HISTORY_CAPACITY};
use crate::engine_state::{EngineState, MarketState};
use crate::ids::CommandId;
use crate::resting_order::RestingOrder;

/// How long one Redis call may take. A save only writes what one command changed, so it
/// is small. The biggest call is the load at startup, which reads everything at once: a
/// full command cache (100k entries) is around 12 MB.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

/// Where the engine keeps its state in Redis. Every command writes what it changed (see
/// [`Change`]), and a start reads all of it back.
///
/// One engine has one market, so the keys only need the engine's instance.
pub struct RedisStore {
    connection: ConnectionManager,
    /// hash: market symbol -> [`Market`] (protobuf)
    markets_key: String,
    // TODO when we have more markets per engine, this will need to say which market it belongs to
    /// hash: order id -> [`RestingOrder`] (msgpack)
    orders_key: String,
    /// list: order ids, in the order the orders joined the back of their price level, so
    /// a load puts every order back in its place in the queue
    orders_ordering_key: String,
    /// hash: command id -> ([`Command`], [`Answer`]) (msgpack)
    command_cache_key: String,
    /// list: command ids, oldest first
    command_cache_ordering_key: String,
    /// list: the last [`EVENT_HISTORY_CAPACITY`] [`EventItem`]s (protobuf), oldest first
    events_key: String,
    /// the `sequence` of the last event
    sequence_key: String,
}

impl RedisStore {
    pub async fn connect(url: &str, instance: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let config = ConnectionManagerConfig::new().set_response_timeout(Some(RESPONSE_TIMEOUT));
        let mut connection = ConnectionManager::new_with_config(client, config).await?;
        let _: String = redis::cmd("PING").query_async(&mut connection).await?;
        Ok(RedisStore {
            connection,
            markets_key: format!("engine:{instance}:store:markets"),
            orders_key: format!("engine:{instance}:store:orders"),
            orders_ordering_key: format!("engine:{instance}:store:orders_ordering"),
            command_cache_key: format!("engine:{instance}:store:command_cache"),
            command_cache_ordering_key: format!("engine:{instance}:store:command_cache_ordering"),
            events_key: format!("engine:{instance}:store:events"),
            sequence_key: format!("engine:{instance}:store:sequence"),
        })
    }

    /// Writes all the changes of one command at once (`MULTI`/`EXEC`): Redis has either
    /// all of them or none.
    pub async fn save_engine_changes(&mut self, changes: &[Change]) -> redis::RedisResult<()> {
        let mut pipe = redis::pipe();
        pipe.atomic();
        for change in changes {
            match change {
                Change::MarketSet(market) => {
                    let hvalue = market.encode_to_vec();
                    pipe.hset(&self.markets_key, &market.symbol, hvalue)
                        .ignore();
                }
                Change::OrderAdded(order) => {
                    pipe.hset(&self.orders_key, order.id(), encode(order))
                        .ignore();
                    pipe.rpush(&self.orders_ordering_key, order.id()).ignore();
                }
                Change::OrderUpdated(order) => {
                    // its place in the queue stays the same
                    pipe.hset(&self.orders_key, order.id(), encode(order))
                        .ignore();
                }
                Change::OrderRemoved(id) => {
                    pipe.hdel(&self.orders_key, id).ignore();
                    pipe.lrem(&self.orders_ordering_key, 1, id).ignore();
                }
                Change::CacheAdded {
                    id,
                    command,
                    answer,
                } => {
                    let hvalue = encode(&(command, answer));
                    pipe.hset(&self.command_cache_key, id, hvalue).ignore();
                    pipe.rpush(&self.command_cache_ordering_key, id).ignore();
                }
                Change::CacheRemoved(id) => {
                    // the cache always removes its oldest entry, which is the first in the list
                    pipe.hdel(&self.command_cache_key, id).ignore();
                    pipe.lpop(&self.command_cache_ordering_key, None).ignore();
                }
                Change::EventsAdded(events) => {
                    let encoded: Vec<Vec<u8>> =
                        events.iter().map(|event| event.encode_to_vec()).collect();
                    pipe.rpush(&self.events_key, encoded).ignore();
                    // keep the same last events as the history in memory
                    pipe.ltrim(&self.events_key, -(EVENT_HISTORY_CAPACITY as isize), -1)
                        .ignore();
                    if let Some(last) = events.last() {
                        pipe.set(&self.sequence_key, last.sequence).ignore();
                    }
                }
            }
        }
        pipe.query_async::<()>(&mut self.connection).await
    }

    /// Reads back everything [`RedisStore::save_engine_changes`] wrote, for
    /// [`Engine::new_from_state`](crate::engine::Engine::new_from_state). An instance that
    /// never saved anything (the first start) gets an empty state.
    ///
    /// It fails when the saved parts don't fit together, instead of starting with a book
    /// or a cache that is different from the saved one.
    pub async fn load_engine_state(&mut self) -> Result<EngineState, Box<dyn std::error::Error>> {
        // all in one `MULTI`/`EXEC`, so the parts are from the same moment
        let (markets, mut orders, order_ids, mut cache, cache_ids, events, sequence): Saved =
            redis::pipe()
                .atomic()
                .hgetall(&self.markets_key)
                .hgetall(&self.orders_key)
                .lrange(&self.orders_ordering_key, 0, -1)
                .hgetall(&self.command_cache_key)
                .lrange(&self.command_cache_ordering_key, 0, -1)
                .lrange(&self.events_key, 0, -1)
                .get(&self.sequence_key)
                .query_async(&mut self.connection)
                .await?;

        // the orders in their queue order, see `orders_ordering_key`
        let mut queued_orders = Vec::with_capacity(order_ids.len());
        for id in order_ids {
            let bytes = orders
                .remove(&id)
                .ok_or("an order id in the ordering list has no saved order")?;
            queued_orders.push(rmp_serde::from_slice::<RestingOrder>(&bytes)?);
        }
        if !orders.is_empty() {
            return Err("a saved order is missing from the ordering list".into());
        }

        let mut market_states = Vec::with_capacity(markets.len());
        for bytes in markets.into_values() {
            market_states.push(MarketState {
                market: Market::decode(bytes.as_slice())?,
                orders: Vec::new(),
            });
        }
        // an order doesn't say which market it is in. one engine has one market, so
        // they are all in that one
        if !queued_orders.is_empty() {
            if market_states.len() != 1 {
                return Err(format!(
                    "the saved orders need exactly one saved market, there are {}",
                    market_states.len()
                )
                .into());
            }
            market_states[0].orders = queued_orders;
        }

        let mut command_cache = Vec::with_capacity(cache_ids.len());
        for id in cache_ids {
            let bytes = cache
                .remove(&id)
                .ok_or("a command id in the cache ordering list has no saved entry")?;
            let (command, answer): (Command, Answer) = rmp_serde::from_slice(&bytes)?;
            command_cache.push((CommandId::new(id), command, answer));
        }
        if !cache.is_empty() {
            return Err("a saved cache entry is missing from the cache ordering list".into());
        }

        let mut event_history = Vec::with_capacity(events.len());
        for bytes in events {
            event_history.push(EventItem::decode(bytes.as_slice())?);
        }

        Ok(EngineState {
            markets: market_states,
            sequence: sequence.unwrap_or(0),
            event_history,
            command_cache,
        })
    }

    /// Deletes everything this instance saved, so the next load is like a first start.
    /// The tests use it to start from an empty engine.
    pub async fn clear(&mut self) -> redis::RedisResult<()> {
        let keys = [
            &self.markets_key,
            &self.orders_key,
            &self.orders_ordering_key,
            &self.command_cache_key,
            &self.command_cache_ordering_key,
            &self.events_key,
            &self.sequence_key,
        ];
        let _: usize = self.connection.del(&keys).await?;
        Ok(())
    }
}

/// What [`RedisStore::load_engine_state`] reads, in the order it asks for it.
type Saved = (
    HashMap<String, Vec<u8>>, // markets
    HashMap<u128, Vec<u8>>,   // orders
    Vec<u128>,                // order ids, in their queue order
    HashMap<u128, Vec<u8>>,   // command cache
    Vec<u128>,                // command ids, oldest first
    Vec<Vec<u8>>,             // events, oldest first
    Option<u64>,              // sequence
);

/// msgpack, like [`EngineState::to_bytes`]
fn encode<T: serde::Serialize>(value: &T) -> Vec<u8> {
    rmp_serde::to_vec_named(value).expect("orders and cache entries always encode")
}
