use proto::Market;

use crate::book::Book;
use crate::error::{EngineError, Result};
use crate::quote::QuoteScale;
use crate::wire::required;

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
