use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

use proto::event_item::Body;
use proto::{
    CancelOrderRequest, CancelOrderResponse, ConfigRejectReason, EventItem, ExecType, Id, Market,
    PlaceOrderRequest, PlaceOrderResponse, RejectReason, SetMarketRequest, SetMarketResponse,
    Timestamp, Uint128,
};

use crate::book::Fill;
use crate::cancel::parse_cancel_order;
use crate::command_cache::{Answer, Command, CommandCache};
use crate::engine_state::{EngineState, MarketState};
use crate::error::{EngineError, Result};
use crate::events::{execution_report, trade_settlement};
use crate::ids::{CommandId, OrderId, TradeId};
use crate::market::{MarketInfo, Placed};
use crate::order::Side;
use crate::place::parse_place_order;
use crate::redis_store::RedisStore;
use crate::resting_order::RestingOrder;
use crate::wire::{order_status, required};

/// One thing a command changed in the [`Engine`]. The engine collects them while it runs
/// a command, and [`State::save_engine_changes`] writes them to Redis.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    MarketSet(Market),
    /// A new order rests at the back of its price level.
    OrderAdded(RestingOrder),
    /// A resting order was partly filled. It keeps its place in the queue.
    OrderUpdated(RestingOrder),
    /// An order left the book: it was filled or cancelled.
    OrderRemoved(OrderId),
    CacheAdded {
        id: CommandId,
        command: Command,
        answer: Answer,
    },
    CacheRemoved(CommandId),
    /// The events of one command, each with its `sequence`.
    EventsAdded(Vec<EventItem>),
}

#[derive(Default)]
pub struct State {
    pub engine: Engine,
    pub store: Option<RedisStore>,
}

impl State {
    /// Saves what the last command changed. Without a store the changes are dropped, so
    /// they don't pile up.
    pub async fn save_engine_changes(&mut self) {
        let changes = std::mem::take(&mut self.engine.changes);
        let Some(store) = &mut self.store else { return };
        if changes.is_empty() {
            // a cache hit or a rejected SetMarket changes nothing
            return;
        }
        store
            .save_engine_changes(&changes)
            .await
            .expect("error while saving changes");
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
    /// the `sequence` of the last event. 0 means no event yet
    sequence: u64,
    /// the last [`EVENT_HISTORY_CAPACITY`] events, oldest first. the consumer gets
    /// all of them again on every connect
    event_history: VecDeque<EventItem>,
}

/// how many events the engine keeps for the consumer. this is the retention window
/// of `how_to_use.md`: a consumer that is down for longer than it takes the engine
/// to make this many events misses the oldest ones. a command saves only its own
/// events, and redis keeps the same last ones, so a bigger number only makes the load
/// at startup bigger
pub const EVENT_HISTORY_CAPACITY: usize = 10_000;

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

        let (command_cache, removed) =
            CommandCache::load_cache(cache_capacity, engine_state.command_cache);
        // only when the capacity is lower than when the cache was saved: the oldest
        // entries didn't fit, and the first save deletes them from redis too
        let changes = removed.into_iter().map(Change::CacheRemoved).collect();
        let mut engine = Engine {
            markets,
            command_cache,
            changes,
            sequence: engine_state.sequence,
            event_history: VecDeque::new(),
        };
        engine.keep_events(engine_state.event_history);
        Ok(engine)
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
            sequence: self.sequence,
            event_history: self.event_history.iter().cloned().collect(),
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

    /// the events kept for the consumer, oldest first
    pub fn event_history(&self) -> &VecDeque<EventItem> {
        &self.event_history
    }

    /// adds the events to the history and drops the oldest ones past the capacity
    fn keep_events(&mut self, events: Vec<EventItem>) {
        self.event_history.extend(events);
        let extra = self
            .event_history
            .len()
            .saturating_sub(EVENT_HISTORY_CAPACITY);
        self.event_history.drain(..extra);
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

    /// places the order and drops its events, see [`Engine::place_order_with_events`]
    pub fn place_order(&mut self, request: PlaceOrderRequest) -> PlaceOrderResponse {
        self.place_order_with_events(request).0
    }

    /// the heart of the place order it trues to place the order
    /// match it if possible against the book and keep the
    /// remainder as rests in the book
    /// this is only in case of limit
    ///
    /// the events go to the backend. TODO
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
        let placed = market.place(&mut self.changes, order)?;
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
    /// and keeps them in the history for the consumer, and in redis
    fn set_unique_sequence(&mut self, symbol: &str, bodies: Vec<Body>) -> Vec<EventItem> {
        let engine_time = Timestamp::from(SystemTime::now());
        let events: Vec<EventItem> = bodies
            .into_iter()
            .map(|body| EventItem {
                sequence: self.next_sequence(),
                engine_time: Some(engine_time),
                market_symbol: symbol.to_string(),
                body: Some(body),
            })
            .collect();
        self.keep_events(events.clone());
        // here and not in `keep_events`: loading calls that too, and must not save
        // the loaded events again. never empty: an RPUSH of nothing fails the whole save
        if !events.is_empty() {
            self.changes.push(Change::EventsAdded(events.clone()));
        }
        events
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

    /// cancels the order and drops its event, see [`Engine::cancel_order_with_events`]
    pub fn cancel_order(&mut self, request: CancelOrderRequest) -> CancelOrderResponse {
        self.cancel_order_with_events(request).0
    }

    /// Cancels an order at its owner's request, and answers with the order's final state.
    ///
    /// A rejected cancel changes nothing.
    ///
    /// the event is one CANCELLED report. a rejected or retried cancel has none
    pub fn cancel_order_with_events(
        &mut self,
        request: CancelOrderRequest,
    ) -> (CancelOrderResponse, Vec<EventItem>) {
        let Some(id) = request.command_id else {
            return self.reject_cancel_order(None, &EngineError::RequiredFieldAbsent);
        };
        let command_id = CommandId::from(id);
        let command = Command::Cancel(request.clone());

        match self.command_cache.search(command_id, &command) {
            Ok(None) => {}
            Ok(Some(Answer::Cancel(response))) => return (response.clone(), Vec::new()),
            Err(error) => return self.reject_cancel_order(Some(command_id), &error),
            _ => {}
        }

        let (response, events) = match self.try_cancel_order(&request) {
            Ok((cancelled_order, events)) => {
                self.changes
                    .push(Change::OrderRemoved(cancelled_order.id()));
                (accept_cancel_order(command_id, &cancelled_order), events)
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
        (response, events)
    }

    /// cancels the order in its market and builds the event for it
    fn try_cancel_order(
        &mut self,
        request: &CancelOrderRequest,
    ) -> Result<(RestingOrder, Vec<EventItem>)> {
        let (market, order_id) = parse_cancel_order(request, &mut self.markets)?;
        let cancelled_order = market.cancel(order_id)?;

        let report = execution_report(&cancelled_order, ExecType::Cancelled, None);
        let events =
            self.set_unique_sequence(&request.market_symbol, vec![Body::ExecutionReport(report)]);
        Ok((cancelled_order, events))
    }

    fn reject_cancel_order(
        &self,
        command_id: Option<CommandId>,
        error: &EngineError,
    ) -> (CancelOrderResponse, Vec<EventItem>) {
        let mut response = CancelOrderResponse {
            command_id: command_id.map(Id::from),
            reject_detail: error.to_string(),
            ..Default::default()
        };
        response.set_reject_reason(reject_reason(error));
        (response, Vec::new())
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

fn accept_cancel_order(
    command_id: CommandId,
    cancelled_order: &RestingOrder,
) -> CancelOrderResponse {
    let mut response = CancelOrderResponse {
        command_id: Some(command_id.into()),
        filled_base_quantity: Some(Uint128::from(cancelled_order.filled_base())),
        filled_quote_quantity: Some(Uint128::from(cancelled_order.filled_quote())),
        ..Default::default()
    };
    response.set_status(order_status(cancelled_order.status()));
    response.set_cancel_reason(proto::CancelReason::UserRequest);
    response
}

/// the events of one match on tradesettlement and 2 execution for 2 sides
fn fill_events(fill: &Fill, fee_account: Id) -> [Body; 3] {
    let trade_id = TradeId::generate();
    let (buyer, seller) = match fill.taker.side() {
        Side::Buy => (&fill.taker, &fill.maker),
        Side::Sell => (&fill.maker, &fill.taker),
    };
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
