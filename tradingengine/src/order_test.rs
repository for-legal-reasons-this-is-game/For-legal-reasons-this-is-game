use super::*;
use crate::ONE;

#[test]
fn opposite_flips_the_side() {
    assert_eq!(Side::Buy.opposite(), Side::Sell);
    assert_eq!(Side::Sell.opposite(), Side::Buy);
}

#[test]
fn opposite_applied_twice_is_the_original() {
    for side in [Side::Buy, Side::Sell] {
        assert_eq!(side.opposite().opposite(), side);
    }
}

#[test]
fn a_limit_order_carries_its_price() {
    let price = Price::from_minor_units(29_000 * ONE).expect("valid price");
    let kind = OrderKind::Limit { price };

    match kind {
        OrderKind::Limit { price: found } => assert_eq!(found, price),
        OrderKind::Market => panic!("constructed a Limit, matched a Market"),
    }
}

#[test]
fn a_market_order_has_nowhere_to_put_a_price() {
    // there is no assertion to make here beyond the one the compiler already
    // makes: OrderKind::Market takes no fields, so no limit price can be
    // attached to it and none has to be checked for at runtime
    let kind = OrderKind::Market;
    assert_ne!(
        kind,
        OrderKind::Limit {
            price: Price::from_minor_units(ONE).expect("valid price"),
        }
    );
}

#[test]
fn limit_orders_differ_by_price() {
    let cheap = OrderKind::Limit {
        price: Price::from_minor_units(29_000 * ONE).expect("valid price"),
    };
    let dear = OrderKind::Limit {
        price: Price::from_minor_units(29_001 * ONE).expect("valid price"),
    };

    assert_ne!(cheap, dear);
    assert_eq!(cheap, cheap);
}

#[test]
fn terminal_statuses_are_the_three_that_end_an_order() {
    assert!(OrderStatus::Filled.is_terminal());
    assert!(OrderStatus::Cancelled.is_terminal());
    assert!(OrderStatus::Rejected.is_terminal());
}

#[test]
fn live_statuses_are_not_terminal() {
    assert!(!OrderStatus::New.is_terminal());
    assert!(!OrderStatus::Open.is_terminal());
    assert!(!OrderStatus::PartiallyFilled.is_terminal());
}

#[test]
fn time_in_force_variants_are_distinct() {
    let all = [
        TimeInForce::GoodTilCancelled,
        TimeInForce::ImmediateOrCancel,
        TimeInForce::FillOrKill,
    ];

    for (index, one) in all.iter().enumerate() {
        for (other_index, other) in all.iter().enumerate() {
            assert_eq!(one == other, index == other_index);
        }
    }
}
