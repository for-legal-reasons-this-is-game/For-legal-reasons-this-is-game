#![allow(unused)]
// ex02: absent is not zero
//
// Sprint: PR A, step 3 ("required() for the ids and the price", "only the
// base_quantity arm of amount"). Real code goes in src/wire.rs / the place parser.
// Read: DATA_MODEL.md "Wire conventions" (absent versus zero).
//
// A message field (Id, Uint128) can be absent, and absent is not zero:
// - an absent required Id or price is BAD_ORDER_PARAMS (proposed)
// - a zero price is BAD_PRICE, a zero quantity is BAD_QUANTITY
// Use `tradingengine::wire::required` and the From/TryFrom impls in wire.rs.

use proto::place_order_request::Amount;
use proto::{Id, RejectReason, Uint128};
use tradingengine::ids::OrderId;
use tradingengine::price::Price;
use tradingengine::quantity::BaseQuantity;
use tradingengine::wire::{required, required_id};

pub fn order_id(field: Option<Id>) -> Result<OrderId, RejectReason> {
    //>>
    required_id(field)
        .map(OrderId::new)
        .map_err(|_| RejectReason::BadOrderParams)
    //<<
}

pub fn limit_price(field: Option<Uint128>) -> Result<Price, RejectReason> {
    //>>
    let wire = required(field).map_err(|_| RejectReason::BadOrderParams)?;
    Price::try_from(wire).map_err(|_| RejectReason::BadPrice)
    //<<
}

// a limit order needs the base arm. No amount at all, or the quote arm, is
// the wrong combination of fields; a zero base is a bad quantity
pub fn base_amount(amount: Option<Amount>) -> Result<BaseQuantity, RejectReason> {
    //>>
    match amount {
        Some(Amount::BaseQuantity(wire)) => BaseQuantity::from(wire)
            .require_positive()
            .map_err(|_| RejectReason::BadQuantity),
        Some(Amount::QuoteQuantity(_)) | None => Err(RejectReason::BadOrderParams),
    }
    //<<
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_must_be_present() {
        assert_eq!(order_id(Some(Id::from(7u128))), Ok(OrderId::new(7)));
        assert_eq!(order_id(None), Err(RejectReason::BadOrderParams));
    }

    #[test]
    fn a_zero_id_is_still_an_id() {
        // any u128 is a valid id, the engine never invents or checks one
        assert_eq!(order_id(Some(Id::from(0u128))), Ok(OrderId::new(0)));
    }

    #[test]
    fn an_absent_price_and_a_zero_price_are_different_mistakes() {
        assert_eq!(
            limit_price(Some(Uint128::from(2_900_000u128))),
            Ok(Price::from_minor_units(2_900_000).unwrap())
        );
        assert_eq!(limit_price(None), Err(RejectReason::BadOrderParams));
        assert_eq!(
            limit_price(Some(Uint128::from(0u128))),
            Err(RejectReason::BadPrice)
        );
    }

    #[test]
    fn only_the_base_arm_is_a_limit_amount() {
        let base = |v: u128| Some(Amount::BaseQuantity(Uint128::from(v)));
        let quote = |v: u128| Some(Amount::QuoteQuantity(Uint128::from(v)));

        assert_eq!(base_amount(base(50)), Ok(BaseQuantity::from_minor_units(50)));
        assert_eq!(base_amount(base(0)), Err(RejectReason::BadQuantity));
        assert_eq!(base_amount(quote(50)), Err(RejectReason::BadOrderParams));
        assert_eq!(base_amount(None), Err(RejectReason::BadOrderParams));
    }
}
