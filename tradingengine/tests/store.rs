//! These tests need a Redis they may write to, so they only run when asked. Start one, for
//! example with `docker run --rm -d -p 6390:6379 redis:7-alpine`, then run
//! `ENGINE_TEST_REDIS_URL=redis://127.0.0.1:6390 cargo test --test store -- --ignored`.
//!
//! Each test uses an engine instance name of its own, and deletes its keys when it passes.

use std::time::{SystemTime, UNIX_EPOCH};

use proto::place_order_request::Amount;
use proto::trading_engine_server::TradingEngine;
use proto::{
    CancelOrderRequest, Id, Market, OrderType, PlaceOrderRequest, RejectReason, SetMarketRequest,
    Side, TimeInForce, Uint128,
};
use tonic::Request;
use tradingengine::engine::Engine;
use tradingengine::ids::OrderId;
use tradingengine::service::EngineService;
use tradingengine::store::RedisStore;

const HALF_BTC: u128 = 50_000_000;
const QUARTER_BTC: u128 = 25_000_000;
const PRICE_29_000: u128 = 2_900_000;

fn redis_url() -> String {
    std::env::var("ENGINE_TEST_REDIS_URL")
        .expect("ENGINE_TEST_REDIS_URL names a Redis these tests may write to")
}

/// An engine instance name that no other test run uses.
fn fresh_instance(test: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_nanos();
    format!("test-{test}-{nanos}")
}

async fn store(instance: &str) -> RedisStore {
    RedisStore::connect(&redis_url(), instance)
        .await
        .expect("redis is reachable")
}

async fn redis() -> redis::aio::ConnectionManager {
    let client = redis::Client::open(redis_url()).expect("valid redis url");
    redis::aio::ConnectionManager::new(client)
        .await
        .expect("redis is reachable")
}

/// Deletes every key an engine instance wrote.
async fn clean_up(instance: &str) {
    let mut conn = redis().await;
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("engine:{instance}:*"))
        .query_async(&mut conn)
        .await
        .expect("lists the keys");
    if !keys.is_empty() {
        let _: i64 = redis::cmd("DEL")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .expect("deletes the keys");
    }
}

fn btc_usd() -> SetMarketRequest {
    SetMarketRequest {
        market: Some(Market {
            symbol: "BTC-USD".to_string(),
            base_decimals: 8,
            quote_decimals: 2,
            price_decimals: 2,
            fee_account: Some(Id::from(1u128)),
        }),
    }
}

/// A valid GTC limit order. Every order gets its own order, command and holder id from `n`.
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

/// A cancel of order `n` by its owner.
fn cancel(n: u128) -> CancelOrderRequest {
    CancelOrderRequest {
        command_id: Some(Id::from(5_000 + n)),
        order_id: Some(Id::from(n)),
        account_holder: Some(Id::from(2_000 + n)),
        market_symbol: "BTC-USD".to_string(),
    }
}

#[tokio::test]
#[ignore = "needs Redis: set ENGINE_TEST_REDIS_URL and run with --ignored"]
async fn an_instance_that_never_saved_starts_empty() {
    let instance = fresh_instance("empty");

    let loaded = store(&instance).await.load().await.expect("loads");

    assert_eq!(loaded, Engine::new());
}

#[tokio::test]
#[ignore = "needs Redis: set ENGINE_TEST_REDIS_URL and run with --ignored"]
async fn the_engine_loads_back_as_it_was_saved() {
    let instance = fresh_instance("reload");
    let mut saved = store(&instance).await;
    let mut engine = saved.load().await.expect("loads");

    // every command is saved after it runs, the way the service does it
    engine.set_market(btc_usd());
    saved.save_market(&engine, "BTC-USD").await.expect("saves");
    for request in [
        limit(1, Side::Sell, PRICE_29_000, HALF_BTC),
        // fills half of order 1, and ends
        limit(2, Side::Buy, PRICE_29_000, QUARTER_BTC),
        limit(3, Side::Buy, 2_800_000, HALF_BTC),
    ] {
        engine.place_order(request);
        saved.save_market(&engine, "BTC-USD").await.expect("saves");
    }
    engine.cancel_order(cancel(3));
    saved.save_market(&engine, "BTC-USD").await.expect("saves");

    let loaded = store(&instance).await.load().await.expect("loads");

    assert_eq!(loaded, engine);
    clean_up(&instance).await;
}

#[tokio::test]
#[ignore = "needs Redis: set ENGINE_TEST_REDIS_URL and run with --ignored"]
async fn each_ended_order_is_saved_once() {
    let instance = fresh_instance("ended-once");
    let mut saved = store(&instance).await;
    let mut engine = saved.load().await.expect("loads");
    engine.set_market(btc_usd());
    engine.place_order(limit(1, Side::Sell, PRICE_29_000, HALF_BTC));
    // fills all of order 1 and all of order 2, so both end
    engine.place_order(limit(2, Side::Buy, PRICE_29_000, HALF_BTC));

    saved.save_market(&engine, "BTC-USD").await.expect("saves");
    saved.save_market(&engine, "BTC-USD").await.expect("saves");

    let saved_ended: usize = redis::cmd("LLEN")
        .arg(format!("engine:{instance}:ended:BTC-USD"))
        .query_async(&mut redis().await)
        .await
        .expect("reads the list");
    assert_eq!(saved_ended, 2);
    clean_up(&instance).await;
}

#[tokio::test]
#[ignore = "needs Redis: set ENGINE_TEST_REDIS_URL and run with --ignored"]
async fn the_service_saves_a_command_before_answering_it() {
    let instance = fresh_instance("service");
    let mut saved = store(&instance).await;
    let engine = saved.load().await.expect("loads");
    let service = EngineService::with_store(engine, saved);

    service
        .set_market(Request::new(btc_usd()))
        .await
        .expect("answers");
    let placed = service
        .place_order(Request::new(limit(1, Side::Buy, PRICE_29_000, HALF_BTC)))
        .await
        .expect("answers")
        .into_inner();
    assert_eq!(placed.reject_reason(), RejectReason::Unspecified);

    // an engine that restarts now still has the order the backend was told about
    let loaded = store(&instance).await.load().await.expect("loads");
    let market = loaded.market("BTC-USD").expect("the market was saved");
    assert!(market.book().get_by_id(OrderId::new(1)).is_some());
    clean_up(&instance).await;
}
