//! The command cache, as `proto/how_to_use.md` describes it in "Handling a command":
//! - a known `command_id` with the same request gets the stored response, and nothing runs again
//! - a known `command_id` with a different request is rejected with `DUPLICATE_COMMAND`
//! - a new `command_id` runs, and its response is stored, rejections included

mod common;

use common::{
    HALF_BTC, PRICE_29_000, btc_usd, cancel, engine, engine_with_cache, in_book, limit, place,
};
use proto::{Id, OrderStatus, RejectReason, SetMarketRequest, Side};
use tradingengine::engine::Engine;

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
        market: Some(btc_usd(2)),
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

#[test]
fn oldest_cache_entry_is_dropped_and_a_newer_one_returns_stored_response_for_place() {
    let mut engine = engine_with_cache(5);
    let oldest = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    for n in 2..=5 {
        let _ = engine.place_order(limit(n, Side::Buy, PRICE_29_000, HALF_BTC));
    }
    let newest = engine.place_order(limit(6, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_eq!(oldest.status(), OrderStatus::New);
    let rerun = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    assert_eq!(rerun.reject_reason(), RejectReason::BadOrderParams); // order 1 is still in the book

    assert_eq!(
        engine.place_order(limit(6, Side::Buy, PRICE_29_000, HALF_BTC)),
        newest
    );
}

#[test]
fn cache_removal_does_not_fire_early_for_place() {
    let mut engine = engine_with_cache(3);
    let res_1 = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    let _ = engine.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC));
    let _ = engine.place_order(limit(3, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_eq!(
        engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)),
        res_1
    );
}

// ---- eviction: cancels ----

#[test]
fn oldest_cache_entry_is_dropped_and_a_newer_one_returns_stored_response_for_cancel() {
    let mut engine = engine_with_cache(3);
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 1
    let cancelled = engine.cancel_order(cancel(1)); // entry 2
    assert_eq!(cancelled.status(), OrderStatus::Cancelled);
    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 3, the cache is full
    let newest = engine.cancel_order(cancel(2)); // entry 4, drops the place of order 1
    place(&mut engine, limit(3, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 5, drops the cancel of order 1

    // run again: order 1 left the book with the first cancel
    let rerun = engine.cancel_order(cancel(1));
    assert_eq!(
        rerun.reject_reason(),
        // TODO this will change to OrderNotWorking after implementing ended orders handling
        RejectReason::UnknownOrder,
        "{}",
        rerun.reject_detail
    );
    assert_eq!(engine.cancel_order(cancel(2)), newest);
}

#[test]
fn cache_removal_does_not_fire_early_for_cancel() {
    let mut engine = engine_with_cache(3);
    place(&mut engine, limit(1, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 1
    let cancelled = engine.cancel_order(cancel(1)); // entry 2
    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 3, the cache is full

    // still stored, so not run again: run again it would be UNKNOWN_ORDER
    assert_eq!(engine.cancel_order(cancel(1)), cancelled);
}

// ---- eviction: places and cancels together ----

/// Eviction goes by when an id first arrived: a retry is answered from the cache and
/// doesn't make its entry newer. (An LRU cache would keep the place of order 1 here and
/// drop the cancel.)
#[test]
fn a_retry_does_not_make_an_entry_newer() {
    let mut engine = engine_with_cache(2);
    let placed = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 1
    let cancelled = engine.cancel_order(cancel(1)); // entry 2, the cache is full
    assert_eq!(
        engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)),
        placed
    );

    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC)); // drops the oldest entry

    assert_eq!(engine.cancel_order(cancel(1)), cancelled);
}

/// DUPLICATE_COMMAND and a missing command_id aren't stored, so they take no slot
/// and push nothing out.
#[test]
fn answers_that_are_not_stored_do_not_take_a_slot() {
    let mut engine = engine_with_cache(2);
    let placed = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 1

    let mut reused = cancel(1);
    reused.command_id = Some(Id::from(1_001)); // the place's command_id
    assert_eq!(
        engine.cancel_order(reused).reject_reason(),
        RejectReason::DuplicateCommand
    );
    let mut no_id = cancel(1);
    no_id.command_id = None;
    assert_eq!(
        engine.cancel_order(no_id).reject_reason(),
        RejectReason::BadOrderParams
    );

    place(&mut engine, limit(2, Side::Buy, PRICE_29_000, HALF_BTC)); // entry 2, the cache is full

    // still stored: run again, order 1 would be rejected as a duplicate order id
    assert_eq!(
        engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)),
        placed
    );
    assert!(in_book(&engine, 1));
}

#[test]
#[ignore = "TODO, after having something that returns `INTERNAL_ERROR` from the engine"]
fn internal_error_answers_are_not_stored_in_cache() {
    unimplemented!("bruh")
}
