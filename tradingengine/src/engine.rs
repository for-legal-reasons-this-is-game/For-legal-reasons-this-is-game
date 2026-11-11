use std::collections::HashMap;
use std::sync::RwLock;

use proto::{ConfigRejectReason, SetMarketRequest, SetMarketResponse};

use crate::error::{EngineError, Result};
use crate::market::MarketInfo;
use crate::market_worker::MarketHandle;
use crate::wire::required;

// how many markets one engine instance runs, each on its own thread
pub const MAX_MARKETS: usize = 8;

const LOCK: &str = "engine lock poisoned";

/// Holds every market, keyed by the market's name (for example `btc-usd`).
///
/// Each market is a [`MarketInfo`].
/// the lock only guards the list of markets, never a book: it is held just long
/// enough to find or add a handle and is never held while waiting on a market
#[derive(Debug, Default)]
pub struct Engine {
    markets: RwLock<HashMap<String, MarketHandle>>,
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
    pub async fn set_market(&self, request: SetMarketRequest) -> SetMarketResponse {
        match self.try_set_market(request).await {
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

    async fn try_set_market(&self, request: SetMarketRequest) -> Result<()> {
        let market = required(request.market)?;

        // a new market is checked and started while holding the write lock, so
        // two requests for the same new symbol cannot both start a thread
        let existing = {
            let mut markets = self.markets.write().expect(LOCK);
            match markets.get(&market.symbol) {
                Some(handle) => handle.clone(),
                None => {
                    if markets.len() >= MAX_MARKETS {
                        return Err(EngineError::TooManyMarkets);
                    }
                    let symbol = market.symbol.clone();
                    let info = MarketInfo::new(market)?;
                    markets.insert(symbol, MarketHandle::spawn(info));
                    return Ok(());
                }
            }
        }; // the lock is released here, before waiting on the market thread

        existing.reconfigure(market).await
    }

    // a copy of the market as it is right now, None if it was never set
    pub async fn market(&self, symbol: &str) -> Option<MarketInfo> {
        let handle = self.handle(symbol)?;
        Some(handle.snapshot().await)
    }

    fn handle(&self, symbol: &str) -> Option<MarketHandle> {
        self.markets.read().expect(LOCK).get(symbol).cloned()
    }
}

fn config_reject_reason(error: &EngineError) -> ConfigRejectReason {
    match error {
        EngineError::HasOpenOrders => ConfigRejectReason::HasOpenOrders,
        _ => ConfigRejectReason::BadDefinition,
    }
}
