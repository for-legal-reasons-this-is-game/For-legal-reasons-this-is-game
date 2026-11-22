use redis::{AsyncCommands, aio::ConnectionManager};

pub struct RedisStore {
    connection: ConnectionManager,
    key: String,
}

impl RedisStore {
    pub async fn connect(url: &str, instance: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let mut connection = ConnectionManager::new(client).await?;
        let _: String = redis::cmd("PING").query_async(&mut connection).await?;
        Ok(RedisStore {
            connection,
            key: format!("engine:{instance}:state"),
        })
    }

    pub async fn load(&mut self) -> redis::RedisResult<Option<Vec<u8>>> {
        self.connection.get(&self.key).await
    }

    pub async fn save(&mut self, state: &[u8]) -> redis::RedisResult<()> {
        self.connection.set(&self.key, state).await
    }
}
