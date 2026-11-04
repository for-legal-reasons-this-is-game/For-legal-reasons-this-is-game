#![allow(unused)]
// ex07: place a validated order in one market
//
// Sprint: PR A, step 4 ("MarketInfo::place(order) -> Execution, calling
// book.insert") and step 5 (status and filled amounts).
// Read: SLICES.md slice 2, DATA_MODEL.md "Layer 5: RestingOrder".
//
// `Market` below stands in for MarketInfo, whose book you cannot reach from
// outside the crate. In the real PR this becomes `MarketInfo::place`, and it
// must need nothing outside the market (slice T moves it onto its own task).
//
// The status after the first match:
// - nothing filled                    New   (the engine's Open, reported New)
// - something filled, the rest rests  PartiallyFilled
// - nothing rests                     Filled
// Use your ex04 filled_quote: one quote_for per fill.

use engine_sprint::types::{Accepted, LimitGtc};
use tradingengine::book::{Book, Execution};
use tradingengine::error::Result;
use tradingengine::order::OrderStatus;
use tradingengine::quantity::QuoteQuantity;
use tradingengine::quote::QuoteScale;
use tradingengine::resting_order::RestingOrder;

pub struct Market {
    pub scale: QuoteScale,
    pub book: Book,
}

impl Market {
    pub fn new(scale: QuoteScale) -> Self {
        Market {
            scale,
            book: Book::new(),
        }
    }

    pub fn place(&mut self, order: LimitGtc) -> Result<Accepted> {
        //>>
        let resting = RestingOrder::new(
            order.order_id,
            order.account_holder,
            order.side,
            order.price,
            order.quantity,
            order.command_id,
        )?;
        let execution = self.book.insert(resting)?;

        let status = match (execution.fills().is_empty(), execution.resting_order_id()) {
            (_, None) => OrderStatus::Filled,
            (true, Some(_)) => OrderStatus::New,
            (false, Some(_)) => OrderStatus::PartiallyFilled,
        };
        let filled_quote = execution
            .fills()
            .iter()
            .try_fold(QuoteQuantity::ZERO, |total, fill| {
                total.checked_add(self.scale.quote_for(fill.price, fill.quantity)?)
            })?;

        Ok(Accepted {
            status,
            filled_base: execution.total_filled_quantity()?,
            filled_quote,
        })
        //<<
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_sprint::fixtures::{BTC, BTC_USD, dollars};
    use tradingengine::error::EngineError;
    use tradingengine::ids::{CommandId, OrderId, UserId};
    use tradingengine::order::Side;
    use tradingengine::price::Price;
    use tradingengine::quantity::BaseQuantity;

    fn market() -> Market {
        Market::new(QuoteScale::new(2, 8, 2).unwrap())
    }

    fn order(id: u128, owner: u128, side: Side, price: u128, base: u128) -> LimitGtc {
        LimitGtc {
            command_id: CommandId::new(1000 + id),
            order_id: OrderId::new(id),
            symbol: BTC_USD.into(),
            account_holder: UserId::new(owner),
            side,
            price: Price::from_minor_units(price).unwrap(),
            quantity: BaseQuantity::from_minor_units(base),
        }
    }

    fn accepted(status: OrderStatus, base: u128, quote: u128) -> Accepted {
        Accepted {
            status,
            filled_base: BaseQuantity::from_minor_units(base),
            filled_quote: QuoteQuantity::from_minor_units(quote),
        }
    }

    #[test]
    fn an_order_that_crosses_nothing_is_new_and_rests() {
        let mut m = market();
        let answer = m.place(order(1, 10, Side::Sell, dollars(29_000), BTC / 2));
        assert_eq!(answer, Ok(accepted(OrderStatus::New, 0, 0)));
        assert_eq!(m.book.len(), 1);
    }

    #[test]
    fn a_sell_then_a_crossing_buy_is_filled() {
        let mut m = market();
        m.place(order(1, 10, Side::Sell, dollars(29_000), BTC / 2))
            .unwrap();

        let answer = m.place(order(2, 20, Side::Buy, dollars(29_000), BTC / 2));

        assert_eq!(
            answer,
            Ok(accepted(OrderStatus::Filled, BTC / 2, 1_450_000))
        );
        assert!(m.book.is_empty());
    }

    #[test]
    fn the_rest_of_a_partial_fill_rests() {
        let mut m = market();
        m.place(order(1, 10, Side::Sell, dollars(29_000), BTC / 2))
            .unwrap();

        let answer = m.place(order(2, 20, Side::Buy, dollars(29_000), BTC));

        assert_eq!(
            answer,
            Ok(accepted(OrderStatus::PartiallyFilled, BTC / 2, 1_450_000))
        );
        assert_eq!(m.book.best_bid(), Price::from_minor_units(dollars(29_000)).ok());
    }

    #[test]
    fn a_fill_happens_at_the_makers_price() {
        let mut m = market();
        m.place(order(1, 10, Side::Sell, dollars(29_000), BTC / 2))
            .unwrap();

        // willing to pay $29,100 but the resting sell asks $29,000
        let answer = m.place(order(2, 20, Side::Buy, dollars(29_100), BTC / 2));

        assert_eq!(
            answer,
            Ok(accepted(OrderStatus::Filled, BTC / 2, 1_450_000))
        );
    }

    #[test]
    fn filled_quote_rounds_per_fill() {
        let mut m = market();
        for id in 1..=3 {
            m.place(order(id, 10, Side::Sell, dollars(29_000), 50))
                .unwrap();
        }

        // three fills of 1.45 cents: 1 + 1 + 1, not floor(4.35) = 4
        let answer = m.place(order(9, 20, Side::Buy, dollars(29_000), 150));

        assert_eq!(answer, Ok(accepted(OrderStatus::Filled, 150, 3)));
    }

    #[test]
    fn a_reused_order_id_is_an_error_and_changes_nothing() {
        let mut m = market();
        m.place(order(1, 10, Side::Sell, dollars(29_000), BTC))
            .unwrap();

        let answer = m.place(order(1, 10, Side::Sell, dollars(30_000), BTC));

        assert_eq!(answer, Err(EngineError::DuplicateOrderId));
        assert_eq!(m.book.len(), 1);
    }
}
