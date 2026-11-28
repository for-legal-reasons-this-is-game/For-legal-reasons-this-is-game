mod common;

use common::{
    HALF_BTC, PRICE_29_000, assert_accepted, assert_rejected, cancel, engine, engine_with_cache,
    filled_base, filled_quote, in_book, limit,
};
use proto::{OrderStatus, RejectReason, Side};
use serde::{Deserialize, Serialize, de::IgnoredAny};
use tradingengine::command_cache::{Answer, Command};
use tradingengine::engine::{EVENT_HISTORY_CAPACITY, Engine};
use tradingengine::engine_state::{EngineState, MarketState};
use tradingengine::error::EngineError;
use tradingengine::ids::CommandId;

#[test]
fn loading_a_saved_engine_works() {
    let mut engine = engine();
    let response_1 = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    let response_2 = engine.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    let state = engine.get_engine_state();
    let mut engine_2 =
        Engine::new_from_state(state, 100_000).expect("loading a saved engine state failed");
    let saved_cached_response_1 = engine_2.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    let saved_cached_response_2 = engine_2.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    assert_accepted(&response_1);
    assert_accepted(&response_2);
    assert_accepted(&saved_cached_response_1);
    assert_accepted(&saved_cached_response_2);
    assert_eq!(response_1, saved_cached_response_1);
    assert_eq!(response_2, saved_cached_response_2);
    let book = engine_2.market("BTC-USD").expect("market exists").book();
    assert_eq!(book.len(), 2);
}

// ---- save and load: helpers ----

const QUARTER_BTC: u128 = 25_000_000;
const PRICE_28_000: u128 = 2_800_000;
const PRICE_30_000: u128 = 3_000_000;
const CACHE_CAPACITY: usize = 50;

/// Saves `engine` to bytes and loads them into a new engine, like a restart.
fn save_and_load(engine: &Engine, capacity: usize) -> Engine {
    let bytes = engine
        .get_engine_state()
        .to_bytes()
        .expect("saving to bytes");
    let state = EngineState::from_bytes(&bytes).expect("loading bytes");
    Engine::new_from_state(state, capacity).expect("a saved engine loads")
}

// ---- save and load ----

#[test]
fn an_engine_with_no_market_loads_back_empty() {
    let engine = Engine::with_cache_capacity(CACHE_CAPACITY);

    let mut loaded = save_and_load(&engine, CACHE_CAPACITY);

    assert_eq!(loaded, engine);
    // like the very first start: no market until the backend sends SetMarket
    let response = loaded.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    assert_eq!(response.reject_reason(), RejectReason::UnknownMarket);
}

#[test]
fn a_loaded_engine_equals_the_saved_one() {
    let mut engine = engine_with_cache(CACHE_CAPACITY);
    // both sides of the book, two orders at one price, a partly filled order
    assert_accepted(&engine.place_order(limit(1, Side::Buy, PRICE_28_000, HALF_BTC)));
    assert_accepted(&engine.place_order(limit(2, Side::Buy, PRICE_28_000, HALF_BTC)));
    assert_accepted(&engine.place_order(limit(3, Side::Sell, PRICE_29_000, HALF_BTC)));
    assert_accepted(&engine.place_order(limit(4, Side::Sell, PRICE_30_000, HALF_BTC)));
    let buy = engine.place_order(limit(5, Side::Buy, PRICE_29_000, QUARTER_BTC)); // fills half of order 3
    assert_accepted(&buy);
    assert_eq!(filled_base(&buy), QUARTER_BTC);
    assert_eq!(filled_quote(&buy), 725_000); // 0.25 BTC at $29,000
    // a cancel and a rejection in the cache too
    let cancelled = engine.cancel_order(cancel(4));
    assert_eq!(cancelled.reject_reason(), RejectReason::Unspecified);
    assert_rejected(
        &mut engine,
        limit(6, Side::Buy, 0, HALF_BTC),
        RejectReason::BadPrice,
    );

    let loaded = save_and_load(&engine, CACHE_CAPACITY);

    // not `loaded == engine`: `engine` still holds the changes it would save to redis,
    // and a loaded engine starts with none
    assert_eq!(loaded.get_engine_state(), engine.get_engine_state());
}

#[test]
fn orders_at_one_price_keep_their_queue_order() {
    let mut engine = engine_with_cache(CACHE_CAPACITY);
    assert_accepted(&engine.place_order(limit(1, Side::Sell, PRICE_29_000, HALF_BTC)));
    assert_accepted(&engine.place_order(limit(2, Side::Sell, PRICE_29_000, HALF_BTC)));

    let mut loaded = save_and_load(&engine, CACHE_CAPACITY);

    // order 1 came first, so a buy for one of the two must fill order 1
    let buy = loaded.place_order(limit(3, Side::Buy, PRICE_29_000, HALF_BTC));
    assert_eq!(buy.status(), OrderStatus::Filled, "{}", buy.reject_detail);
    assert!(!in_book(&loaded, 1));
    assert!(in_book(&loaded, 2));
}

#[test]
fn a_retry_after_loading_gets_the_stored_answer_and_runs_nothing() {
    let mut engine = engine_with_cache(CACHE_CAPACITY);
    assert_accepted(&engine.place_order(limit(1, Side::Sell, PRICE_29_000, HALF_BTC)));
    assert_accepted(&engine.place_order(limit(2, Side::Sell, PRICE_29_000, HALF_BTC)));
    assert_accepted(&engine.place_order(limit(3, Side::Buy, PRICE_28_000, HALF_BTC)));
    let buy = engine.place_order(limit(4, Side::Buy, PRICE_29_000, HALF_BTC)); // fills order 1
    assert_eq!(buy.status(), OrderStatus::Filled, "{}", buy.reject_detail);
    let cancelled = engine.cancel_order(cancel(3));
    assert_eq!(
        cancelled.status(),
        OrderStatus::Cancelled,
        "{}",
        cancelled.reject_detail
    );

    let mut loaded = save_and_load(&engine, CACHE_CAPACITY);

    // run again, the buy would fill order 2, and the cancel would be rejected: order 3 is gone
    assert_eq!(
        loaded.place_order(limit(4, Side::Buy, PRICE_29_000, HALF_BTC)),
        buy
    );
    assert_eq!(loaded.cancel_order(cancel(3)), cancelled);
    assert!(in_book(&loaded, 2));
}

/// The cache capacity comes from configuration, not from the saved state, so it can be
/// smaller after a restart. Loading then keeps the newest entries, as eviction would.
#[test]
fn loading_into_a_smaller_cache_keeps_the_newest_entries() {
    let mut engine = engine_with_cache(CACHE_CAPACITY);
    assert_accepted(&engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)));
    assert_accepted(&engine.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC)));
    let newest = engine.place_order(limit(3, Side::Buy, PRICE_29_000, HALF_BTC));
    assert_accepted(&newest);

    let mut loaded = save_and_load(&engine, 2);

    assert_eq!(
        loaded.place_order(limit(3, Side::Buy, PRICE_29_000, HALF_BTC)),
        newest
    );
    // order 1's entry didn't fit, so its retry runs again, and order 1 is still in the book
    let rerun = loaded.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    assert_eq!(
        rerun.reject_reason(),
        RejectReason::BadOrderParams,
        "{}",
        rerun.reject_detail
    );
}

#[test]
fn a_saved_state_with_a_duplicate_order_does_not_load() {
    let mut engine = engine_with_cache(CACHE_CAPACITY);
    assert_accepted(&engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)));
    let mut state = engine.get_engine_state();
    let order = state.markets[0].orders[0].clone();
    state.markets[0].orders.push(order);

    assert_eq!(
        Engine::new_from_state(state, CACHE_CAPACITY),
        Err(EngineError::DuplicateOrderId)
    );
}

/// A rolling update stops the old engine, which saves, and starts the new one, which
/// must load that save. So a save has to load into a version that added a field.
#[test]
fn a_save_still_loads_after_a_field_is_added() {
    // `EngineState` as a later version could look: a market data sequence,
    // added between the fields that exist now
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct NextEngineState {
        markets: IgnoredAny,
        sequence: u64,
        #[serde(default)]
        md_sequence: u64,
        event_history: IgnoredAny,
        command_cache: IgnoredAny,
    }
    let mut engine = engine_with_cache(CACHE_CAPACITY);
    assert_accepted(&engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)));
    let bytes = engine
        .get_engine_state()
        .to_bytes()
        .expect("saving to bytes");

    rmp_serde::from_slice::<NextEngineState>(&bytes)
        .expect("a save from this version loads into the next one");
}

/// The contract wants `sequence` to keep counting across restarts, with no gaps or repeats.
#[test]
fn the_event_sequence_counts_up_and_survives_a_restart() {
    let mut engine = engine();
    assert_eq!(engine.next_sequence(), 1);
    assert_eq!(engine.next_sequence(), 2);

    let mut restarted = save_and_load(&engine, CACHE_CAPACITY);

    assert_eq!(restarted.next_sequence(), 3);
}

/// The consumer gets the whole history again on every connect, so a restart must keep it.
#[test]
fn the_event_history_survives_a_restart() {
    let mut engine = engine();
    assert_accepted(&engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)));
    assert_eq!(engine.event_history().len(), 1);

    let restarted = save_and_load(&engine, CACHE_CAPACITY);

    assert_eq!(restarted.event_history(), engine.event_history());
}

#[test]
fn the_event_history_keeps_only_the_newest_events() {
    let mut engine = engine();
    let extra = 5;
    for n in 1..=(EVENT_HISTORY_CAPACITY + extra) as u128 {
        assert_accepted(&engine.place_order(limit(n, Side::Buy, PRICE_29_000, HALF_BTC)));
    }

    let history = engine.event_history();
    assert_eq!(history.len(), EVENT_HISTORY_CAPACITY);
    let oldest = history.front().expect("history").sequence;
    let newest = history.back().expect("history").sequence;
    assert_eq!(oldest, extra as u64 + 1);
    assert_eq!(newest, (EVENT_HISTORY_CAPACITY + extra) as u64);
}

/// A save from before the history existed has no `event_history`. It must still load.
#[test]
fn a_save_from_before_the_event_history_still_loads() {
    #[derive(Serialize)]
    struct OldEngineState {
        markets: Vec<MarketState>,
        sequence: u64,
        command_cache: Vec<(CommandId, Command, Answer)>,
    }
    let mut engine = engine();
    assert_accepted(&engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)));
    let state = engine.get_engine_state();
    let old = OldEngineState {
        markets: state.markets,
        sequence: state.sequence,
        command_cache: state.command_cache,
    };
    let bytes = rmp_serde::to_vec_named(&old).expect("saving to bytes");

    let loaded = EngineState::from_bytes(&bytes).expect("an old save loads");

    assert!(loaded.event_history.is_empty());
    assert_eq!(loaded.sequence, 1);
}
