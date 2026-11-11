use std::collections::HashMap;

use proto::{
    ConfigRejectReason, PlaceOrderRequest, PlaceOrderResponse, RejectReason, SetMarketRequest,
    SetMarketResponse, Uint128,
};

use crate::error::{EngineError, Result};
use crate::market::{MarketInfo, Placed};
use crate::order::OrderStatus;
use crate::place::parse_place_order;
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

    /// checks a [`PlaceOrderRequest`] then matches it in its markets book and rests
    /// whats left
    ///
    /// a rejected order changes nothing. the response is just the quick answer
    /// for the user money only moves later with the `TradeSettlement`
    pub fn place_order(&mut self, request: PlaceOrderRequest) -> PlaceOrderResponse {
        todo!("parse then try then place");
    }
}

fn config_reject_reason(error: &EngineError) -> ConfigRejectReason {
    match error {
        EngineError::HasOpenOrders => ConfigRejectReason::HasOpenOrders,
        _ => ConfigRejectReason::BadDefinition,
    }
}

/// the [`RejectReason`] for each error a place can hit like in
/// `proto/how_to_use.md`
fn reject_reason(error: &EngineError) -> RejectReason {
    match error {
        EngineError::UnknownMarket => RejectReason::UnknownMarket,
        // a reused order_id is the clients mistake so we answer it. a panic
        // would let any client kill the pod. DUPLICATE_COMMAND is for command_id
        EngineError::BadOrderParams(_)
        | EngineError::RequiredFieldAbsent
        | EngineError::DuplicateOrderId => RejectReason::BadOrderParams,
        // the contract says a quote over 128 bits is BAD_QUANTITY
        EngineError::QuantityNotPositive | EngineError::Overflow => RejectReason::BadQuantity,
        EngineError::PriceNotPositive => RejectReason::BadPrice,
        EngineError::Unsupported(_) => RejectReason::Unsupported,
        // no reason in the contract for these. the team still has to decide
        // wich ones panic untill then they are UNSUPPORTED
        _ => RejectReason::Unsupported,
    }
}

/// our [`OrderStatus`] as the proto one
///
/// the proto has no `Open` so a resting order with no fills is `New`
fn order_status(status: OrderStatus) -> proto::OrderStatus {
    match status {
        OrderStatus::New | OrderStatus::Open => proto::OrderStatus::New,
        OrderStatus::PartiallyFilled => proto::OrderStatus::PartiallyFilled,
        OrderStatus::Filled => proto::OrderStatus::Filled,
        OrderStatus::Cancelled => proto::OrderStatus::Cancelled,
        // a rejected order never existed so it has no status
        OrderStatus::Rejected => proto::OrderStatus::Unspecified,
    }
}
