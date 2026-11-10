use std::collections::HashMap;

use proto::{ConfigRejectReason, SetMarketRequest, SetMarketResponse};

use crate::error::{EngineError, Result};
use crate::market::MarketInfo;
use crate::wire::required;

/// Holds every market, keyed by the market's name (for example `btc-usd`).
///
/// Each market is a [`MarketInfo`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Engine {
    markets: HashMap<String, MarketInfo>,
}

impl Engine {
    /// Creates an engine without any books. This might change for recovery.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a market's values from the backend's [`SetMarketRequest`].
    ///
    /// It can also reconfigure an existing market's scale or values, but only if the
    /// market's book is empty.
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
