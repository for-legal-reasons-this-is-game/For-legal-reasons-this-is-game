use std::time::Duration;

use redis::{
    AsyncCommands,
    aio::{ConnectionManager, ConnectionManagerConfig},
};

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
