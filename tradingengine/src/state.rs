//! The engine's state as it is saved in Redis, and how it is read back.
//!
//! Every value is JSON, so it can be read with `redis-cli`. The field names below, and the
//! names of [`Side`] and [`OrderStatus`], are the saved format: renaming one breaks loading
//! the state that is already saved. `tests/state.rs` holds saved values that must keep
//! loading.
//!
//! This module only turns state into values and back. Where each value goes is up to
//! [`RedisStore`](crate::store::RedisStore).

use std::fmt;

use proto::{Id, Market};
use serde::{Deserialize, Serialize};

use crate::ended::EndedOrder;
use crate::error::EngineError;
use crate::ids::{CommandId, OrderId, UserId};
use crate::market::MarketInfo;
use crate::order::{OrderStatus, Side};
use crate::price::Price;
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::resting_order::RestingOrder;

/// Why saved state could not be written or read back.
#[derive(Debug)]
pub enum StateError {
    /// A value isn't JSON of the expected shape.
    Json(serde_json::Error),
    /// The values are well formed, but describe a market the engine can't be in.
    Invalid(String),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "saved state isn't the expected JSON: {error}"),
            Self::Invalid(detail) => write!(f, "saved state is invalid: {detail}"),
        }
    }
}

impl std::error::Error for StateError {}

impl From<serde_json::Error> for StateError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<EngineError> for StateError {
    fn from(error: EngineError) -> Self {
        Self::Invalid(error.to_string())
    }
}

/// A market's definition as it is saved: every field of [`Market`].
#[derive(Debug, Serialize, Deserialize)]
struct SavedMarket {
    symbol: String,
    base_decimals: u32,
    quote_decimals: u32,
    price_decimals: u32,
    fee_account: Option<u128>,
}

/// An order that rests in the book, as it is saved. Its status follows from its quantities.
#[derive(Debug, Serialize, Deserialize)]
struct SavedOrder {
    id: u128,
    user_id: u128,
    side: Side,
    price: u128,
    qty_total: u128,
    qty_remaining: u128,
    filled_quote: u128,
    command_id: u128,
}

/// An ended order, as it is saved.
#[derive(Debug, Serialize, Deserialize)]
struct SavedEnded {
    id: u128,
    owner: u128,
    status: OrderStatus,
}

/// A market's definition, as it is saved.
pub fn market_json(market: &Market) -> Result<String, StateError> {
    // no `..` here: if Market gets a new field, this stops compiling until the field is saved
    let Market {
        symbol,
        base_decimals,
        quote_decimals,
        price_decimals,
        fee_account,
    } = market;
    let saved = SavedMarket {
        symbol: symbol.clone(),
        base_decimals: *base_decimals,
        quote_decimals: *quote_decimals,
        price_decimals: *price_decimals,
        fee_account: fee_account.map(u128::from),
    };
    Ok(serde_json::to_string(&saved)?)
}

/// The orders that rest in a market's book, as they are saved: the bids from the best price
/// down, then the asks from the best price up, and the oldest first within each price.
pub fn book_json(market: &MarketInfo) -> Result<String, StateError> {
    let book = market.book();
    let orders: Vec<SavedOrder> = book
        .bids()
        .chain(book.asks())
        .map(|order| SavedOrder {
            id: order.id().value(),
            user_id: order.user_id().value(),
            side: order.side(),
            price: order.price().minor_units(),
            qty_total: order.qty_total().minor_units(),
            qty_remaining: order.qty_remaining().minor_units(),
            filled_quote: order.filled_quote().minor_units(),
            command_id: order.command_id().value(),
        })
        .collect();
    Ok(serde_json::to_string(&orders)?)
}

/// One ended order, as it is saved.
pub fn ended_json(id: OrderId, ended: &EndedOrder) -> Result<String, StateError> {
    let saved = SavedEnded {
        id: id.value(),
        owner: ended.owner.value(),
        status: ended.status,
    };
    Ok(serde_json::to_string(&saved)?)
}

/// Rebuilds a market from what [`market_json`], [`book_json`] and [`ended_json`] saved, with
/// the ended orders oldest first. A market saved without a book gets an empty one.
pub fn restore_market(
    definition: &str,
    book: Option<&str>,
    ended: &[String],
) -> Result<MarketInfo, StateError> {
    let saved: SavedMarket = serde_json::from_str(definition)?;
    let market = Market {
        symbol: saved.symbol,
        base_decimals: saved.base_decimals,
        quote_decimals: saved.quote_decimals,
        price_decimals: saved.price_decimals,
        fee_account: saved.fee_account.map(Id::from),
    };

    let saved_orders: Vec<SavedOrder> = match book {
        Some(json) => serde_json::from_str(json)?,
        None => Vec::new(),
    };
    let orders = saved_orders
        .into_iter()
        .map(restore_order)
        .collect::<Result<Vec<_>, _>>()?;
    let ended = ended
        .iter()
        .map(|json| restore_ended(json))
        .collect::<Result<Vec<_>, _>>()?;

    let restored = MarketInfo::restore(market, orders, ended)?;
    check_restored(&restored)?;
    Ok(restored)
}

/// Rebuilds an order that rested in the book: it is created again, then filled by what it
/// had filled when it was saved.
fn restore_order(saved: SavedOrder) -> Result<RestingOrder, StateError> {
    let invalid = |detail: &str| StateError::Invalid(format!("order {}: {detail}", saved.id));

    let mut order = RestingOrder::new(
        OrderId::new(saved.id),
        UserId::new(saved.user_id),
        saved.side,
        Price::from_minor_units(saved.price)?,
        BaseQuantity::from_minor_units(saved.qty_total),
        CommandId::new(saved.command_id),
    )?;

    let remaining = BaseQuantity::from_minor_units(saved.qty_remaining);
    if remaining > order.qty_total() {
        return Err(invalid("more of it remains than its total"));
    }
    let filled = order.qty_total().checked_sub(remaining)?;
    let filled_quote = QuoteQuantity::from_minor_units(saved.filled_quote);
    if !filled.is_zero() {
        order.fill(filled, filled_quote)?;
    } else if !filled_quote.is_zero() {
        return Err(invalid("nothing of it filled, but it has a filled quote"));
    }

    if order.status().is_terminal() {
        return Err(invalid("it rests in the book with nothing left"));
    }
    Ok(order)
}

/// Rebuilds one ended order.
fn restore_ended(json: &str) -> Result<(OrderId, EndedOrder), StateError> {
    let saved: SavedEnded = serde_json::from_str(json)?;
    if !saved.status.is_terminal() {
        return Err(StateError::Invalid(format!(
            "ended order {} has the status {:?}, which hasn't ended",
            saved.id, saved.status
        )));
    }
    let ended = EndedOrder {
        owner: UserId::new(saved.owner),
        status: saved.status,
    };
    Ok((OrderId::new(saved.id), ended))
}

/// Checks what [`MarketInfo::restore`] can't: that the book doesn't cross, and that no order
/// is both resting and ended.
fn check_restored(market: &MarketInfo) -> Result<(), StateError> {
    let book = market.book();
    if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask())
        && bid >= ask
    {
        return Err(StateError::Invalid(format!(
            "the book crosses: the best bid {} isn't below the best ask {}",
            bid.minor_units(),
            ask.minor_units()
        )));
    }

    let both = book
        .bids()
        .chain(book.asks())
        .find(|order| market.ended().contains(order.id()));
    if let Some(order) = both {
        return Err(StateError::Invalid(format!(
            "order {} is both in the book and ended",
            order.id().value()
        )));
    }
    Ok(())
}
