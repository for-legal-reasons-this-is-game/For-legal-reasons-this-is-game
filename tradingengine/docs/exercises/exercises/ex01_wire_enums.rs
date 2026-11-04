#![allow(unused)]
// TODO: replace every todo!() until `./check.sh` passes this file. Hints: HINTS.md
// ex01: enums off the wire
//
// Sprint: PR A, step 3 ("i32 to Side, OrderType, TimeInForce, rejecting
// UNSPECIFIED and unknown numbers"). Real code goes in src/wire.rs.
// Read: DATA_MODEL.md "Wire conventions" and "Layer 2: enums".
//
// prost hands enums over as i32. `proto::Side::try_from(raw)` turns one into
// the proto enum and fails on a number it does not know. UNSPECIFIED (0) is
// known to prost but is never a valid engine value.
// Both cases are BAD_ORDER_PARAMS (proposed in DATA_MODEL, confirm in the PR).

use proto::RejectReason;
use tradingengine::order::Side;

pub fn side_from_wire(raw: i32) -> Result<Side, RejectReason> {
    todo!()
}

pub fn order_type_from_wire(raw: i32) -> Result<proto::OrderType, RejectReason> {
    todo!()
}

pub fn time_in_force_from_wire(raw: i32) -> Result<proto::TimeInForce, RejectReason> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_sides_convert() {
        assert_eq!(side_from_wire(proto::Side::Buy as i32), Ok(Side::Buy));
        assert_eq!(side_from_wire(proto::Side::Sell as i32), Ok(Side::Sell));
    }

    #[test]
    fn unspecified_is_a_rejection_not_a_default() {
        assert_eq!(side_from_wire(0), Err(RejectReason::BadOrderParams));
        assert_eq!(order_type_from_wire(0), Err(RejectReason::BadOrderParams));
        assert_eq!(time_in_force_from_wire(0), Err(RejectReason::BadOrderParams));
    }

    #[test]
    fn unknown_numbers_are_rejected() {
        for raw in [-1, 3, 99, i32::MAX] {
            assert_eq!(side_from_wire(raw), Err(RejectReason::BadOrderParams));
        }
        assert_eq!(order_type_from_wire(5), Err(RejectReason::BadOrderParams));
        assert_eq!(time_in_force_from_wire(5), Err(RejectReason::BadOrderParams));
    }

    #[test]
    fn every_real_type_and_lifetime_converts() {
        use proto::{OrderType, TimeInForce};
        for t in [
            OrderType::Limit,
            OrderType::Market,
            OrderType::StopLimit,
            OrderType::StopMarket,
        ] {
            assert_eq!(order_type_from_wire(t as i32), Ok(t));
        }
        for t in [
            TimeInForce::Gtc,
            TimeInForce::Gtd,
            TimeInForce::Ioc,
            TimeInForce::Fok,
        ] {
            assert_eq!(time_in_force_from_wire(t as i32), Ok(t));
        }
    }
}
