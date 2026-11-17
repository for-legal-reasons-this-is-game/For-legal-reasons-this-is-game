//! The command cache, as `proto/how_to_use.md` describes it in "Handling a command":
//! - a known `command_id` with the same request gets the stored response, and nothing runs again
//! - a known `command_id` with a different request is rejected with `DUPLICATE_COMMAND`
//! - a new `command_id` runs, and its response is stored, rejections included

use proto::place_order_request::Amount;
use proto::{
    CancelOrderRequest, Id, Market, OrderStatus, OrderType, PlaceOrderRequest, RejectReason,
    SetMarketRequest, Side, TimeInForce, Uint128,
};
use tradingengine::engine::Engine;
use tradingengine::ids::OrderId;

// BTC-USD: base in satoshis (8), quote in cents (2), price in cents (2)
const HALF_BTC: u128 = 50_000_000;
const PRICE_29_000: u128 = 2_900_000;

fn btc_usd() -> Market {
    Market {
        symbol: "BTC-USD".to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals: 2,
        fee_account: Some(Id::from(1u128)),
    }
}

/// An engine with the BTC-USD market set.
fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_market(SetMarketRequest {
        market: Some(btc_usd()),
    });
    engine
}

/// A valid GTC limit order. Every order gets its own order, command and holder id from `n`:
/// order `n` has `command_id` `1_000 + n`.
fn limit(n: u128, side: Side, price: u128, base_quantity: u128) -> PlaceOrderRequest {
    let mut request = PlaceOrderRequest {
        command_id: Some(Id::from(1_000 + n)),
        order_id: Some(Id::from(n)),
        market_symbol: "BTC-USD".to_string(),
        account_holder: Some(Id::from(2_000 + n)),
        base_account: Some(Id::from(3_000 + n)),
        quote_account: Some(Id::from(4_000 + n)),
        amount: Some(Amount::BaseQuantity(Uint128::from(base_quantity))),
        limit_price: Some(Uint128::from(price)),
        maker_fee_bps: 5,
        taker_fee_bps: 10,
        ..PlaceOrderRequest::default()
    };
    request.set_side(side);
    request.set_type(OrderType::Limit);
    request.set_time_in_force(TimeInForce::Gtc);
    request
}

/// A cancel of order `n` by its owner, with `command_id` `5_000 + n`.
fn cancel(n: u128) -> CancelOrderRequest {
    CancelOrderRequest {
        command_id: Some(Id::from(5_000 + n)),
        order_id: Some(Id::from(n)),
        account_holder: Some(Id::from(2_000 + n)),
        market_symbol: "BTC-USD".to_string(),
    }
}

/// Places an order and checks that it was accepted, so a broken place fails here.
fn place(engine: &mut Engine, request: PlaceOrderRequest) {
    let response = engine.place_order(request);
    assert_eq!(
        response.reject_reason(),
        RejectReason::Unspecified,
        "place rejected: {}",
        response.reject_detail
    );
}

/// Whether order `n` is in the BTC-USD book.
fn in_book(engine: &Engine, n: u128) -> bool {
    engine
        .market("BTC-USD")
        .expect("market exists")
        .book()
        .get_by_id(OrderId::new(n))
        .is_some()
}

// ---- same command_id, same request: the stored response ----

#[test]
fn a_retried_buy_gets_the_first_answer_and_does_not_fill_again() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));
    place(&mut engine, limit(2, Side::Sell, PRICE_29_000, HALF_BTC));
    let buy = limit(3, Side::Buy, PRICE_29_000, HALF_BTC);

    let first = engine.place_order(buy.clone());
    assert_eq!(
        first.status(),
        OrderStatus::Filled,
        "{}",
        first.reject_detail
    );

    let retry = engine.place_order(buy);

    assert_eq!(retry, first);
    // run again, the buy would also have filled against order 2
    assert!(in_book(&engine, 2));
}

#[test]
fn a_retried_cancel_gets_the_first_answer_although_the_order_is_gone() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let first = engine.cancel_order(cancel(1));
    assert_eq!(
        first.status(),
        OrderStatus::Cancelled,
        "{}",
        first.reject_detail
    );

    // run again, the cancel would be rejected: order 1 left the book with the first cancel
    assert_eq!(engine.cancel_order(cancel(1)), first);
}

#[test]
fn a_rejection_is_stored_too_so_its_retry_cannot_succeed_later() {
    let mut engine = Engine::new(); // no market yet
    let request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);

    let first = engine.place_order(request.clone());
    assert_eq!(first.reject_reason(), RejectReason::UnknownMarket);

    engine.set_market(SetMarketRequest {
        market: Some(btc_usd()),
    });
    let retry = engine.place_order(request);

    // the backend refunds an order when it gets a rejection, so the retry must not place it now
    assert_eq!(retry, first);
    assert!(!in_book(&engine, 1));
}

// ---- same command_id, different request: DUPLICATE_COMMAND ----

#[test]
fn a_place_that_reuses_a_command_id_is_duplicate_command_and_the_first_answer_stays() {
    let mut engine = engine();
    let first = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let mut other = limit(2, Side::Buy, PRICE_29_000, HALF_BTC);
    other.command_id = Some(Id::from(1_001)); // order 1's command_id
    let response = engine.place_order(other);

    assert_eq!(
        response.reject_reason(),
        RejectReason::DuplicateCommand,
        "{}",
        response.reject_detail
    );
    assert_eq!(response.command_id, Some(Id::from(1_001)));
    assert!(!in_book(&engine, 2));
    // the rejection didn't replace what is stored under 1_001
    assert_eq!(
        engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)),
        first
    );
}

#[test]
fn a_cancel_that_reuses_a_place_command_id_is_duplicate_command() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let mut request = cancel(1);
    request.command_id = Some(Id::from(1_001)); // the place's command_id
    let response = engine.cancel_order(request);

    assert_eq!(
        response.reject_reason(),
        RejectReason::DuplicateCommand,
        "{}",
        response.reject_detail
    );
    assert!(in_book(&engine, 1));
}

// ---- no command_id: rejected, and not stored, since there's no id to store it under ----

#[test]
fn a_place_without_a_command_id_is_bad_order_params() {
    let mut engine = engine();
    let mut request = limit(1, Side::Buy, PRICE_29_000, HALF_BTC);
    request.command_id = None;

    let response = engine.place_order(request);

    assert_eq!(
        response.reject_reason(),
        RejectReason::BadOrderParams,
        "{}",
        response.reject_detail
    );
    assert!(!in_book(&engine, 1));
}

#[test]
fn a_cancel_without_a_command_id_is_bad_order_params() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    let mut request = cancel(1);
    request.command_id = None;

    let response = engine.cancel_order(request);

    assert_eq!(
        response.reject_reason(),
        RejectReason::BadOrderParams,
        "{}",
        response.reject_detail
    );
    assert!(in_book(&engine, 1));
}

fn engine_with_cache_capacity(n: usize) -> Engine {
    let mut engine = Engine::with_cache_capacity(n);
    engine.set_market(SetMarketRequest {
        market: Some(btc_usd()),
    });
    engine
}

#[test]
fn oldest_cache_entry_is_dropped_and_a_newer_one_returns_stored_response() {
    let mut engine = engine_with_cache_capacity(5);
    let oldest = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    for n in 2..=5 {
        let _ = engine.place_order(limit(n, Side::Buy, PRICE_29_000, HALF_BTC));
    }
    let newest = engine.place_order(limit(6, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_ne!(
        engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)),
        oldest
    );
    assert_eq!(
        engine.place_order(limit(6, Side::Buy, PRICE_29_000, HALF_BTC)),
        newest
    );
}
