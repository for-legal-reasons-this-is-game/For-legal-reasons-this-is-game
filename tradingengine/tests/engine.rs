use std::sync::Arc;

use proto::{ConfigRejectReason, Id, Market, SetMarketRequest, SetMarketResponse};

use tradingengine::engine::{Engine, MAX_MARKETS};

fn market(symbol: &str, base_decimals: u32, quote_decimals: u32, price_decimals: u32) -> Market {
    Market {
        symbol: symbol.to_string(),
        base_decimals,
        quote_decimals,
        price_decimals,
        fee_account: Some(Id::from(1u128)),
    }
}

/// A market in satoshis and cents, with prices written in cents: k = 2 + 8 - 2 = 8.
fn btc_usd() -> Market {
    market("BTC-USD", 8, 2, 2)
}

async fn set(engine: &Engine, market: Market) -> SetMarketResponse {
    engine
        .set_market(SetMarketRequest {
            market: Some(market),
        })
        .await
}

fn assert_accepted(response: &SetMarketResponse) {
    assert_eq!(
        response.reject_reason(),
        ConfigRejectReason::Unspecified,
        "rejected: {}",
        response.reject_detail
    );
    assert!(response.reject_detail.is_empty());
}

fn assert_bad_definition(response: &SetMarketResponse) {
    assert_eq!(response.reject_reason(), ConfigRejectReason::BadDefinition);
    assert!(!response.reject_detail.is_empty());
}

#[tokio::test]
async fn a_new_engine_has_no_markets() {
    let engine = Engine::new();

    assert!(engine.market("BTC-USD").await.is_none());
}

#[tokio::test]
async fn set_market_creates_an_empty_book() {
    let engine = Engine::new();

    let response = set(&engine, btc_usd()).await;

    assert_accepted(&response);
    let state = engine.market("BTC-USD").await.expect("BTC-USD was set");
    assert_eq!(state.market(), &btc_usd());
    assert_eq!(state.scale().exponent(), 8);
    assert!(state.book().is_empty());
}

#[tokio::test]
async fn markets_are_kept_apart_by_symbol() {
    let engine = Engine::new();

    assert_accepted(&set(&engine, btc_usd()).await);
    assert_accepted(&set(&engine, market("ETH-USD", 18, 2, 2)).await);

    assert_eq!(
        engine.market("BTC-USD").await.unwrap().scale().exponent(),
        8
    );
    assert_eq!(
        engine.market("ETH-USD").await.unwrap().scale().exponent(),
        18
    );
    assert!(engine.market("SOL-USD").await.is_none());
}

<<<<<<< HEAD
// which definitions are bad is tested on MarketInfo in tests/market.rs,
// here only that a bad one comes back as BAD_DEFINITION and adds nothing
#[tokio::test]
async fn a_scale_above_38_is_a_bad_definition() {
    let engine = Engine::new();
=======
/// Which definitions are bad is tested on
/// [`MarketInfo`](tradingengine::market::MarketInfo) in `tests/market.rs`. This test only
/// checks that a bad one comes back as `BAD_DEFINITION` and adds nothing.
#[test]
fn a_scale_above_38_is_a_bad_definition() {
    let mut engine = Engine::new();
>>>>>>> 55564ca0340801403ee06c3a408bbd23074ebf04

    // k = 9 + 30 - 0 = 39, and 10^39 does not fit in a u128
    let response = set(&engine, market("BTC-USD", 30, 0, 9)).await;

    assert_bad_definition(&response);
    assert!(engine.market("BTC-USD").await.is_none());
}

#[tokio::test]
async fn an_absent_market_is_a_bad_definition() {
    let engine = Engine::new();

    let response = engine.set_market(SetMarketRequest { market: None }).await;

    assert_bad_definition(&response);
}

#[tokio::test]
async fn setting_a_market_again_replaces_its_definition() {
    let engine = Engine::new();
    assert_accepted(&set(&engine, btc_usd()).await);

    // the book is empty, so even the decimals may change
    let mut changed = market("BTC-USD", 8, 2, 3);
    changed.fee_account = Some(Id::from(2u128));
    assert_accepted(&set(&engine, changed.clone()).await);

    let state = engine.market("BTC-USD").await.unwrap();
    assert_eq!(state.market(), &changed);
    assert_eq!(state.scale().exponent(), 9);
}

#[tokio::test]
async fn a_rejected_change_keeps_the_old_definition() {
    let engine = Engine::new();
    assert_accepted(&set(&engine, btc_usd()).await);

    assert_bad_definition(&set(&engine, market("BTC-USD", 30, 0, 9)).await);

    let state = engine.market("BTC-USD").await.unwrap();
    assert_eq!(state.market(), &btc_usd());
    assert_eq!(state.scale().exponent(), 8);
}

#[tokio::test]
async fn a_ninth_market_is_refused() {
    let engine = Engine::new();
    for n in 0..MAX_MARKETS {
        assert_accepted(&set(&engine, market(&format!("COIN{n}-USD"), 8, 2, 2)).await);
    }

    let response = set(&engine, market("ONE-TOO-MANY", 8, 2, 2)).await;

    assert_bad_definition(&response);
    assert!(engine.market("ONE-TOO-MANY").await.is_none());
}

#[tokio::test]
async fn a_full_engine_can_still_change_its_own_markets() {
    let engine = Engine::new();
    for n in 0..MAX_MARKETS {
        assert_accepted(&set(&engine, market(&format!("COIN{n}-USD"), 8, 2, 2)).await);
    }

    assert_accepted(&set(&engine, market("COIN0-USD", 8, 2, 3)).await);

    let state = engine.market("COIN0-USD").await.unwrap();
    assert_eq!(state.scale().exponent(), 9);
}

// the backend's requests arrive on many tasks at once, like gRPC calls do
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn markets_can_be_set_from_many_tasks_at_once() {
    let engine = Arc::new(Engine::new());

    let mut tasks = Vec::new();
    for n in 0..MAX_MARKETS {
        let engine = Arc::clone(&engine);
        tasks.push(tokio::spawn(async move {
            set(&engine, market(&format!("COIN{n}-USD"), 8, 2, 2)).await
        }));
    }
    for task in tasks {
        assert_accepted(&task.await.unwrap());
    }

    for n in 0..MAX_MARKETS {
        assert!(engine.market(&format!("COIN{n}-USD")).await.is_some());
    }
}

// two requests racing to create the same market must not start two threads:
// both are accepted, and the market ends up with one of the two definitions
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_same_new_market_set_twice_at_once_exists_once() {
    let engine = Arc::new(Engine::new());
    let mut first = btc_usd();
    first.fee_account = Some(Id::from(1u128));
    let mut second = btc_usd();
    second.fee_account = Some(Id::from(2u128));

    let a = tokio::spawn({
        let engine = Arc::clone(&engine);
        async move { set(&engine, first).await }
    });
    let b = tokio::spawn({
        let engine = Arc::clone(&engine);
        async move { set(&engine, second).await }
    });
    assert_accepted(&a.await.unwrap());
    assert_accepted(&b.await.unwrap());

    let fee_account = engine.market("BTC-USD").await.unwrap().market().fee_account;
    assert!(fee_account == Some(Id::from(1u128)) || fee_account == Some(Id::from(2u128)));
}
