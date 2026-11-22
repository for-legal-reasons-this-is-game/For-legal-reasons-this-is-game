use proto::CancelOrderRequest;
use proto::trading_engine_server::TradingEngine;
use proto::{
    Id, Market, OrderType, PlaceOrderRequest, RejectReason, SetMarketRequest, Side, TimeInForce,
    Uint128, place_order_request::Amount,
};
use tonic::Request;
use tradingengine::command_cache::DEFAULT_CACHE_CAPACITY;
use tradingengine::engine::Engine;
use tradingengine::engine_state::EngineState;
use tradingengine::redis_store::RedisStore;
use tradingengine::service::EngineService;

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_instance_that_never_saved_loads_nothing() {
    let mut store = RedisStore::connect(&redis_url(), "test-never-saved")
        .await
        .expect("connect");
    assert_eq!(store.load().await.expect("load"), None);
}

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn saved_bytes_come_back_unchanged() {
    let mut store = RedisStore::connect(&redis_url(), "test-round-trip")
        .await
        .expect("connect");
    let bytes = vec![0, 1, 2, 255, 0];
    store.save(&bytes).await.expect("save");
    assert_eq!(store.load().await.expect("load"), Some(bytes));
}

// ---- the service saves before it answers ----

fn btc_usd() -> Market {
    Market {
        symbol: "BTC-USD".to_string(),
        base_decimals: 8,
        quote_decimals: 2,
        price_decimals: 2,
        fee_account: Some(Id::from(1u128)),
    }
}

/// A GTC limit buy of 0.5 BTC at $29,000, with ids from `n`.
fn limit_buy(n: u128) -> PlaceOrderRequest {
    let mut request = PlaceOrderRequest {
        command_id: Some(Id::from(1_000 + n)),
        order_id: Some(Id::from(n)),
        market_symbol: "BTC-USD".to_string(),
        account_holder: Some(Id::from(2_000 + n)),
        base_account: Some(Id::from(3_000 + n)),
        quote_account: Some(Id::from(4_000 + n)),
        amount: Some(Amount::BaseQuantity(Uint128::from(50_000_000u128))),
        limit_price: Some(Uint128::from(2_900_000u128)),
        maker_fee_bps: 5,
        taker_fee_bps: 10,
        ..PlaceOrderRequest::default()
    };
    request.set_side(Side::Buy);
    request.set_type(OrderType::Limit);
    request.set_time_in_force(TimeInForce::Gtc);
    request
}

/// The contract: nothing leaves the engine until the state it depends on is in Redis. So
/// once a handler has answered, a restarted engine loads what that command did.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_answered_command_is_already_saved() {
    let instance = "test-service-saves";
    let store = RedisStore::connect(&redis_url(), instance)
        .await
        .expect("connect");
    let service = EngineService::new(Engine::default(), Some(store));

    service
        .set_market(Request::new(SetMarketRequest {
            market: Some(btc_usd()),
        }))
        .await
        .expect("set_market answers");
    let placed = service
        .place_order(Request::new(limit_buy(1)))
        .await
        .expect("place_order answers")
        .into_inner();
    assert_eq!(
        placed.reject_reason(),
        RejectReason::Unspecified,
        "{}",
        placed.reject_detail
    );

    // what a restarted engine would load
    let mut store = RedisStore::connect(&redis_url(), instance)
        .await
        .expect("connect");
    let bytes = store
        .load()
        .await
        .expect("load")
        .expect("the service saved");
    let state = EngineState::from_bytes(&bytes).expect("a saved state");
    let mut restarted = Engine::load(state, DEFAULT_CACHE_CAPACITY).expect("it loads");

    let book = restarted
        .market("BTC-USD")
        .expect("the market was saved")
        .book();
    assert_eq!(book.len(), 1);
    // the cache entry was saved too: a retry gets the same answer
    assert_eq!(restarted.place_order(limit_buy(1)), placed);
}

// ---- every handler saves, rejections too ----

/// A cancel for order `n`, by the holder `limit_buy(n)` used.
fn cancel(n: u128) -> CancelOrderRequest {
    CancelOrderRequest {
        command_id: Some(Id::from(5_000 + n)),
        order_id: Some(Id::from(n)),
        account_holder: Some(Id::from(2_000 + n)),
        market_symbol: "BTC-USD".to_string(),
    }
}

/// A service that saves to `instance`, with the BTC-USD market set.
async fn service_with_market(instance: &str) -> EngineService {
    let store = RedisStore::connect(&redis_url(), instance)
        .await
        .expect("connect");
    let service = EngineService::new(Engine::default(), Some(store));
    service
        .set_market(Request::new(SetMarketRequest {
            market: Some(btc_usd()),
        }))
        .await
        .expect("set_market answers");
    service
}

/// The engine a restart of `instance` would load.
async fn restarted(instance: &str) -> Engine {
    let mut store = RedisStore::connect(&redis_url(), instance)
        .await
        .expect("connect");
    let bytes = store
        .load()
        .await
        .expect("load")
        .expect("the service saved");
    let state = EngineState::from_bytes(&bytes).expect("a saved state");
    Engine::load(state, DEFAULT_CACHE_CAPACITY).expect("it loads")
}

#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_answered_cancel_is_already_saved() {
    let instance = "test-service-saves-cancel";
    let service = service_with_market(instance).await;
    service
        .place_order(Request::new(limit_buy(1)))
        .await
        .expect("place_order answers");
    let cancelled = service
        .cancel_order(Request::new(cancel(1)))
        .await
        .expect("cancel_order answers")
        .into_inner();
    assert_eq!(cancelled.reject_reason(), RejectReason::Unspecified);

    let book_len = restarted(instance)
        .await
        .market("BTC-USD")
        .expect("the market was saved")
        .book()
        .len();
    assert_eq!(book_len, 0);
}

/// A rejection's answer is cached too, so it has to be saved: after a restart, the same
/// command id with a different request is still a DUPLICATE_COMMAND.
#[tokio::test]
#[ignore = "needs a running Redis at REDIS_URL"]
async fn an_answered_rejection_is_already_saved() {
    let instance = "test-service-saves-rejection";
    let service = service_with_market(instance).await;
    let mut no_quantity = limit_buy(1);
    no_quantity.amount = Some(Amount::BaseQuantity(Uint128::from(0u128)));
    let rejected = service
        .place_order(Request::new(no_quantity))
        .await
        .expect("place_order answers")
        .into_inner();
    assert_eq!(rejected.reject_reason(), RejectReason::BadQuantity);

    // same command id, a valid request this time
    let retry = restarted(instance).await.place_order(limit_buy(1));
    assert_eq!(retry.reject_reason(), RejectReason::DuplicateCommand);
}
