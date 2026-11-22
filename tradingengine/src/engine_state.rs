use proto::Market;
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
