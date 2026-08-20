use tradingengine::ONE;
use tradingengine::book::Book;
use tradingengine::error::EngineError;
use tradingengine::ids::{IdempotencyKey, OrderId, SeqNo, UserId};
use tradingengine::order::{OrderStatus, Side};
use tradingengine::price::Price;
use tradingengine::quantity::Quantity;
use tradingengine::resting_order::RestingOrder;

fn order(id: u64, side: Side, whole_price: i64) -> RestingOrder {
    RestingOrder::new(
        OrderId::new(id),
        UserId::new(u128::from(id)),
        side,
        Price::from_minor_units(whole_price * ONE).expect("valid price"),
        Quantity::from_minor_units(10).expect("valid quantity"),
        SeqNo::new(id),
        IdempotencyKey::new(format!("order-{id}")).expect("valid idempotency key"),
    )
    .expect("valid resting order")
}

fn ids<'a>(orders: impl Iterator<Item = &'a RestingOrder>) -> Vec<u64> {
    orders.map(|resting| resting.id().value()).collect()
}

fn price(whole: i64) -> Price {
    Price::from_minor_units(whole * ONE).expect("valid price")
}

#[test]
fn an_empty_book_has_no_top_of_book() {
    let book = Book::new();

    assert!(book.is_empty());
    assert_eq!(book.len(), 0);
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
}

#[test]
fn insert_puts_an_order_where_get_can_find_it() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");

    let found = book
        .get_by_id(OrderId::new(1))
        .expect("the order was just inserted");
    assert_eq!(found.id(), OrderId::new(1));
    assert_eq!(found.price(), price(29_000));
    assert_eq!(book.len(), 1);
    assert!(!book.is_empty());
}

#[test]
fn get_returns_nothing_for_an_unknown_id() {
    let book = Book::new();
    assert!(book.get_by_id(OrderId::new(99)).is_none());
}

#[test]
fn best_bid_is_the_highest_and_best_ask_the_lowest() {
    let mut book = Book::new();
    // inserted out of order on purpose; the map is what sorts them
    for (id, side, whole_price) in [
        (1, Side::Buy, 29_000),
        (2, Side::Buy, 29_002),
        (3, Side::Buy, 29_001),
        (4, Side::Sell, 29_010),
        (5, Side::Sell, 29_008),
        (6, Side::Sell, 29_009),
    ] {
        book.insert(order(id, side, whole_price))
            .expect("valid insert");
    }

    assert_eq!(book.best_bid(), Some(price(29_002)));
    assert_eq!(book.best_ask(), Some(price(29_008)));
}

#[test]
fn bids_read_highest_price_first_and_asks_lowest_first() {
    let mut book = Book::new();
    for (id, side, whole_price) in [
        (1, Side::Buy, 29_000),
        (2, Side::Buy, 29_002),
        (3, Side::Buy, 29_001),
        (4, Side::Sell, 29_010),
        (5, Side::Sell, 29_008),
        (6, Side::Sell, 29_009),
    ] {
        book.insert(order(id, side, whole_price))
            .expect("valid insert");
    }

    let bid_prices: Vec<i64> = book.bids().map(|o| o.price().minor_units() / ONE).collect();
    let ask_prices: Vec<i64> = book.asks().map(|o| o.price().minor_units() / ONE).collect();

    assert_eq!(bid_prices, vec![29_002, 29_001, 29_000]);
    assert_eq!(ask_prices, vec![29_008, 29_009, 29_010]);
}

#[test]
fn orders_at_one_price_read_oldest_first() {
    let mut book = Book::new();
    for id in [1, 2, 3] {
        book.insert(order(id, Side::Buy, 29_000))
            .expect("valid insert");
    }

    let order_ids = ids(book.bids());
    assert_eq!(order_ids, vec![1, 2, 3]);
}

#[test]
fn time_priority_holds_across_price_levels() {
    let mut book = Book::new();
    // two at the better price, two at the worse one, interleaved on insert
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");
    book.insert(order(2, Side::Buy, 29_001))
        .expect("valid insert");
    book.insert(order(3, Side::Buy, 29_000))
        .expect("valid insert");
    book.insert(order(4, Side::Buy, 29_001))
        .expect("valid insert");

    let order_ids = ids(book.bids());
    // price first: both 29_001 orders, oldest first; then both 29_000 orders
    assert_eq!(order_ids, vec![2, 4, 1, 3]);
}

#[test]
fn the_two_sides_hold_the_same_price_only_when_it_does_not_cross() {
    let mut book = Book::new();
    // a bid below every ask: the two sides coexist because nothing crosses
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");
    book.insert(order(2, Side::Sell, 29_001))
        .expect("valid insert");

    assert_eq!(book.best_bid(), Some(price(29_000)));
    assert_eq!(book.best_ask(), Some(price(29_001)));
    assert_eq!(book.bids().count(), 1);
    assert_eq!(book.asks().count(), 1);
    assert_eq!(book.len(), 2);
}

#[test]
fn remove_returns_the_order_and_takes_it_out() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");

    let removed = book
        .remove(OrderId::new(1))
        .expect("the order is in the book");

    assert_eq!(removed.id(), OrderId::new(1));
    assert!(book.is_empty());
    assert_eq!(book.best_bid(), None);
    assert!(book.get_by_id(OrderId::new(1)).is_none());
}

#[test]
fn removing_the_last_order_at_a_price_drops_the_level() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_001))
        .expect("valid insert");
    book.insert(order(2, Side::Buy, 29_000))
        .expect("valid insert");

    book.remove(OrderId::new(1))
        .expect("the order is in the book");

    // if the emptied level were left behind, best_bid would still read 29_001
    assert_eq!(book.best_bid(), Some(price(29_000)));
    assert_eq!(book.len(), 1);
}

#[test]
fn removing_one_order_leaves_the_rest_of_its_level() {
    let mut book = Book::new();
    for id in [1, 2, 3] {
        book.insert(order(id, Side::Buy, 29_000))
            .expect("valid insert");
    }

    book.remove(OrderId::new(2))
        .expect("the order is in the book");

    let order_ids = ids(book.bids());
    assert_eq!(order_ids, vec![1, 3]);
    assert_eq!(book.best_bid(), Some(price(29_000)));
}

#[test]
fn remove_rejects_an_unknown_id() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");

    assert_eq!(
        book.remove(OrderId::new(99)),
        Err(EngineError::OrderNotFound)
    );
    assert_eq!(book.len(), 1);
}

#[test]
fn remove_twice_rejects_the_second_time() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");

    book.remove(OrderId::new(1))
        .expect("the order is in the book");
    assert_eq!(
        book.remove(OrderId::new(1)),
        Err(EngineError::OrderNotFound)
    );
}

#[test]
fn insert_rejects_a_duplicate_id_without_changing_the_book() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");
    let before = book.clone();

    assert_eq!(
        book.insert(order(1, Side::Sell, 30_000)),
        Err(EngineError::DuplicateOrderId)
    );
    assert_eq!(book, before);
}

#[test]
fn remove_returns_the_order_without_touching_its_status() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");

    let removed = book
        .remove(OrderId::new(1))
        .expect("the order is in the book");

    // the book removes; the caller decides what the removal means and calls
    // RestingOrder::cancel to record it
    assert_eq!(removed.status(), OrderStatus::Open);
}

#[test]
fn amend_replaces_the_order_and_returns_the_old_one() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");

    let (replaced, _) = book
        .amend(OrderId::new(1), order(2, Side::Buy, 29_000))
        .expect("the order is in the book");

    assert_eq!(replaced.id(), OrderId::new(1));
    assert!(book.get_by_id(OrderId::new(1)).is_none());
    assert_eq!(ids(book.bids()), vec![2]);
    assert_eq!(book.len(), 1);
}

#[test]
fn amend_sends_the_replacement_to_the_back_of_its_level() {
    let mut book = Book::new();
    for id in [1, 2, 3] {
        book.insert(order(id, Side::Buy, 29_000))
            .expect("valid insert");
    }

    // order 1 was at the front of the queue; amending forfeits that
    book.amend(OrderId::new(1), order(4, Side::Buy, 29_000))
        .expect("the order is in the book");

    assert_eq!(ids(book.bids()), vec![2, 3, 4]);
}

#[test]
fn amend_can_move_the_order_to_another_price() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");

    book.amend(OrderId::new(1), order(2, Side::Buy, 29_005))
        .expect("the order is in the book");

    // the emptied level went with it
    assert_eq!(book.best_bid(), Some(price(29_005)));
    assert_eq!(book.len(), 1);
}

#[test]
fn amend_may_reuse_the_id_it_is_replacing() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");
    book.insert(order(2, Side::Buy, 29_000))
        .expect("valid insert");

    book.amend(OrderId::new(1), order(1, Side::Buy, 29_000))
        .expect("the order is in the book");

    // same id, but it still goes to the back
    assert_eq!(ids(book.bids()), vec![2, 1]);
}

#[test]
fn amend_rejects_an_unknown_id_without_changing_the_book() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");
    let before = book.clone();

    assert_eq!(
        book.amend(OrderId::new(99), order(2, Side::Buy, 29_000)),
        Err(EngineError::OrderNotFound)
    );
    assert_eq!(book, before);
}

#[test]
fn amend_rejects_a_replacement_id_already_in_the_book() {
    let mut book = Book::new();
    book.insert(order(1, Side::Buy, 29_000))
        .expect("valid insert");
    book.insert(order(2, Side::Buy, 29_000))
        .expect("valid insert");
    let before = book.clone();

    // without the pre-check, order 1 would already be gone by the time the
    // insert bounced
    assert_eq!(
        book.amend(OrderId::new(1), order(2, Side::Buy, 29_000)),
        Err(EngineError::DuplicateOrderId)
    );
    assert_eq!(book, before);
}

// ---- matching ----

fn sized(id: u64, side: Side, whole_price: i64, quantity: i64) -> RestingOrder {
    RestingOrder::new(
        OrderId::new(id),
        UserId::new(u128::from(id)),
        side,
        Price::from_minor_units(whole_price * ONE).expect("valid price"),
        Quantity::from_minor_units(quantity).expect("valid quantity"),
        SeqNo::new(id),
        IdempotencyKey::new(format!("order-{id}")).expect("valid idempotency key"),
    )
    .expect("valid resting order")
}

fn qty(minor_units: i64) -> Quantity {
    Quantity::from_minor_units(minor_units).expect("valid quantity")
}

#[test]
fn an_order_that_does_not_cross_just_rests() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_010, 10))
        .expect("valid insert");

    let execution = book
        .insert(sized(2, Side::Buy, 29_000, 10))
        .expect("valid insert");

    assert!(execution.fills().is_empty());
    assert_eq!(execution.resting(), Some(OrderId::new(2)));
    assert_eq!(book.len(), 2);
}

#[test]
fn a_crossing_order_trades_instead_of_resting() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_000, 10))
        .expect("valid insert");

    let execution = book
        .insert(sized(2, Side::Buy, 29_000, 10))
        .expect("valid insert");

    assert_eq!(execution.fills().len(), 1);
    assert_eq!(execution.resting(), None);
    assert_eq!(execution.filled(), Ok(qty(10)));
    // both sides traded out completely, so the book is empty again
    assert!(book.is_empty());
}

#[test]
fn a_fill_happens_at_the_makers_price() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_000, 10))
        .expect("valid insert");

    // buyer would have paid 29_005; the resting seller's price is what stands,
    // so the price improvement goes to the taker
    let execution = book
        .insert(sized(2, Side::Buy, 29_005, 10))
        .expect("valid insert");

    assert_eq!(execution.fills()[0].price(), price(29_000));
    assert_eq!(execution.fills()[0].maker(), OrderId::new(1));
    assert_eq!(execution.fills()[0].taker(), OrderId::new(2));
}

#[test]
fn a_partial_fill_leaves_the_remainder_resting() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_000, 4))
        .expect("valid insert");

    let execution = book
        .insert(sized(2, Side::Buy, 29_000, 10))
        .expect("valid insert");

    assert_eq!(execution.filled(), Ok(qty(4)));
    assert_eq!(execution.resting(), Some(OrderId::new(2)));

    let rested = book
        .get_by_id(OrderId::new(2))
        .expect("the remainder rests");
    assert_eq!(rested.qty_remaining(), qty(6));
    assert_eq!(rested.status(), OrderStatus::PartiallyFilled);
    // the maker was consumed and is gone
    assert!(book.get_by_id(OrderId::new(1)).is_none());
    assert_eq!(book.best_ask(), None);
}

#[test]
fn a_maker_that_is_not_exhausted_stays_with_less_left() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_000, 10))
        .expect("valid insert");

    let execution = book
        .insert(sized(2, Side::Buy, 29_000, 4))
        .expect("valid insert");

    assert_eq!(execution.resting(), None);

    let maker = book.get_by_id(OrderId::new(1)).expect("the maker remains");
    assert_eq!(maker.qty_remaining(), qty(6));
    assert_eq!(maker.status(), OrderStatus::PartiallyFilled);
    assert_eq!(book.len(), 1);
}

#[test]
fn a_taker_walks_levels_cheapest_first() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_002, 5))
        .expect("valid insert");
    book.insert(sized(2, Side::Sell, 29_000, 5))
        .expect("valid insert");
    book.insert(sized(3, Side::Sell, 29_001, 5))
        .expect("valid insert");

    let execution = book
        .insert(sized(4, Side::Buy, 29_002, 15))
        .expect("valid insert");

    let prices: Vec<i64> = execution
        .fills()
        .iter()
        .map(|fill| fill.price().minor_units() / ONE)
        .collect();
    assert_eq!(prices, vec![29_000, 29_001, 29_002]);
    assert!(book.is_empty());
}

#[test]
fn a_taker_stops_at_the_limit_price() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_000, 5))
        .expect("valid insert");
    book.insert(sized(2, Side::Sell, 29_005, 5))
        .expect("valid insert");

    let execution = book
        .insert(sized(3, Side::Buy, 29_000, 10))
        .expect("valid insert");

    // only the 29_000 ask is acceptable; the rest of the buy rests
    assert_eq!(execution.filled(), Ok(qty(5)));
    assert_eq!(execution.resting(), Some(OrderId::new(3)));
    assert_eq!(book.best_bid(), Some(price(29_000)));
    assert_eq!(book.best_ask(), Some(price(29_005)));
}

#[test]
fn fills_take_the_oldest_order_at_a_price_first() {
    let mut book = Book::new();
    for id in [1, 2, 3] {
        book.insert(sized(id, Side::Sell, 29_000, 5))
            .expect("valid insert");
    }

    let execution = book
        .insert(sized(4, Side::Buy, 29_000, 10))
        .expect("valid insert");

    let makers: Vec<u64> = execution
        .fills()
        .iter()
        .map(|fill| fill.maker().value())
        .collect();
    assert_eq!(makers, vec![1, 2]);
    assert_eq!(ids(book.asks()), vec![3]);
}

#[test]
fn a_selling_taker_walks_the_bids_highest_first() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Buy, 29_000, 5))
        .expect("valid insert");
    book.insert(sized(2, Side::Buy, 29_002, 5))
        .expect("valid insert");

    let execution = book
        .insert(sized(3, Side::Sell, 29_000, 10))
        .expect("valid insert");

    let prices: Vec<i64> = execution
        .fills()
        .iter()
        .map(|fill| fill.price().minor_units() / ONE)
        .collect();
    assert_eq!(prices, vec![29_002, 29_000]);
    assert!(book.is_empty());
}

#[test]
fn an_amended_order_matches_like_a_new_arrival() {
    let mut book = Book::new();
    book.insert(sized(1, Side::Sell, 29_000, 10))
        .expect("valid insert");
    book.insert(sized(2, Side::Buy, 28_000, 10))
        .expect("valid insert");

    // repricing the bid up to the ask makes it cross
    let (replaced, execution) = book
        .amend(OrderId::new(2), sized(3, Side::Buy, 29_000, 10))
        .expect("the order is in the book");

    assert_eq!(replaced.id(), OrderId::new(2));
    assert_eq!(execution.filled(), Ok(qty(10)));
    assert_eq!(execution.resting(), None);
    assert!(book.is_empty());
}
