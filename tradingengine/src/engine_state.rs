use proto::{EventItem, Market};
use serde::{Deserialize, Serialize};

use crate::{
    command_cache::{Answer, Command},
    ids::CommandId,
    proto_bytes,
    resting_order::RestingOrder,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineState {
    pub markets: Vec<MarketState>,
    /// the `sequence` of the last event, so a restart keeps counting from there
    pub sequence: u64,
    /// the last events, oldest first. they are sent again to the consumer on every
    /// connect, so a restart must not lose them. `default` so a save from before
    /// this field still loads
    #[serde(default, with = "proto_bytes::list")]
    pub event_history: Vec<EventItem>,
    pub command_cache: Vec<(CommandId, Command, Answer)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketState {
    #[serde(with = "proto_bytes")]
    pub market: Market,
    pub orders: Vec<RestingOrder>,
}

impl EngineState {
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }
}
