// One whole unit at 8 decimals. A test fixture only: scale is per market now.
const ONE: u128 = 100_000_000;
use tradingengine::error::EngineError;
use tradingengine::ids::{CommandId, OrderId, UserId};
use tradingengine::order::{OrderStatus, Side};
use tradingengine::price::Price;
use tradingengine::quantity::BaseQuantity;
use tradingengine::resting_order::RestingOrder;

fn qty(minor_units: u128) -> BaseQuantity {
    BaseQuantity::from_minor_units(minor_units)
}

fn order(quantity: BaseQuantity) -> RestingOrder {
    RestingOrder::new(
        OrderId::new(11),
        UserId::new(22),
        Side::Buy,
        Price::from_minor_units(29_000 * ONE).expect("valid price"),
        quantity,
        CommandId::new(33),
    )
    .expect("valid resting order")
}

#[test]
fn new_order_is_open_with_its_whole_quantity_remaining() {
    let original = qty(10);
    let order = order(original);

    assert_eq!(order.id(), OrderId::new(11));
    assert_eq!(order.user_id(), UserId::new(22));
    assert_eq!(order.side(), Side::Buy);
    assert_eq!(order.price().minor_units(), 29_000 * ONE);
    assert_eq!(order.qty_total(), original);
    assert_eq!(order.qty_remaining(), original);
    assert_eq!(order.status(), OrderStatus::Open);
    assert_eq!(order.command_id(), CommandId::new(33));
}

#[test]
fn rejects_an_order_with_zero_original_quantity() {
    assert_eq!(
        RestingOrder::new(
            OrderId::new(11),
            UserId::new(22),
            Side::Buy,
            Price::from_minor_units(ONE).expect("valid price"),
            BaseQuantity::ZERO,
            CommandId::new(33),
        ),
        Err(EngineError::QuantityNotPositive)
    );
}

#[test]
fn partial_fill_moves_quantity_and_status_together() {
    let mut order = order(qty(10));

    assert_eq!(order.fill(qty(4)), Ok(()));
    assert_eq!(order.qty_remaining(), qty(6));
    assert_eq!(order.status(), OrderStatus::PartiallyFilled);
}

#[test]
fn exact_fill_moves_quantity_to_zero_and_status_to_filled() {
    let mut order = order(qty(10));

    assert_eq!(order.fill(qty(10)), Ok(()));
    assert_eq!(order.qty_remaining(), BaseQuantity::ZERO);
    assert_eq!(order.status(), OrderStatus::Filled);
}

#[test]
fn consecutive_fills_preserve_the_original_quantity() {
    let mut order = order(qty(10));

    order.fill(qty(4)).expect("valid partial fill");
    order.fill(qty(6)).expect("valid final fill");

    assert_eq!(order.qty_total(), qty(10));
    assert_eq!(order.qty_remaining(), BaseQuantity::ZERO);
    assert_eq!(order.status(), OrderStatus::Filled);
}

#[test]
fn rejects_zero_fill_without_changing_the_order() {
    let mut order = order(qty(10));
    let before = order.clone();

    assert_eq!(
        order.fill(BaseQuantity::ZERO),
        Err(EngineError::QuantityNotPositive)
    );
    assert_eq!(order, before);
}

#[test]
fn cancel_moves_an_open_order_to_cancelled() {
    let mut order = order(qty(10));

    assert_eq!(order.cancel(), Ok(()));
    assert_eq!(order.status(), OrderStatus::Cancelled);
}

#[test]
fn cancel_keeps_the_quantity_a_partially_filled_order_had_left() {
    let mut order = order(qty(10));
    order.fill(qty(4)).expect("valid partial fill");

    assert_eq!(order.cancel(), Ok(()));
    assert_eq!(order.status(), OrderStatus::Cancelled);
    assert_eq!(order.qty_remaining(), qty(6));
    assert_eq!(order.qty_total(), qty(10));
}

#[test]
fn cancel_rejects_a_filled_order_without_changing_it() {
    let mut order = order(qty(10));
    order.fill(qty(10)).expect("valid final fill");
    let before = order.clone();

    assert_eq!(order.cancel(), Err(EngineError::OrderNotLive));
    assert_eq!(order, before);
}

#[test]
fn cancel_twice_rejects_the_second_time() {
    let mut order = order(qty(10));
    order.cancel().expect("the order is open");

    assert_eq!(order.cancel(), Err(EngineError::OrderNotLive));
    assert_eq!(order.status(), OrderStatus::Cancelled);
}

#[test]
fn rejects_overfill_without_changing_the_order() {
    let mut order = order(qty(10));
    let before = order.clone();

    assert_eq!(order.fill(qty(11)), Err(EngineError::FillExceedsRemaining));
    assert_eq!(order, before);
}

#[test]
fn modify_sets_the_new_total_and_keeps_what_was_filled() {
    let mut order = order(qty(10));
    order.fill(qty(3)).expect("valid fill");

    order
        .modify(
            Some(qty(7)),
            Some(Price::from_minor_units(ONE).expect("valid price")),
        )
        .expect("7 is above the 3 filled");

    assert_eq!(order.qty_total(), qty(7));
    assert_eq!(order.qty_remaining(), qty(4));
    assert_eq!(
        order.price(),
        Price::from_minor_units(ONE).expect("valid price")
    );
    assert_eq!(order.status(), OrderStatus::PartiallyFilled);
}

#[test]
fn modify_rejects_a_total_not_above_filled_without_changing_the_order() {
    let mut order = order(qty(10));
    order.fill(qty(3)).expect("valid fill");
    let before = order.clone();

    assert_eq!(
        order.modify(Some(qty(3)), None),
        Err(EngineError::QuantityNotAboveFilled)
    );
    assert_eq!(order, before);
}

#[test]
fn modify_rejects_a_finished_order() {
    let mut order = order(qty(10));
    order.cancel().expect("an open order can be cancelled");

    assert_eq!(
        order.modify(Some(qty(5)), None),
        Err(EngineError::OrderNotLive)
    );
}
