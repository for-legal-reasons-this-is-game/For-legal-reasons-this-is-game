use std::collections::HashMap;

use proto::{ConfigRejectReason, SetMarketRequest, SetMarketResponse};

use crate::error::{EngineError, Result};
use crate::market::MarketInfo;
use crate::wire::required;

// the engine holds a hashmap of the key pair value of each market here
// String is the name will possibaly be changed to marketid which is more preferable
// this change should be done by the backend if they decide and the value are the markets (market.rs)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Engine {
    markets: HashMap<String, MarketInfo>,
}

impl Engine {
    // Engine starts without any books this might change for recovery
    pub fn new() -> Self {
        Self::default()
    }

    // set market tries to set a Market's values from backend
    // it can also reconfigure a markets scale or values
    // the market can only be be reconfigured if it has an empty book
    pub fn set_market(&mut self, request: SetMarketRequest) -> SetMarketResponse {
        match self.try_set_market(request) {
            Ok(()) => SetMarketResponse::default(),
            Err(error) => {
                let mut response = SetMarketResponse {
                    reject_detail: error.to_string(),
                    ..SetMarketResponse::default()
                };
                response.set_reject_reason(config_reject_reason(&error));
                response
            }
        }
    }

    fn try_set_market(&mut self, request: SetMarketRequest) -> Result<()> {
        let market = required(request.market)?;
        match self.markets.get_mut(&market.symbol) {
            Some(state) => state.reconfigure(market),
            None => {
                let symbol = market.symbol.clone();
                self.markets.insert(symbol, MarketInfo::new(market)?);
                Ok(())
            }
        }
    }

    pub fn market(&self, symbol: &str) -> Option<&MarketInfo> {
        self.markets.get(symbol)
    }
}

fn config_reject_reason(error: &EngineError) -> ConfigRejectReason {
    match error {
        EngineError::HasOpenOrders => ConfigRejectReason::HasOpenOrders,
        _ => ConfigRejectReason::BadDefinition,
    }
}
