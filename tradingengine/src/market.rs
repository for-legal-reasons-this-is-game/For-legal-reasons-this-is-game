use proto::Market;

use crate::book::Book;
use crate::error::{EngineError, Result};
use crate::ids::OrderId;
use crate::order::{OrderStatus, Side};
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::quote::QuoteScale;
use crate::resting_order::RestingOrder;
use crate::wire::required;

/// what happend to a new order when it was placed. its used for the
/// [`PlaceOrderResponse`](proto::PlaceOrderResponse)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// `New` `PartiallyFilled` or `Filled`
    pub status: OrderStatus,
    pub filled_base: BaseQuantity,
    pub filled_quote: QuoteQuantity,
}

/// Everything a market needs: a [`Market`], a [`QuoteScale`] and a [`Book`].
///
/// The backend gives the [`Market`], the engine calculates the scale, and the book is
/// held by the market.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketInfo {
    market: Market,
    scale: QuoteScale,
    book: Book,
}

impl MarketInfo {
    pub fn new(market: Market) -> Result<MarketInfo> {
        let scale = check_definition_and_get_scale(&market)?;
        Ok(MarketInfo {
            market,
            scale,
            book: Book::new(),
        })
    }

    /// Reconfigures an existing market. This only works if its book has no open orders.
    ///
    /// The book can't be changed here. That will be implemented separately, with a
    /// [`CancelOrdersRequest`](proto::CancelOrdersRequest).
    pub fn reconfigure(&mut self, market: Market) -> Result<()> {
        let scale = check_definition_and_get_scale(&market)?;
        if !self.book.is_empty() && !same_decimals(&self.market, &market) {
            return Err(EngineError::HasOpenOrders);
        }
        self.market = market;
        self.scale = scale;
        Ok(())
    }

    pub fn market(&self) -> &Market {
        &self.market
    }

    pub fn scale(&self) -> QuoteScale {
        self.scale
    }

    pub fn book(&self) -> &Book {
        &self.book
    }

    /// matches the order against the book and rests whats left
    /// the matched and rest are returned as data in placed
    pub fn place(&mut self, order: RestingOrder) -> Result<Placed> {
        self.check_quote_fits(&order)?;

        let execution = self.book.insert(order)?;

        // round down per fill same as each TradeSettlement. the sum can come
        // out lower then one quote_for on the total
        let mut filled_quote = QuoteQuantity::ZERO;
        for fill in execution.fills() {
            let quote = self.scale.quote_for(fill.price, fill.quantity)?;
            filled_quote = filled_quote.checked_add(quote)?;
        }
        let filled_base = execution.total_filled_quantity()?;

        let status = match execution.resting_order_id() {
            None => OrderStatus::Filled,
            Some(_) if filled_base.is_zero() => OrderStatus::New,
            Some(_) => OrderStatus::PartiallyFilled,
        };
        Ok(Placed {
            status,
            filled_base,
            filled_quote,
        })
    }

    /// Cancels a resting order and takes it out of the book.
    ///
    /// The cancel is tried on a copy first, so a rejected cancel leaves the book as it was.
    pub fn cancel(&mut self, order_id: OrderId) -> Result<RestingOrder> {
        let mut order = self
            .book
            .get_by_id(order_id)
            .ok_or(EngineError::OrderNotFound)?
            .clone();
        order.cancel()?;
        self.book.remove(order_id)?;
        Ok(order)
    }

    /// make sure the quote does not over flow
    fn check_quote_fits(&self, order: &RestingOrder) -> Result<()> {
        let highest_price = match (order.side(), self.book.best_bid()) {
            (Side::Sell, Some(best_bid)) => best_bid.max(order.price()),
            _ => order.price(),
        };
        self.scale.quote_for(highest_price, order.qty_total())?;
        Ok(())
    }
}

/// Checks everything a market must have before it can be created.
///
/// Checking the scale already calculates it, so the function returns the
/// [`QuoteScale`] as well, to avoid doing the calculation twice.
fn check_definition_and_get_scale(market: &Market) -> Result<QuoteScale> {
    if market.symbol.is_empty() {
        return Err(EngineError::MarketSymbolEmpty);
    }

    required(market.fee_account.as_ref())?;
    QuoteScale::try_from(market)
}

fn same_decimals(old: &Market, new: &Market) -> bool {
    old.base_decimals == new.base_decimals
        && old.quote_decimals == new.quote_decimals
        && old.price_decimals == new.price_decimals
}
