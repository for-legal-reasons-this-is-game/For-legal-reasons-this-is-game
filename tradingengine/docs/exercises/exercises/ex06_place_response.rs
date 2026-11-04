#![allow(unused)]
// TODO: replace every todo!() until `./check.sh` passes this file. Hints: HINTS.md
// ex06: build the PlaceOrderResponse
//
// Sprint: PR A, step 5, and the decision "OrderStatus::Open is reported as New".
// Read: DATA_MODEL.md "Responses" and "Layer 2: enums".
//
// - command_id is echoed on every response, accepted or not
// - a rejection sets reject_reason and a non-empty reject_detail, and leaves
//   status UNSPECIFIED and both filled amounts absent
// - an acceptance leaves reject_reason UNSPECIFIED and always sets both filled
//   amounts, zero included
// - the engine's Open has no proto value: it is New. Rejected never reaches a
//   response (a rejection creates no order), so it panics.

use proto::{Id, PlaceOrderResponse, RejectReason, Uint128};

use engine_sprint::types::Accepted;
use tradingengine::ids::CommandId;
use tradingengine::order::OrderStatus;

pub fn wire_status(status: OrderStatus) -> proto::OrderStatus {
    todo!()
}

pub fn place_response(
    command_id: CommandId,
    outcome: Result<Accepted, (RejectReason, String)>,
) -> PlaceOrderResponse {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tradingengine::quantity::{BaseQuantity, QuoteQuantity};

    fn accepted(status: OrderStatus, base: u128, quote: u128) -> Accepted {
        Accepted {
            status,
            filled_base: BaseQuantity::from_minor_units(base),
            filled_quote: QuoteQuantity::from_minor_units(quote),
        }
    }

    #[test]
    fn open_is_reported_as_new() {
        assert_eq!(wire_status(OrderStatus::Open), proto::OrderStatus::New);
        assert_eq!(wire_status(OrderStatus::New), proto::OrderStatus::New);
        assert_eq!(wire_status(OrderStatus::Filled), proto::OrderStatus::Filled);
    }

    #[test]
    #[should_panic]
    fn rejected_is_not_a_status() {
        wire_status(OrderStatus::Rejected);
    }

    #[test]
    fn a_fill_is_reported_with_its_amounts() {
        let response = place_response(
            CommandId::new(42),
            Ok(accepted(OrderStatus::Filled, 50_000_000, 1_450_000)),
        );
        assert_eq!(response.command_id, Some(Id::from(42u128)));
        assert_eq!(response.reject_reason(), RejectReason::Unspecified);
        assert_eq!(response.status(), proto::OrderStatus::Filled);
        assert_eq!(response.filled_base_quantity, Some(Uint128::from(50_000_000u128)));
        assert_eq!(response.filled_quote_quantity, Some(Uint128::from(1_450_000u128)));
    }

    #[test]
    fn a_resting_order_reports_zero_not_absent() {
        let response = place_response(CommandId::new(1), Ok(accepted(OrderStatus::Open, 0, 0)));
        assert_eq!(response.status(), proto::OrderStatus::New);
        assert_eq!(response.filled_base_quantity, Some(Uint128::from(0u128)));
        assert_eq!(response.filled_quote_quantity, Some(Uint128::from(0u128)));
    }

    #[test]
    fn a_rejection_has_a_reason_and_nothing_else() {
        let response = place_response(
            CommandId::new(7),
            Err((RejectReason::BadPrice, "price must be greater than zero".into())),
        );
        assert_eq!(response.command_id, Some(Id::from(7u128)));
        assert_eq!(response.reject_reason(), RejectReason::BadPrice);
        assert!(!response.reject_detail.is_empty());
        assert_eq!(response.status(), proto::OrderStatus::Unspecified);
        assert_eq!(response.filled_base_quantity, None);
        assert_eq!(response.filled_quote_quantity, None);
    }
}
