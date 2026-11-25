use std::time::Duration;

use redis::{
    AsyncCommands,
    aio::{ConnectionManager, ConnectionManagerConfig},
};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct RedisStore {
    connection: ConnectionManager,
    key: String,
}

impl RedisStore {
    pub async fn connect(url: &str, instance: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let config = ConnectionManagerConfig::new().set_response_timeout(Some(RESPONSE_TIMEOUT));
        let mut connection = ConnectionManager::new_with_config(client, config).await?;
        let _: String = redis::cmd("PING").query_async(&mut connection).await?;
        Ok(RedisStore {
            connection,
            key: format!("engine:{instance}:state"),
        })
    }

    pub async fn load_state(&mut self) -> redis::RedisResult<Option<Vec<u8>>> {
        self.connection.get(&self.key).await
    }

    pub async fn save_state(&mut self, state: &[u8]) -> redis::RedisResult<()> {
        self.connection.set(&self.key, state).await
    }
}
