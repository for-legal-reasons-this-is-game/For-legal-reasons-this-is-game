use proto::{Id, Market};
use tradingengine::ended::EndedOrder;
use tradingengine::ids::{CommandId, OrderId, UserId};
use tradingengine::market::MarketInfo;
use tradingengine::order::{OrderStatus, Side};
use tradingengine::price::Price;
use tradingengine::quantity::{BaseQuantity, QuoteQuantity};
use tradingengine::resting_order::RestingOrder;
use tradingengine::state::{self, StateError};

/// Ids from the backend's `tb::id()` use all 128 bits, so the saved ones must keep them.
const BIG: u128 = 1 << 120;

/// BTC-USD in satoshis and cents, with prices in cents.
fn btc_usd() -> Market {
    Market {
        symbol: "BTC-USD".to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals: 2,
        fee_account: Some(Id::from(BIG + 1)),
    }
}

fn id(n: u128) -> OrderId {
    OrderId::new(BIG + n)
}

/// Order `n`. Each order has its own account holder, so no two orders belong to one user.
fn order(n: u128, side: Side, price: u128, quantity: u128) -> RestingOrder {
    RestingOrder::new(
        id(n),
        UserId::new(BIG + 1_000 + n),
        side,
        Price::from_minor_units(price).expect("valid price"),
        BaseQuantity::from_minor_units(quantity),
        CommandId::new(BIG + 2_000 + n),
    )
    .expect("valid order")
}

/// Saves a market the way the store does, and reads it back.
fn save_and_restore(market: &MarketInfo) -> MarketInfo {
    let definition = state::market_json(market.market()).expect("saves");
    let book = state::book_json(market).expect("saves");
    let ended: Vec<String> = market
        .ended()
        .iter()
        .map(|(id, ended)| state::ended_json(id, ended).expect("saves"))
        .collect();
    state::restore_market(&definition, Some(&book), &ended).expect("loads")
}

#[test]
fn a_saved_market_loads_back_the_same() {
    let mut market = MarketInfo::new(btc_usd()).expect("valid market");
    // both sides, several prices, and two orders at one price
    for (n, side, price) in [
        (1, Side::Buy, 2_899_000),
        (2, Side::Buy, 2_899_000),
        (3, Side::Buy, 2_898_000),
        (4, Side::Sell, 2_901_000),
        (5, Side::Sell, 2_902_000),
    ] {
        market
            .place(order(n, side, price, 50_000_000))
            .expect("rests");
    }
    // fills half of order 4, and all of order 6 the moment it arrives
    market
        .place(order(6, Side::Buy, 2_901_000, 25_000_000))
        .expect("fills");
    market.cancel(id(2)).expect("cancels");

    let restored = save_and_restore(&market);

    // all of it: the definition, every order with what it filled, the time priority, and
    // the ended orders in the order they ended
    assert_eq!(restored, market);
}

#[test]
fn time_priority_comes_back_with_the_book() {
    let mut market = MarketInfo::new(btc_usd()).expect("valid market");
    for n in [3, 1, 2] {
        market
            .place(order(n, Side::Buy, 2_899_000, 50_000_000))
            .expect("rests");
    }

    let restored = save_and_restore(&market);

    let ids: Vec<OrderId> = restored.book().bids().map(|order| order.id()).collect();
    assert_eq!(ids, vec![id(3), id(1), id(2)]);
}

#[test]
fn state_saved_in_this_format_keeps_loading() {
    // this is what the engine writes to Redis. If one of these has to change, the state that
    // is already saved can't load anymore
    let definition = r#"{"symbol":"BTC-USD","base_decimals":8,"quote_decimals":2,"price_decimals":2,"fee_account":1329227995784915872903807060280344577}"#;
    let book = r#"[{"id":7,"user_id":8,"side":"buy","price":2900000,"qty_total":50000000,"qty_remaining":25000000,"filled_quote":725000,"command_id":9}]"#;
    let ended = [
        r#"{"id":5,"owner":6,"status":"filled"}"#.to_string(),
        r#"{"id":3,"owner":4,"status":"cancelled"}"#.to_string(),
    ];

    let market = state::restore_market(definition, Some(book), &ended).expect("loads");

    assert_eq!(market.market(), &btc_usd());
    let resting = market.book().get_by_id(OrderId::new(7)).expect("rests");
    assert_eq!(
        resting.qty_remaining(),
        BaseQuantity::from_minor_units(25_000_000)
    );
    assert_eq!(
        resting.filled_quote(),
        QuoteQuantity::from_minor_units(725_000)
    );
    assert_eq!(resting.status(), OrderStatus::PartiallyFilled);
    assert_eq!(
        market.ended().get(OrderId::new(5)),
        Some(&EndedOrder {
            owner: UserId::new(6),
            status: OrderStatus::Filled,
        })
    );
    assert_eq!(market.ended().newest(), Some(OrderId::new(3)));

    // and the engine still writes exactly this
    assert_eq!(
        state::market_json(market.market()).expect("saves"),
        definition
    );
    assert_eq!(state::book_json(&market).expect("saves"), book);
    let written: Vec<String> = market
        .ended()
        .iter()
        .map(|(id, ended)| state::ended_json(id, ended).expect("saves"))
        .collect();
    assert_eq!(written, ended);
}

// ---- saved state the engine can't be in is refused ----

/// One resting order as it is saved, to break one rule at a time.
fn saved_order(id: u128, side: &str, price: u128, total: u128, remaining: u128) -> String {
    format!(
        r#"{{"id":{id},"user_id":{},"side":"{side}","price":{price},"qty_total":{total},"qty_remaining":{remaining},"filled_quote":0,"command_id":{}}}"#,
        100 + id,
        200 + id
    )
}

fn load(book: &str, ended: &[String]) -> Result<MarketInfo, StateError> {
    let definition = state::market_json(&btc_usd()).expect("saves");
    state::restore_market(&definition, Some(book), ended)
}

fn is_invalid(result: Result<MarketInfo, StateError>) -> bool {
    matches!(result, Err(StateError::Invalid(_)))
}

#[test]
fn a_book_that_crosses_is_refused() {
    let book = format!(
        "[{},{}]",
        saved_order(1, "buy", 2_901_000, 10, 10),
        saved_order(2, "sell", 2_900_000, 10, 10)
    );

    assert!(is_invalid(load(&book, &[])));
}

#[test]
fn an_order_resting_with_nothing_left_is_refused() {
    let book = format!("[{}]", saved_order(1, "buy", 2_900_000, 10, 0));

    assert!(is_invalid(load(&book, &[])));
}

#[test]
fn an_order_with_more_left_than_its_total_is_refused() {
    let book = format!("[{}]", saved_order(1, "buy", 2_900_000, 10, 11));

    assert!(is_invalid(load(&book, &[])));
}

#[test]
fn an_order_that_rests_and_has_ended_is_refused() {
    let book = format!("[{}]", saved_order(1, "buy", 2_900_000, 10, 10));
    let ended = [r#"{"id":1,"owner":101,"status":"cancelled"}"#.to_string()];

    assert!(is_invalid(load(&book, &ended)));
}

#[test]
fn an_ended_order_that_has_not_ended_is_refused() {
    let ended = [r#"{"id":1,"owner":101,"status":"open"}"#.to_string()];

    assert!(is_invalid(load("[]", &ended)));
}

#[test]
fn a_value_in_another_format_is_refused() {
    assert!(matches!(load("not json", &[]), Err(StateError::Json(_))));

    // a side the engine doesn't have
    let book = format!("[{}]", saved_order(1, "up", 2_900_000, 10, 10));
    assert!(matches!(load(&book, &[]), Err(StateError::Json(_))));
}
