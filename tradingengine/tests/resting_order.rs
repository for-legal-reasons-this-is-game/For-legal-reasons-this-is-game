use tradingengine::ONE;
use tradingengine::error::EngineError;
use tradingengine::ids::{IdempotencyKey, OrderId, SeqNo, UserId};
use tradingengine::order::{OrderStatus, Side};
use tradingengine::price::Price;
use tradingengine::quantity::Quantity;
use tradingengine::resting_order::RestingOrder;

fn qty(minor_units: i64) -> Quantity {
    Quantity::from_minor_units(minor_units).expect("valid quantity")
}

fn order(quantity: Quantity) -> RestingOrder {
    RestingOrder::new(
        OrderId::new(11),
        UserId::new(22),
        Side::Buy,
        Price::from_minor_units(29_000 * ONE).expect("valid price"),
        quantity,
        SeqNo::new(33),
        IdempotencyKey::new("order-11".to_owned()).expect("valid idempotency key"),
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
    assert_eq!(order.qty_original(), original);
    assert_eq!(order.qty_remaining(), original);
    assert_eq!(order.status(), OrderStatus::Open);
    assert_eq!(order.seq_no(), SeqNo::new(33));
    assert_eq!(order.idempotency_key().as_str(), "order-11");
}

#[test]
fn rejects_an_order_with_zero_original_quantity() {
    assert_eq!(
        RestingOrder::new(
            OrderId::new(11),
            UserId::new(22),
            Side::Buy,
            Price::from_minor_units(ONE).expect("valid price"),
            Quantity::ZERO,
            SeqNo::new(33),
            IdempotencyKey::new("order-11".to_owned()).expect("valid idempotency key"),
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
    assert_eq!(order.qty_remaining(), Quantity::ZERO);
    assert_eq!(order.status(), OrderStatus::Filled);
}

#[test]
fn consecutive_fills_preserve_the_original_quantity() {
    let mut order = order(qty(10));

    order.fill(qty(4)).expect("valid partial fill");
    order.fill(qty(6)).expect("valid final fill");

    assert_eq!(order.qty_original(), qty(10));
    assert_eq!(order.qty_remaining(), Quantity::ZERO);
    assert_eq!(order.status(), OrderStatus::Filled);
}

#[test]
fn rejects_zero_fill_without_changing_the_order() {
    let mut order = order(qty(10));
    let before = order.clone();

    assert_eq!(
        order.fill(Quantity::ZERO),
        Err(EngineError::QuantityNotPositive)
    );
    assert_eq!(order, before);
}

#[test]
fn rejects_overfill_without_changing_the_order() {
    let mut order = order(qty(10));
    let before = order.clone();

    assert_eq!(order.fill(qty(11)), Err(EngineError::FillExceedsRemaining));
    assert_eq!(order, before);
}
