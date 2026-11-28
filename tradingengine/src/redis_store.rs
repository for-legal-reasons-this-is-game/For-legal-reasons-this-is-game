use std::time::Duration;

use prost::Message;
use redis::{
    AsyncCommands,
    aio::{ConnectionManager, ConnectionManagerConfig},
};

use crate::engine::Change;

/// We currently save the whole state with every request, which could get
/// very big if the command_cache is full, and will take time. This connection
/// config timeout is shared while the save (set) happens. once we change
/// the saving process to only save what changed instead of rewriting the
/// whole state, we can lower this timeout. I didn't test the times with hand,
/// claude did and says the state will reach around 12mb with full
/// command_cache (100k entries).
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct RedisStore {
    connection: ConnectionManager,
    key: String,
    markets_key: String,
    // TODO when we have more markets per engine, this will need to say which market it belongs to
    orders_key: String,
    command_cache_key: String,
    command_cache_ordering_key: String,
}

impl RedisStore {
    pub async fn connect(url: &str, instance: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let config = ConnectionManagerConfig::new().set_response_timeout(Some(RESPONSE_TIMEOUT));
        let mut connection = ConnectionManager::new_with_config(client, config).await?;
        let _: String = redis::cmd("PING").query_async(&mut connection).await?;
        Ok(RedisStore {
            connection,
            key: format!("engine:{instance}:store"),
            markets_key: format!("engine:{instance}:store:markets"),
            orders_key: format!("engine:{instance}:store:orders"),
            command_cache_key: format!("engine:{instance}:store:command_cache"),
            command_cache_ordering_key: format!("engine:{instance}:store:command_cache_ordering"),
        })
    }

    pub async fn load_state(&mut self) -> redis::RedisResult<Option<Vec<u8>>> {
        self.connection.get(&self.key).await
    }

    pub async fn save_state(&mut self, state: &[u8]) -> redis::RedisResult<()> {
        self.connection.set(&self.key, state).await
    }

    pub async fn save_engine_changes(
        &mut self,
        changes: &mut Vec<Change>,
    ) -> redis::RedisResult<()> {
        let mut pipe = redis::pipe();
        pipe.atomic();
        for change in changes {
            match change {
                Change::MarketSet(market) => {
                    let hvalue = market.encode_to_vec();
                    pipe.hset(&self.markets_key, &market.symbol, hvalue)
                        .ignore();
                }
                Change::OrderSaved(order) => {
                    let hvalue = rmp_serde::to_vec_named(order).expect("hehe");
                    pipe.hset(&self.orders_key, order.id(), hvalue).ignore();
                }
                Change::OrderRemoved(id) => {
                    pipe.hdel(&self.orders_key, id).ignore();
                }
                Change::CacheAdded {
                    id,
                    command,
                    answer,
                } => {
                    let hvalue = rmp_serde::to_vec_named(&(command, answer)).expect("hehe");
                    pipe.hset(&self.command_cache_key, &id, hvalue).ignore();
                    pipe.rpush(&self.command_cache_ordering_key, id).ignore();
                }
                Change::CacheRemoved(id) => {
                    pipe.hdel(&self.command_cache_key, &id).ignore();
                    pipe.lpop(&self.command_cache_ordering_key, None).ignore();
                }
            }
        }
        pipe.query_async::<()>(&mut self.connection).await?;
        Ok(())
    }
}
