mod common;

use common::{HALF_BTC, PRICE_29_000, cancel, engine, in_book, limit, owner, place};
use proto::{
    CancelOrderRequest, CancelOrderResponse, CancelReason, Id, OrderStatus, RejectReason, Side,
};
use tradingengine::engine::Engine;

// BTC-USD: base in satoshis (8), quote in cents (2), price in cents (2), so
// k = 2 + 8 - 2 = 8 and 0.25 BTC at $29,000 is 725_000 cents
const QUARTER_BTC: u128 = 25_000_000;
const QUARTER_BTC_AT_29_000: u128 = 725_000;

fn filled_base(response: &CancelOrderResponse) -> u128 {
    response.filled_base_quantity.map(u128::from).unwrap_or(0)
}

fn filled_quote(response: &CancelOrderResponse) -> u128 {
    response.filled_quote_quantity.map(u128::from).unwrap_or(0)
}

/// Sends a cancel and checks that it was accepted: the same `command_id` comes back, and the
/// order is reported as cancelled at its user's request.
fn assert_cancelled(engine: &mut Engine, request: CancelOrderRequest) -> CancelOrderResponse {
    let command_id = request.command_id;
    let response = engine.cancel_order(request);
    assert_eq!(
        response.reject_reason(),
        RejectReason::Unspecified,
        "rejected: {}",
        response.reject_detail
    );
    assert_eq!(response.command_id, command_id);
    assert_eq!(response.status(), OrderStatus::Cancelled);
    assert_eq!(response.cancel_reason(), CancelReason::UserRequest);
    response
}

/// Sends a cancel and checks that it was rejected with `reason`: it says why, the same
/// `command_id` comes back, and nothing in the markets changed.
fn assert_rejected(engine: &mut Engine, request: CancelOrderRequest, reason: RejectReason) {
    let before = engine.markets().clone();
    let command_id = request.command_id;
    let response = engine.cancel_order(request);
    assert_eq!(
        response.reject_reason(),
        reason,
        "{}",
        response.reject_detail
    );
    assert!(!response.reject_detail.is_empty());
    assert_eq!(response.command_id, command_id);
    assert_eq!(*engine.markets(), before);
}

// ---- accepted ----

#[test]
fn the_owner_can_cancel_a_resting_order() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let response = assert_cancelled(&mut engine, cancel(1));

    assert_eq!(filled_base(&response), 0);
    assert_eq!(filled_quote(&response), 0);
    assert!(!in_book(&engine, 1));
}

#[test]
fn a_cancelled_order_no_longer_trades() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));
    assert_cancelled(&mut engine, cancel(1));

    // without the cancel, this buy would fill against order 1
    let response = engine.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_eq!(response.status(), OrderStatus::New);
    assert!(in_book(&engine, 2));
}

#[test]
fn cancelling_one_order_leaves_the_others_in_the_book() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_cancelled(&mut engine, cancel(1));

    assert!(!in_book(&engine, 1));
    assert!(in_book(&engine, 2));
}

#[test]
fn a_partly_filled_order_can_be_cancelled() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));
    // fills half of order 1
    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, QUARTER_BTC));

    let response = assert_cancelled(&mut engine, cancel(1));

    assert_eq!(filled_base(&response), QUARTER_BTC);
    assert!(!in_book(&engine, 1));
}

/// The contract (`proto/how_to_use.md`, "Handling command responses") says a cancel answers
/// with both filled amounts, like a place does.
#[test]
#[ignore = "TODO(mehras): RestingOrder doesn't keep its filled quote yet"]
fn a_cancel_reports_the_filled_quote_too() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));
    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, QUARTER_BTC));

    let response = assert_cancelled(&mut engine, cancel(1));

    assert_eq!(filled_quote(&response), QUARTER_BTC_AT_29_000);
}

// ---- rejected ----

#[test]
fn an_unknown_or_empty_market_is_rejected() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let other_market = CancelOrderRequest {
        market_symbol: "ETH-USD".to_string(),
        ..cancel(1)
    };
    assert_rejected(&mut engine, other_market, RejectReason::UnknownMarket);

    let no_market = CancelOrderRequest {
        market_symbol: String::new(),
        ..cancel(2)
    };
    assert_rejected(&mut engine, no_market, RejectReason::UnknownMarket);
}

#[test]
fn an_unknown_order_is_rejected() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_rejected(&mut engine, cancel(42), RejectReason::UnknownOrder);
}

#[test]
fn only_the_owner_can_cancel() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    let stranger = CancelOrderRequest {
        account_holder: Some(Id::from(owner(2))),
        ..cancel(1)
    };
    assert_rejected(&mut engine, stranger, RejectReason::NotOrderOwner);

    assert!(in_book(&engine, 1));
}

#[test]
fn a_second_cancel_of_the_same_order_is_rejected() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    assert_cancelled(&mut engine, cancel(1));

    // a new command: a retry of the first cancel would reuse its command_id
    let again = CancelOrderRequest {
        command_id: Some(Id::from(6_001)),
        ..cancel(1)
    };
    // TODO(mehras): the contract wants ORDER_NOT_WORKING for an order that has ended, but the
    // book forgets ended orders. Until the engine remembers them, it can only say UNKNOWN_ORDER
    assert_rejected(&mut engine, again, RejectReason::UnknownOrder);
}

#[test]
fn a_filled_order_cannot_be_cancelled() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Sell, PRICE_29_000, HALF_BTC));
    // fills all of order 1, which leaves the book
    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    // UNKNOWN_ORDER for the same reason as in a_second_cancel_of_the_same_order_is_rejected
    assert_rejected(&mut engine, cancel(1), RejectReason::UnknownOrder);
}

#[test]
fn a_cancel_without_an_order_id_or_holder_is_rejected() {
    let mut engine = engine();
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC));

    // a missing field is BAD_ORDER_PARAMS, like in PlaceOrder
    let no_order = CancelOrderRequest {
        order_id: None,
        ..cancel(1)
    };
    assert_rejected(&mut engine, no_order, RejectReason::BadOrderParams);

    let no_holder = CancelOrderRequest {
        account_holder: None,
        ..cancel(2)
    };
    assert_rejected(&mut engine, no_holder, RejectReason::BadOrderParams);
}
