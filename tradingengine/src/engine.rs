use std::collections::HashMap;

use proto::{
    CancelOrderRequest, CancelOrderResponse, ConfigRejectReason, Id, Market, PlaceOrderRequest,
    PlaceOrderResponse, RejectReason, SetMarketRequest, SetMarketResponse, Uint128,
};

use crate::cancel::parse_cancel_order;
use crate::command_cache::{Answer, Command, CommandCache};
use crate::engine_state::{EngineState, MarketState};
use crate::error::{EngineError, Result};
use crate::ids::{CommandId, OrderId};
use crate::market::{MarketInfo, Placed};
use crate::order::OrderStatus;
use crate::place::parse_place_order;
use crate::quantity::BaseQuantity;
use crate::redis_store::RedisStore;
use crate::resting_order::RestingOrder;
use crate::wire::required;

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    MarketSet(Market),
    OrderSaved(RestingOrder),
    OrderRemoved(OrderId),
    CacheAdded {
        id: CommandId,
        command: Command,
        answer: Answer,
    },
    CacheRemoved(CommandId),
}

#[derive(Default)]
pub struct State {
    pub engine: Engine,
    pub store: Option<RedisStore>,
}

impl State {
    pub async fn save_engine_changes(&mut self) {
        let Some(store) = &mut self.store else { return };
        let changes = &mut self.engine.changes;
        store
            .save_engine_changes(changes)
            .await
            .expect("error while saving state");
        self.engine.changes.clear();
    }
}

/// Holds every market, keyed by the market's name (for example `btc-usd`).
///
/// Each market is a [`MarketInfo`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Engine {
    markets: HashMap<String, MarketInfo>,
    // The cache lives here, so an [`EngineService`] messege handler that will lock
    // the [`Engine`] mutex will guarantee no two requests race on the cache
    command_cache: CommandCache,
    changes: Vec<Change>,
}

impl Engine {
    // Engine starts without any books this might change for recovery
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_state(engine_state: EngineState, cache_capacity: usize) -> Result<Engine> {
        let markets = engine_state
            .markets
            .into_iter()
            .map(|m| {
                Ok((
                    m.market.symbol.clone(),
                    MarketInfo::load_orders(m.market, m.orders)?,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let command_cache = CommandCache::load_cache(cache_capacity, engine_state.command_cache);
        let changes = Vec::new();
        Ok(Engine {
            markets,
            command_cache,
            changes,
        })
    }

    /// This function will be used in tests only
    pub fn get_engine_state(&self) -> EngineState {
        EngineState {
            markets: self
                .markets
                .values()
                .map(|info| MarketState {
                    market: info.market().clone(),
                    orders: info.book().get_orders(),
                })
                .collect(),
            command_cache: self.command_cache.get_cache(),
        }
    }

    pub fn with_cache_capacity(n: usize) -> Self {
        Engine {
            command_cache: CommandCache::with_capacity(n),
            ..Default::default()
        }
    }

    /// Sets a market's values from the backend's [`SetMarketRequest`].
    ///
    /// It can also reconfigure an existing market's scale or values, but only if the
    /// market's book is empty.
    pub fn set_market(&mut self, request: SetMarketRequest) -> SetMarketResponse {
        match self.try_set_market(&request) {
            Ok(()) => {
                let response = SetMarketResponse::default();
                self.changes
                    .push(Change::MarketSet(request.market.unwrap()));
                response
            }
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

    fn try_set_market(&mut self, request: &SetMarketRequest) -> Result<()> {
        let market = required(request.market.clone())?;
        match self.markets.get_mut(&market.symbol) {
            Some(state) => state.reconfigure(market),
            None => {
                let symbol = market.symbol.clone();
                self.markets.insert(symbol, MarketInfo::new(market)?);
                Ok(())
            }
        }
    }

    pub fn markets(&self) -> &HashMap<String, MarketInfo> {
        &self.markets
    }

    pub fn market(&self, symbol: &str) -> Option<&MarketInfo> {
        self.markets.get(symbol)
    }

    /// the heart of the place order it trues to place the order
    /// match it if possible against the book and keep the
    /// remainder as rests in the book
    /// this is only in case of limit
    pub fn place_order(&mut self, request: PlaceOrderRequest) -> PlaceOrderResponse {
        let Some(id) = request.command_id else {
            return self.reject_place_order(None, &EngineError::RequiredFieldAbsent);
        };
        let command_id = CommandId::from(id);
        let command = Command::Place(Box::new(request.clone()));

        match self.command_cache.search(command_id, &command) {
            Ok(None) => {}
            Ok(Some(Answer::Place(response))) => return response.clone(),
            Err(error) => return self.reject_place_order(Some(command_id), &error),
            _ => {}
        }

        // TODO unimplemented `Change` saving in this path. need to save all changed orders. do it with the stream with Mehres
        let response = match self.try_place_order(request) {
            Ok(placed) => {
                let mut response = PlaceOrderResponse {
                    command_id: Some(command_id.into()),
                    filled_base_quantity: Some(Uint128::from(placed.filled_base)),
                    filled_quote_quantity: Some(Uint128::from(placed.filled_quote)),
                    ..Default::default()
                };
                response.set_status(order_status(placed.status));
                response
            }
            Err(error) => self.reject_place_order(Some(command_id), &error),
        };

        match self.command_cache.add_entry(
            command_id,
            command.clone(),
            Answer::Place(response.clone()),
        ) {
            None => {}
            Some(removed_id) => self.changes.push(Change::CacheRemoved(removed_id)),
        }
        self.changes.push(Change::CacheAdded {
            id: command_id,
            command,
            answer: Answer::Place(response.clone()),
        });
        response
    }

    fn try_place_order(&mut self, request: PlaceOrderRequest) -> Result<Placed> {
        let market = self
            .markets
            .get_mut(&request.market_symbol)
            .ok_or(EngineError::UnknownMarket)?;

        let order = parse_place_order(request)?;
        market.place(order)
    }

    fn reject_place_order(
        &self,
        command_id: Option<CommandId>,
        error: &EngineError,
    ) -> PlaceOrderResponse {
        let mut response = PlaceOrderResponse {
            command_id: command_id.map(Id::from),
            reject_detail: error.to_string(),
            ..Default::default()
        };
        response.set_reject_reason(reject_reason(error));
        response
    }
    /// Cancels an order at its owner's request, and answers with the order's final state.
    ///
    /// A rejected cancel changes nothing.
    pub fn cancel_order(&mut self, request: CancelOrderRequest) -> CancelOrderResponse {
        let Some(id) = request.command_id else {
            return self.reject_cancel_order(None, &EngineError::RequiredFieldAbsent);
        };
        let command_id = CommandId::from(id);
        let command = Command::Cancel(request.clone());

        match self.command_cache.search(command_id, &command) {
            Ok(None) => {}
            Ok(Some(Answer::Cancel(response))) => return response.clone(),
            Err(error) => return self.reject_cancel_order(Some(command_id), &error),
            _ => {}
        }

        let response = match self.try_cancel_order(&request) {
            Ok((cancelled_order, filled_base)) => {
                // TODO(mehras): RestingOrder doesn't keep the quote it has filled, so the answer
                // has no filled_quote_quantity yet. The contract wants both filled amounts
                // (how_to_use.md, "Handling command responses"). Settle how to track it, and
                // until then tell Alvaro that a cancel answer only has the filled base
                let mut response = CancelOrderResponse {
                    command_id: Some(command_id.into()),
                    filled_base_quantity: Some(Uint128::from(filled_base)),
                    ..Default::default()
                };
                response.set_status(order_status(cancelled_order.status()));
                response.set_cancel_reason(proto::CancelReason::UserRequest);
                self.changes
                    .push(Change::OrderRemoved(cancelled_order.id()));
                response
            }
            Err(error) => self.reject_cancel_order(Some(command_id), &error),
        };

        match self.command_cache.add_entry(
            command_id,
            command.clone(),
            Answer::Cancel(response.clone()),
        ) {
            None => {}
            Some(removed_id) => self.changes.push(Change::CacheRemoved(removed_id)),
        }
        self.changes.push(Change::CacheAdded {
            id: command_id,
            command,
            answer: Answer::Cancel(response.clone()),
        });
        response
    }

    /// The cancelled order and how much of its base it filled before the cancel.
    fn try_cancel_order(
        &mut self,
        request: &CancelOrderRequest,
    ) -> Result<(RestingOrder, BaseQuantity)> {
        let (market, order_id) = parse_cancel_order(request, &mut self.markets)?;
        let order = market.cancel(order_id)?;
        let filled_base = order.qty_total().checked_sub(order.qty_remaining())?;
        Ok((order, filled_base))
    }

    fn reject_cancel_order(
        &self,
        command_id: Option<CommandId>,
        error: &EngineError,
    ) -> CancelOrderResponse {
        let mut response = CancelOrderResponse {
            command_id: command_id.map(Id::from),
            reject_detail: error.to_string(),
            ..Default::default()
        };
        response.set_reject_reason(reject_reason(error));
        response
    }
}

fn config_reject_reason(error: &EngineError) -> ConfigRejectReason {
    match error {
        EngineError::HasOpenOrders => ConfigRejectReason::HasOpenOrders,
        _ => ConfigRejectReason::BadDefinition,
    }
}

// the contract's reject reasons for a place and a cancel (how_to_use.md, validating a place,
// validating a cancel or modify command)
fn reject_reason(error: &EngineError) -> RejectReason {
    match error {
        EngineError::UnknownMarket => RejectReason::UnknownMarket,
        EngineError::BadOrderParams(_)
        | EngineError::RequiredFieldAbsent
        | EngineError::DuplicateOrderId => RejectReason::BadOrderParams,
        EngineError::QuantityNotPositive | EngineError::Overflow => RejectReason::BadQuantity,
        EngineError::PriceNotPositive => RejectReason::BadPrice,
        EngineError::Unsupported(_) => RejectReason::Unsupported,
        EngineError::OrderNotFound => RejectReason::UnknownOrder,
        EngineError::NotOrderOwner => RejectReason::NotOrderOwner,
        EngineError::OrderNotLive => RejectReason::OrderNotWorking,
        EngineError::DuplicateCommand => RejectReason::DuplicateCommand,
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
