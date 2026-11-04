#![allow(unused)]
// ex05: EngineError to RejectReason, and panic on the rest
//
// Sprint: PR A, error mapping. Read: DATA_MODEL.md "Errors with no proto
// meaning (decided)" and the INTERNAL_ERROR comment in proto/common.proto.
//
// Rules:
// - list every EngineError variant by name, no `_ =>` arm, so a new variant
//   forces a decision instead of falling into a default
// - an error with no proto meaning panics (the pod has panic = "abort")
// - INTERNAL_ERROR is NOT a mapping target. The contract keeps it for one
//   case only: a command that was still queued when the engine stopped, so
//   it was never processed. Overflow stays BAD_QUANTITY.
// - DuplicateOrderId is still open (UNSUPPORTED, or a panic because the backend
//   never reuses an order_id). The tests don't check it: pick one and say so
//   in the PR.

use proto::RejectReason;
use tradingengine::error::EngineError;

pub fn reject_reason(error: &EngineError) -> RejectReason {
    //>>
    match error {
        EngineError::RequiredFieldAbsent | EngineError::TimestampOutOfRange => {
            RejectReason::BadOrderParams
        }
        EngineError::QuantityNotPositive
        | EngineError::Overflow
        | EngineError::QuantityNotAboveFilled => RejectReason::BadQuantity,
        EngineError::PriceNotPositive => RejectReason::BadPrice,
        EngineError::OrderNotFound => RejectReason::UnknownOrder,
        EngineError::OrderNotLive => RejectReason::OrderNotWorking,
        // open, see the header. Chosen here: the backend owns order ids and
        // never reuses one, so a duplicate is a broken invariant
        EngineError::DuplicateOrderId
        | EngineError::QuantityNegative
        | EngineError::FillExceedsRemaining
        | EngineError::DecimalsOutOfRange
        | EngineError::MarketSymbolEmpty
        | EngineError::HasOpenOrders => {
            panic!("{error}: no RejectReason for this, the engine is in a broken state")
        }
    }
    //<<
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECIDED: [(EngineError, RejectReason); 8] = [
        (EngineError::RequiredFieldAbsent, RejectReason::BadOrderParams),
        (EngineError::TimestampOutOfRange, RejectReason::BadOrderParams),
        (EngineError::QuantityNotPositive, RejectReason::BadQuantity),
        (EngineError::Overflow, RejectReason::BadQuantity),
        (EngineError::QuantityNotAboveFilled, RejectReason::BadQuantity),
        (EngineError::PriceNotPositive, RejectReason::BadPrice),
        (EngineError::OrderNotFound, RejectReason::UnknownOrder),
        (EngineError::OrderNotLive, RejectReason::OrderNotWorking),
    ];

    #[test]
    fn contract_errors_map_to_their_reason() {
        for (error, reason) in DECIDED {
            assert_eq!(reject_reason(&error), reason, "{error:?}");
        }
    }

    #[test]
    fn overflow_is_not_an_internal_error() {
        assert_eq!(reject_reason(&EngineError::Overflow), RejectReason::BadQuantity);
    }

    #[test]
    fn no_error_maps_to_internal_error_or_accepted() {
        for (error, _) in DECIDED {
            let reason = reject_reason(&error);
            assert_ne!(reason, RejectReason::InternalError, "{error:?}");
            assert_ne!(reason, RejectReason::Unspecified, "{error:?}");
        }
    }

    #[test]
    #[should_panic]
    fn a_broken_invariant_panics() {
        reject_reason(&EngineError::FillExceedsRemaining);
    }

    #[test]
    #[should_panic]
    fn a_negative_quantity_panics() {
        reject_reason(&EngineError::QuantityNegative);
    }

    #[test]
    #[should_panic]
    fn a_config_error_is_not_a_command_rejection() {
        reject_reason(&EngineError::HasOpenOrders);
    }
}
