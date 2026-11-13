use tradingengine::ended::{EndedOrder, EndedOrders};
use tradingengine::ids::{CommandId, OrderId, UserId};
use tradingengine::order::{OrderStatus, Side};
use tradingengine::price::Price;
use tradingengine::quantity::{BaseQuantity, QuoteQuantity};
use tradingengine::resting_order::RestingOrder;

fn owner(id: u128) -> UserId {
    UserId::new(100 + id)
}

/// Order `id` for 10 units, held by `owner(id)`. It is still open.
fn open(id: u128) -> RestingOrder {
    RestingOrder::new(
        OrderId::new(id),
        owner(id),
        Side::Buy,
        Price::from_minor_units(29_000).expect("valid price"),
        BaseQuantity::from_minor_units(10),
        CommandId::new(id),
    )
    .expect("valid resting order")
}

/// Order `id`, cancelled before anything of it filled.
fn cancelled(id: u128) -> RestingOrder {
    let mut order = open(id);
    order.cancel().expect("an open order can be cancelled");
    order
}

#[test]
fn nothing_has_ended_at_first() {
    let ended = EndedOrders::default();

    assert!(ended.is_empty());
    assert_eq!(ended.get(OrderId::new(1)), None);
}

#[test]
fn an_ended_order_is_remembered_with_its_owner_and_status() {
    let mut ended = EndedOrders::default();
    let mut filled = open(2);
    filled
        .fill(
            BaseQuantity::from_minor_units(10),
            QuoteQuantity::from_minor_units(290),
        )
        .expect("a fill of the whole order");

    ended.record(&cancelled(1));
    ended.record(&filled);

    assert_eq!(
        ended.get(OrderId::new(1)),
        Some(&EndedOrder {
            owner: owner(1),
            status: OrderStatus::Cancelled,
        })
    );
    assert_eq!(
        ended.get(OrderId::new(2)),
        Some(&EndedOrder {
            owner: owner(2),
            status: OrderStatus::Filled,
        })
    );
    assert!(!ended.contains(OrderId::new(3)));
}

#[test]
fn the_oldest_ended_order_is_forgotten_first() {
    let mut ended = EndedOrders::with_capacity(2);

    for id in [1, 2, 3] {
        ended.record(&cancelled(id));
    }

    assert_eq!(ended.len(), 2);
    assert!(!ended.contains(OrderId::new(1)));
    assert!(ended.contains(OrderId::new(2)));
    assert!(ended.contains(OrderId::new(3)));
}

#[test]
fn an_order_recorded_twice_takes_one_place() {
    let mut ended = EndedOrders::with_capacity(2);

    ended.record(&cancelled(1));
    ended.record(&cancelled(1));
    ended.record(&cancelled(2));

    // if order 1 had taken two places, recording order 2 would have pushed it out
    assert_eq!(ended.len(), 2);
    assert!(ended.contains(OrderId::new(1)));
    assert!(ended.contains(OrderId::new(2)));
}

#[test]
fn a_market_remembers_the_number_the_contract_promises() {
    // proto/how_to_use.md tells the backend how many ended orders each market remembers
    assert_eq!(EndedOrders::DEFAULT_CAPACITY, 100_000);
}
