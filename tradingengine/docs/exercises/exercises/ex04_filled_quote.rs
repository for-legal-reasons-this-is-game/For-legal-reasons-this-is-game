#![allow(unused)]
// TODO: replace every todo!() until `./check.sh` passes this file. Hints: HINTS.md
// ex04: filled_quote is the sum of the quote of each fill
//
// Sprint: PR A, step 5 ("filled_quote as the sum of quote_for per fill").
// Read: DATA_MODEL.md "Quote math".
//
// Every fill becomes its own TradeSettlement, and each one rounds down on its
// own. The backend adds those up, so the response must too. One quote_for on
// the total rounds only once and can come out higher than what really moved.

use tradingengine::book::Fill;
use tradingengine::error::Result;
use tradingengine::quantity::QuoteQuantity;
use tradingengine::quote::QuoteScale;

pub fn filled_quote(scale: QuoteScale, fills: &[Fill]) -> Result<QuoteQuantity> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_sprint::fixtures::{BTC, dollars};
    use tradingengine::error::EngineError;
    use tradingengine::ids::OrderId;
    use tradingengine::price::Price;
    use tradingengine::quantity::BaseQuantity;

    fn scale() -> QuoteScale {
        QuoteScale::new(2, 8, 2).unwrap()
    }

    fn fill(price: u128, base: u128) -> Fill {
        Fill {
            maker: OrderId::new(1),
            taker: OrderId::new(2),
            price: Price::from_minor_units(price).unwrap(),
            quantity: BaseQuantity::from_minor_units(base),
        }
    }

    #[test]
    fn no_fills_is_zero() {
        assert_eq!(filled_quote(scale(), &[]), Ok(QuoteQuantity::ZERO));
    }

    #[test]
    fn one_fill_is_its_quote() {
        // half a bitcoin at $29,000 is $14,500.00
        let fills = [fill(dollars(29_000), BTC / 2)];
        assert_eq!(
            filled_quote(scale(), &fills),
            Ok(QuoteQuantity::from_minor_units(1_450_000))
        );
    }

    #[test]
    fn each_fill_rounds_down_on_its_own() {
        // 50 sats at $29,000 is 1.45 cents, rounded to 1, three times: 3.
        // the total, 150 sats, is 4.35 cents, rounded once: 4.
        let fills = [
            fill(dollars(29_000), 50),
            fill(dollars(29_000), 50),
            fill(dollars(29_000), 50),
        ];
        assert_eq!(
            filled_quote(scale(), &fills),
            Ok(QuoteQuantity::from_minor_units(3))
        );
    }

    #[test]
    fn fills_at_different_prices_add_up() {
        // 0.1 BTC at $29,000 plus 0.1 BTC at $29,100
        let fills = [fill(dollars(29_000), BTC / 10), fill(dollars(29_100), BTC / 10)];
        assert_eq!(
            filled_quote(scale(), &fills),
            Ok(QuoteQuantity::from_minor_units(290_000 + 291_000))
        );
    }

    #[test]
    fn an_overflowing_fill_is_an_error_not_a_wrap() {
        let fills = [fill(u128::MAX, u128::MAX)];
        assert_eq!(filled_quote(scale(), &fills), Err(EngineError::Overflow));
    }
}
