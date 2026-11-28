//! `RedisStore` on its own: what the engine saves, one command at a time, loads back as the
//! same engine. How the engine uses it, saving before every answer, is tested in
//! `tests/router_consumer.rs`.
//!
//! These need a running Redis at `REDIS_URL`, so they're `#[ignore]`d. CI starts a Redis and
//! runs them with `-- --ignored`.

mod common;

use common::{HALF_BTC, PRICE_29_000, btc_usd, cancel, engine_with_cache, filled_base, limit};
use proto::{SetMarketRequest, Side};
use tradingengine::command_cache::DEFAULT_CACHE_CAPACITY;
use tradingengine::engine::{Engine, State};
use tradingengine::redis_store::RedisStore;

const QUARTER_BTC: u128 = 25_000_000;
const PRICE_28_000: u128 = 2_800_000;
const PRICE_30_000: u128 = 3_000_000;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

async fn store(instance: &str) -> RedisStore {
    RedisStore::connect(&redis_url(), instance)
        .await
        .expect("connect")
}

/// The store of `instance`, with what an earlier run of the test saved deleted.
async fn empty_store(instance: &str) -> RedisStore {
    let mut store = store(instance).await;
    store.clear().await.expect("clear");
    store
}

/// The engine a restart of `instance` loads, with room for `capacity` cache entries.
async fn restarted(instance: &str, capacity: usize) -> Engine {
    let state = store(instance)
        .await
        .load_engine_state()
        .await
        .expect("load");
    Engine::new_from_state(state, capacity).expect("it loads")
}

/// `engine`, saving to `instance`'s store.
async fn saving(engine: Engine, instance: &str) -> State {
    State {
        engine,
        store: Some(empty_store(instance).await),
    }
}

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_instance_that_never_saved_loads_an_empty_engine() {
    let mut store = empty_store("test-store-empty").await;

    let state = store.load_engine_state().await.expect("load");

    assert_eq!(state, Engine::default().get_engine_state());
}

/// Every kind of change, saved after each command like the router consumer does. If a
/// command changed something it didn't save, the restarted engine would be different.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn a_restarted_engine_is_the_engine_that_was_saved() {
    let instance = "test-store-restart";
    let mut state = saving(Engine::default(), instance).await;
    state.engine.set_market(SetMarketRequest {
        market: Some(btc_usd(2)),
    });
    state.save_engine_changes().await;

    for request in [
        limit(1, Side::Sell, PRICE_29_000, HALF_BTC),
        limit(2, Side::Sell, PRICE_29_000, HALF_BTC), // behind order 1 in the queue
        limit(3, Side::Sell, PRICE_30_000, HALF_BTC),
        limit(4, Side::Buy, PRICE_28_000, HALF_BTC), // rests, nothing to match
        limit(5, Side::Sell, PRICE_29_000, HALF_BTC), // behind order 2
        limit(6, Side::Buy, 0, HALF_BTC),            // rejected, its answer is cached
    ] {
        state.engine.place_order(request);
        state.save_engine_changes().await;
    }
    // fills order 1 (it leaves the book) and half of order 2 (it stays, at the front),
    // and has nothing left to rest
    let taker = state
        .engine
        .place_order(limit(7, Side::Buy, PRICE_29_000, HALF_BTC + QUARTER_BTC));
    assert_eq!(filled_base(&taker), HALF_BTC + QUARTER_BTC);
    state.save_engine_changes().await;
    state.engine.cancel_order(cancel(3));
    state.save_engine_changes().await;

    let restarted = restarted(instance, DEFAULT_CACHE_CAPACITY).await;

    // the markets, the books with their queues, the cache, the events and the sequence
    assert_eq!(restarted, state.engine);
    let asks: Vec<u128> = restarted
        .market("BTC-USD")
        .expect("the market was saved")
        .book()
        .asks()
        .map(|order| order.id().value())
        .collect();
    assert_eq!(asks, [2, 5], "order 2 is still ahead of order 5");
}

/// A full cache drops its oldest entry for a new one. Redis must drop the same one.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn the_oldest_cache_entry_leaves_redis_too() {
    let instance = "test-store-cache";
    let mut state = saving(engine_with_cache(2), instance).await;
    state.save_engine_changes().await;
    for n in 1..=3 {
        state
            .engine
            .place_order(limit(n, Side::Buy, PRICE_28_000, HALF_BTC));
        state.save_engine_changes().await;
    }

    assert_eq!(restarted(instance, 2).await, state.engine);
}

/// With a lower cache capacity than when the entries were saved, the oldest ones don't fit.
/// The first save deletes them from Redis too, so the next start loads the same cache.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn a_lower_cache_capacity_deletes_the_oldest_saved_entries() {
    let instance = "test-store-lower-capacity";
    let mut state = saving(engine_with_cache(3), instance).await;
    state.save_engine_changes().await;
    for n in 1..=3 {
        state
            .engine
            .place_order(limit(n, Side::Buy, PRICE_28_000, HALF_BTC));
        state.save_engine_changes().await;
    }

    let mut smaller = State {
        engine: restarted(instance, 2).await,
        store: Some(store(instance).await),
    };
    smaller.save_engine_changes().await;

    assert_eq!(restarted(instance, 2).await, smaller.engine);
}
