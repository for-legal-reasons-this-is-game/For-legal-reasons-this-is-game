use std::collections::HashMap;
use std::time::SystemTime;

use proto::event_item::Body;
use proto::{
    CancelOrderRequest, CancelOrderResponse, ConfigRejectReason, EventItem, ExecType, Id,
    PlaceOrderRequest, PlaceOrderResponse, RejectReason, SetMarketRequest, SetMarketResponse,
    Timestamp, Uint128,
};

use crate::book::Fill;
use crate::cancel::parse_cancel_order;
use crate::command_cache::{Answer, Command, CommandCache};
use crate::engine_state::{EngineState, MarketState};
use crate::error::{EngineError, Result};
use crate::events::{execution_report, trade_settlement};
use crate::ids::{CommandId, TradeId};
use crate::market::{MarketInfo, Placed};
use crate::place::parse_place_order;
use crate::quantity::BaseQuantity;
use crate::redis_store::RedisStore;
use crate::resting_order::RestingOrder;
use crate::wire::{order_status, required};

#[derive(Default)]
pub struct State {
    pub engine: Engine,
    pub store: Option<RedisStore>,
}

impl State {
    /// Saves `engine`'s state to redis' `store`
    pub async fn save(&mut self) {
        let Some(store) = &mut self.store else {
            return;
        };
        let state_bytes = self
            .engine
            .get_engine_state()
            .to_bytes()
            .expect("error converting `EngineState` to bytes");
        match store.save_state(&state_bytes).await {
            Ok(_) => {}
            Err(error) => panic!("error saving in `RedisStore`: {error}"),
        };
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
    /// the `sequence` of the last event. 0 means no event yet
    sequence: u64,
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
        Ok(Engine {
            markets,
            command_cache,
            sequence: engine_state.sequence,
        })
    }

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
            sequence: self.sequence,
            command_cache: self.command_cache.get_cache(),
        }
    }

    /// The `sequence` for the next event: one more than the last, so the first event
    /// gets 1. The contract wants no gaps and no repeats.
    /// a u64 doesnt run out, even a billion events a second takes 500+ years
    pub fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
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

    pub fn markets(&self) -> &HashMap<String, MarketInfo> {
        &self.markets
    }

    pub fn market(&self, symbol: &str) -> Option<&MarketInfo> {
        self.markets.get(symbol)
    }

    /// places the order and drops its events, see [`Engine::place_order_with_events`]
    pub fn place_order(&mut self, request: PlaceOrderRequest) -> PlaceOrderResponse {
        self.place_order_with_events(request).0
    }

    /// the heart of the place order it trues to place the order
    /// match it if possible against the book and keep the
    /// remainder as rests in the book
    /// this is only in case of limit
    ///
    /// the events go to the backend.
    pub fn place_order_with_events(
        &mut self,
        request: PlaceOrderRequest,
    ) -> (PlaceOrderResponse, Vec<EventItem>) {
        let Some(id) = request.command_id else {
            return self.reject_place_order(None, &EngineError::RequiredFieldAbsent);
        };
        let command_id = CommandId::from(id);
        let command = Command::Place(Box::new(request.clone()));

        match self.command_cache.search(command_id, &command) {
            Ok(None) => {}
            Ok(Some(Answer::Place(response))) => return (response.clone(), Vec::new()),
            Err(error) => return self.reject_place_order(Some(command_id), &error),
            _ => {}
        }

        let (response, events) = match self.try_place_order(request) {
            Ok((placed, events)) => (accept_place_order(command_id, &placed), events),
            Err(error) => self.reject_place_order(Some(command_id), &error),
        };

        self.command_cache
            .add_entry(command_id, command, Answer::Place(response.clone()));
        (response, events)
    }

    /// places the order in its market and builds the events for it
    fn try_place_order(&mut self, request: PlaceOrderRequest) -> Result<(Placed, Vec<EventItem>)> {
        let symbol = request.market_symbol.clone();
        let market = self
            .markets
            .get_mut(&symbol)
            .ok_or(EngineError::UnknownMarket)?;

        let order = parse_place_order(request)?;
        let new_order = order.clone();
        let placed = market.place(order)?;
        let fee_account = market.fee_account();

        let events = self.place_events(&symbol, &new_order, &placed.fills, fee_account);
        Ok((placed, events))
    }

    /// NEW for the order, then the events of each match in the order they happend
    fn place_events(
        &mut self,
        symbol: &str,
        new_order: &RestingOrder,
        fills: &[Fill],
        fee_account: Id,
    ) -> Vec<EventItem> {
        let mut bodies = vec![Body::ExecutionReport(execution_report(
            new_order,
            ExecType::New,
            None,
        ))];
        for fill in fills {
            bodies.extend(fill_events(fill, fee_account));
        }
        self.set_unique_sequence(symbol, bodies)
    }

    /// gives every event of one command the next `sequence`, and `time`
    fn set_unique_sequence(&mut self, symbol: &str, bodies: Vec<Body>) -> Vec<EventItem> {
        let engine_time = Timestamp::from(SystemTime::now());
        bodies
            .into_iter()
            .map(|body| EventItem {
                sequence: self.next_sequence(),
                engine_time: Some(engine_time),
                market_symbol: symbol.to_string(),
                body: Some(body),
            })
            .collect()
    }

    fn reject_place_order(
        &self,
        command_id: Option<CommandId>,
        error: &EngineError,
    ) -> (PlaceOrderResponse, Vec<EventItem>) {
        let mut response = PlaceOrderResponse {
            command_id: command_id.map(Id::from),
            reject_detail: error.to_string(),
            ..Default::default()
        };
        response.set_reject_reason(reject_reason(error));
        (response, Vec::new())
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
                let mut response = CancelOrderResponse {
                    command_id: Some(command_id.into()),
                    filled_base_quantity: Some(Uint128::from(filled_base)),
                    filled_quote_quantity: Some(Uint128::from(cancelled_order.filled_quote())),
                    ..Default::default()
                };
                response.set_status(order_status(cancelled_order.status()));
                response.set_cancel_reason(proto::CancelReason::UserRequest);
                response
            }
            Err(error) => self.reject_cancel_order(Some(command_id), &error),
        };
        self.command_cache
            .add_entry(command_id, command, Answer::Cancel(response.clone()));
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

fn accept_place_order(command_id: CommandId, placed: &Placed) -> PlaceOrderResponse {
    let mut response = PlaceOrderResponse {
        command_id: Some(command_id.into()),
        filled_base_quantity: Some(Uint128::from(placed.filled_base)),
        filled_quote_quantity: Some(Uint128::from(placed.filled_quote)),
        ..Default::default()
    };
    response.set_status(order_status(placed.status));
    response
}

/// the events of one match on tradesettlement and 2 execution for 2 sides
fn fill_events(fill: &Fill, fee_account: Id) -> [Body; 3] {
    let trade_id = TradeId::generate();
    let (buyer, seller) = fill.buyer_and_seller();
    [
        Body::TradeSettlement(trade_settlement(fill, trade_id, fee_account)),
        Body::ExecutionReport(execution_report(buyer, ExecType::Trade, Some(trade_id))),
        Body::ExecutionReport(execution_report(seller, ExecType::Trade, Some(trade_id))),
    ]
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
