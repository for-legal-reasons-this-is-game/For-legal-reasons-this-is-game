use prost::Message;
use proto::EngineCommand;
use redis::{Commands, Connection, RedisResult};

const ACTIVE_PODS: &str = "task:active";
const PENDING_EVENTS: &str = "task:pending";
const TO_BE_CREATED_PODS: &str = "task:to_be_created";

/// This service only writes to Redis; the engine pod pops and computes.
pub struct Redis {
    inner: Connection,
}

impl Redis {
    pub fn new(inner: Connection) -> Self {
        Redis { inner }
    }

    fn pending_key(market_symbol: &str) -> String {
        format!("{PENDING_EVENTS}:{market_symbol}")
    }

    // fn get_active_pods(&mut self) -> RedisResult<Vec<String>> {
    //     self.inner.smembers(ACTIVE_PODS)
    // }
    //
    // fn get_to_be_created_pods(&mut self) -> RedisResult<Vec<String>> {
    //     self.inner.lrange(TO_BE_CREATED_PODS, 0, -1)
    // }

    fn is_active(&mut self, market_symbol: &str) -> RedisResult<bool> {
        self.inner.sismember(ACTIVE_PODS, market_symbol)
    }

    /// Ask for a pod for `market_name` (KEDA watches this list). No-op if already waiting.
    pub fn add_to_be_created_pod(&mut self, market_name: &str) -> RedisResult<()> {
        let waiting: Vec<String> = self.inner.lrange(TO_BE_CREATED_PODS, 0, -1)?;
        if !waiting.iter().any(|m| m == market_name) {
            let _: () = self.inner.rpush(TO_BE_CREATED_PODS, market_name)?;
        }
        Ok(())
    }

    /// Push a command for the engine to compute. Always queued; if the market has
    /// no pod yet, one is requested. Returns `true` if a pod was already active.
    pub fn push_command(
        &mut self,
        market_symbol: &str,
        command: EngineCommand,
    ) -> RedisResult<bool> {
        let _: () = self
            .inner
            .rpush(Self::pending_key(market_symbol), command.encode_to_vec())?;
        let active = self.is_active(market_symbol)?;
        if !active {
            self.add_to_be_created_pod(market_symbol)?;
        }
        Ok(active)
    }
}
