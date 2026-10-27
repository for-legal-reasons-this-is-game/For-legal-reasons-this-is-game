use proto::Market;

use crate::book::Book;
use crate::error::{EngineError, Result};
use crate::quote::QuoteScale;
use crate::wire::required;

// every market requires a Market, a scale and a book
// scale is calculated by the engine market is given by backend and
// book is help by the Market
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

    // if a market already exists it can be configured only if it has no open orders in the book
    // the book cannot be modified here only but that will be implemented seperately (CancelOrdersRequest event)
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

// everything a market must have before it can be initialized
// checking the scale also returns the scale so to avoid doing
// the calculations twice the funtion returns the scale as well
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
