use std::collections::HashMap;

use proto::{
    CancelOrderRequest, CancelOrderResponse, ConfigRejectReason, PlaceOrderRequest,
    PlaceOrderResponse, RejectReason, SetMarketRequest, SetMarketResponse, Uint128,
};

use crate::error::{EngineError, Result};
use crate::market::MarketInfo;
// use crate::market::{MarketInfo, Placed};
use crate::order::OrderStatus;
// use crate::place::parse_place_order;
use crate::wire::required;

/// Holds every market, keyed by the market's name (for example `btc-usd`).
///
/// Each market is a [`MarketInfo`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Engine {
    markets: HashMap<String, MarketInfo>,
}

impl Engine {
    // Engine starts without any books this might change for recovery
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

    /// the heart of the place order it trues to place the order
    /// match it if possible against the book and keep the
    /// remainder as rests in the book
    /// this is only in case of limit
    pub fn place_order(&mut self, request: PlaceOrderRequest) -> PlaceOrderResponse {
        let command_id = request.command_id;
        match self.try_place_order(request) {
            Ok(placed) => {
                let mut respone = PlaceOrderResponse {
                    command_id,
                    filled_base_quantity: Some(Uint128::from(placed.filled_base)),
                    filled_quote_quantity: Some(Uint128::from(placed.filled_quote)),
                    ..PlaceOrderResponse::default()
                };
                respone.set_status(order_status(placed.status));
                respone
            }
            Err(error) => {
                let mut response = PlaceOrderResponse {
                    command_id,
                    reject_detail: error.to_string(),
                    ..PlaceOrderResponse::default()
                };
                response.set_reject_reason(reject_reason(&error));
                response
            }
        }
    }

    fn try_place_order(&mut self, request: PlaceOrderRequest) -> Result<Placed> {
        let market = self
            .markets
            .get_mut(&request.market_symbol)
            .ok_or(EngineError::UnknownMarket)?;

        let order = parse_place_order(request)?;
        market.place(order)
    }

    ///
    pub fn cancel_order(&mut self, request: CancelOrderRequest) -> CancelOrderResponse {
        let mut response = CancelOrderResponse {
            ..CancelOrderResponse::default()
        }
        // if (request.market_symbol.is_empty()) {
        //     response.set_reject_reason(config_reject_reason(error));
        // }

        // parse_cancel_order(request.);
        let market = self.markets.get(&request.market_symbol);
        CancelOrderResponse::default()
    }
}

fn config_reject_reason(error: &EngineError) -> ConfigRejectReason {
    match error {
        EngineError::HasOpenOrders => ConfigRejectReason::HasOpenOrders,
        _ => ConfigRejectReason::BadDefinition,
    }
}

// the contract's reject reasons for a place (how_to_use.md, validating a place)
fn reject_reason(error: &EngineError) -> RejectReason {
    match error {
        EngineError::UnknownMarket => RejectReason::UnknownMarket,
        EngineError::BadOrderParams(_)
        | EngineError::RequiredFieldAbsent
        | EngineError::DuplicateOrderId => RejectReason::BadOrderParams,
        EngineError::QuantityNotPositive | EngineError::Overflow => RejectReason::BadQuantity,
        EngineError::PriceNotPositive => RejectReason::BadPrice,
        EngineError::Unsupported(_) => RejectReason::Unsupported,
        _ => RejectReason::Unsupported,
    }
}

// the proto has no Open, a resting order with no fills is New there
fn order_status(status: OrderStatus) -> proto::OrderStatus {
    match status {
        OrderStatus::New | OrderStatus::Open => proto::OrderStatus::New,
        OrderStatus::PartiallyFilled => proto::OrderStatus::PartiallyFilled,
        OrderStatus::Filled => proto::OrderStatus::Filled,
        OrderStatus::Cancelled => proto::OrderStatus::Cancelled,
        OrderStatus::Rejected => proto::OrderStatus::Unspecified,
    }
}
