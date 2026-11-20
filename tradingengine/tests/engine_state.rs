use proto::{
    CancelOrderRequest, Id, Market, OrderStatus, OrderType, PlaceOrderRequest, PlaceOrderResponse,
    RejectReason, SetMarketRequest, Side, TimeInForce, Uint128, place_order_request::Amount,
};
use serde::{Deserialize, de::IgnoredAny};
use tradingengine::{engine::Engine, engine_state::EngineState, error::EngineError, ids::OrderId};

// BTC-USD: base in satoshis (8), quote in cents (2), price in cents (2), so
// k = 2 + 8 - 2 = 8 and 0.5 BTC at $29,000 is 1_450_000 cents
const HALF_BTC: u128 = 50_000_000;
const PRICE_29_000: u128 = 2_900_000;

fn btc_usd(price_decimals: u32) -> Market {
    Market {
        symbol: "BTC-USD".to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals,
        fee_account: Some(Id::from(1u128)),
    }
}

fn engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_market(SetMarketRequest {
        market: Some(btc_usd(2)),
    });
    engine
}

/// a valid GTC limit order. every order gets its own order, command and holder
/// id from `n`, so no two orders collide and no order trades with its own holder
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

fn filled_base(response: &PlaceOrderResponse) -> u128 {
    response.filled_base_quantity.map(u128::from).unwrap_or(0)
}

fn filled_quote(response: &PlaceOrderResponse) -> u128 {
    response.filled_quote_quantity.map(u128::from).unwrap_or(0)
}

fn assert_accepted(response: &PlaceOrderResponse) {
    assert_eq!(
        response.reject_reason(),
        RejectReason::Unspecified,
        "rejected: {}",
        response.reject_detail
    );
}

// a rejected place must not create an order or touch the book
fn assert_rejected(engine: &mut Engine, request: PlaceOrderRequest, reason: RejectReason) {
    let before = engine.markets().clone();
    let response = engine.place_order(request);
    assert_eq!(
        response.reject_reason(),
        reason,
        "{}",
        response.reject_detail
    );
    assert!(!response.reject_detail.is_empty());
    assert_eq!(*engine.markets(), before);
}

#[test]
fn loading_a_saved_engine_works() {
    let mut engine = engine();
    let response_1 = engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC));
    let response_2 = engine.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    let state = engine.save();
    let mut engine_2 = Engine::load(state, 100_000).expect("loading a saved engine state failed");
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

/// An engine with the BTC-USD market and room for `capacity` cache entries.
fn engine_with_cache(capacity: usize) -> Engine {
    let mut engine = Engine::with_cache_capacity(capacity);
    engine.set_market(SetMarketRequest {
        market: Some(btc_usd(2)),
    });
    engine
}

/// Saves `engine` to bytes and loads them into a new engine, like a restart.
fn save_and_load(engine: &Engine, capacity: usize) -> Engine {
    let bytes = engine.save().to_bytes().expect("saving to bytes");
    let state = EngineState::from_bytes(&bytes).expect("loading bytes");
    Engine::load(state, capacity).expect("a saved engine loads")
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

/// Whether order `n` is in the BTC-USD book.
fn in_book(engine: &Engine, n: u128) -> bool {
    engine
        .market("BTC-USD")
        .expect("market exists")
        .book()
        .get_by_id(OrderId::new(n))
        .is_some()
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

    assert_eq!(loaded, engine);
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
    let mut state = engine.save();
    let order = state.markets[0].orders[0].clone();
    state.markets[0].orders.push(order);

    assert_eq!(
        Engine::load(state, CACHE_CAPACITY),
        Err(EngineError::DuplicateOrderId)
    );
}

/// A rolling update stops the old engine, which saves, and starts the new one, which
/// must load that save. So a save has to load into a version that added a field.
#[test]
fn a_save_still_loads_after_a_field_is_added() {
    // `EngineState` as a later version could look: the contract's event `sequence`,
    // added between the fields that exist now
    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct NextEngineState {
        markets: IgnoredAny,
        #[serde(default)]
        sequence: u64,
        command_cache: IgnoredAny,
    }
    let mut engine = engine_with_cache(CACHE_CAPACITY);
    assert_accepted(&engine.place_order(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)));
    let bytes = engine.save().to_bytes().expect("saving to bytes");

    rmp_serde::from_slice::<NextEngineState>(&bytes)
        .expect("a save from this version loads into the next one");
}
