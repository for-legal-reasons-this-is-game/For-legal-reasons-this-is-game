use proto::{Id, Market};

use crate::book::{Book, Execution, Fill};
use crate::engine::Change;
use crate::error::{EngineError, Result};
use crate::ids::OrderId;
use crate::order::{OrderStatus, Side};
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::quote::QuoteScale;
use crate::resting_order::RestingOrder;
use crate::wire::required;

/// what happend to a new order when it was placed. its used for the
/// [`PlaceOrderResponse`](proto::PlaceOrderResponse)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    /// `New` `PartiallyFilled` or `Filled`
    pub status: OrderStatus,
    pub filled_base: BaseQuantity,
    pub filled_quote: QuoteQuantity,
    pub fills: Vec<Fill>,
}

/// Everything a market needs: a [`Market`], a [`QuoteScale`] and a [`Book`].
///
/// The backend gives the [`Market`], the engine calculates the scale, and the book is
/// held by the market.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketInfo {
    market: Market,
    scale: QuoteScale,
    fee_account: Id, // taken out of `market` once so it never has to be checked again
    book: Book,
}

impl MarketInfo {
    pub fn new(market: Market) -> Result<MarketInfo> {
        let scale = check_definition_and_get_scale(&market)?;
        let fee_account = required(market.fee_account)?;
        Ok(MarketInfo {
            market,
            scale,
            fee_account,
            book: Book::new(),
        })
    }

    pub fn load_orders(market: Market, orders: Vec<RestingOrder>) -> Result<MarketInfo> {
        let mut info = MarketInfo::new(market)?;
        for order in orders {
            info.book.load_order(order)?;
        }
        Ok(info)
    }

    /// Reconfigures an existing market. This only works if its book has no open orders.
    ///
    /// The book can't be changed here. That will be implemented separately, with a
    /// [`CancelOrdersRequest`](proto::CancelOrdersRequest).
    pub fn reconfigure(&mut self, market: Market) -> Result<()> {
        let scale = check_definition_and_get_scale(&market)?;
        let fee_account = required(market.fee_account)?;
        if !self.book.is_empty() && !same_decimals(&self.market, &market) {
            return Err(EngineError::HasOpenOrders);
        }
        self.market = market;
        self.scale = scale;
        self.fee_account = fee_account;
        Ok(())
    }

    pub fn market(&self) -> &Market {
        &self.market
    }

    pub fn scale(&self) -> QuoteScale {
        self.scale
    }

    /// the account the fees of every trade go to
    pub fn fee_account(&self) -> Id {
        self.fee_account
    }

    pub fn book(&self) -> &Book {
        &self.book
    }

    /// matches the order against the book and rests whats left
    /// the matched and rest are returned as data in placed
    ///
    /// What it changed in the book goes to `changes`, see [`MarketInfo::book_changes`].
    pub fn place(&mut self, changes: &mut Vec<Change>, order: RestingOrder) -> Result<Placed> {
        self.check_quote_fits(&order)?;

        let execution = self.book.insert(order, self.scale)?;
        // right after the insert, before anything below can fail, so the saved book is
        // always the book in memory
        self.book_changes(&execution, changes);

        // round down per fill same as each TradeSettlement. the sum can come
        // out lower then one quote_for on the total
        let mut filled_quote = QuoteQuantity::ZERO;
        for fill in execution.fills() {
            filled_quote = filled_quote.checked_add(fill.quote)?;
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
            fills: execution.fills().to_vec(),
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

    /// What an insert did to the book, for Redis. It asks the book itself, because the
    /// copies in the fills don't say whether an order is still in the book:
    /// - each maker the new order matched is still resting (partly filled) or gone (filled)
    /// - what's left of the new order rests at the back of its price level. If nothing is
    ///   left, it was never in the book, so there's nothing to save for it.
    fn book_changes(&self, execution: &Execution, changes: &mut Vec<Change>) {
        for fill in execution.fills() {
            let id = fill.maker.id();
            match self.book.get_by_id(id) {
                Some(maker) => changes.push(Change::OrderUpdated(maker.clone())),
                None => changes.push(Change::OrderRemoved(id)),
            }
        }
        if let Some(id) = execution.resting_order_id() {
            let resting = self
                .book
                .get_by_id(id)
                .expect("the insert just put this order in the book");
            changes.push(Change::OrderAdded(resting.clone()));
        }
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

/// Checks the symbol and the decimals a market must have before it can be created.
/// the fee account is checked by the caller since it keeps it
///
/// Checking the scale already calculates it, so the function returns the
/// [`QuoteScale`] as well, to avoid doing the calculation twice.
fn check_definition_and_get_scale(market: &Market) -> Result<QuoteScale> {
    if market.symbol.is_empty() {
        return Err(EngineError::MarketSymbolEmpty);
    }

    QuoteScale::try_from(market)
}

fn same_decimals(old: &Market, new: &Market) -> bool {
    old.base_decimals == new.base_decimals
        && old.quote_decimals == new.quote_decimals
        && old.price_decimals == new.price_decimals
}
