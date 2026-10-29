use proto::{Id, Market};

use tradingengine::error::EngineError;
use tradingengine::market::MarketInfo;
use tradingengine::market_worker::MarketHandle;

fn market(symbol: &str, base_decimals: u32, quote_decimals: u32, price_decimals: u32) -> Market {
    Market {
        symbol: symbol.to_string(),
        base_decimals,
        quote_decimals,
        price_decimals,
        fee_account: Some(Id::from(1u128)),
    }
}

// satoshis, cents, and a price written in cents: k = 2 + 8 - 2 = 8
fn btc_usd() -> Market {
    market("BTC-USD", 8, 2, 2)
}

fn spawn_btc_usd() -> MarketHandle {
    MarketHandle::spawn(MarketInfo::new(btc_usd()).unwrap())
}

#[tokio::test]
async fn a_new_worker_holds_the_market_it_was_given() {
    let handle = spawn_btc_usd();

    let info = handle.snapshot().await;

    assert_eq!(info.market(), &btc_usd());
    assert!(info.book().is_empty());
}

#[tokio::test]
async fn a_reconfigure_is_seen_by_the_next_job() {
    let handle = spawn_btc_usd();

    handle
        .reconfigure(market("BTC-USD", 8, 2, 3))
        .await
        .unwrap();

    assert_eq!(handle.snapshot().await.scale().exponent(), 9);
}

#[tokio::test]
async fn a_rejected_reconfigure_comes_back_and_changes_nothing() {
    let handle = spawn_btc_usd();

    let result = handle.reconfigure(market("BTC-USD", 30, 0, 9)).await;

    assert_eq!(result, Err(EngineError::DecimalsOutOfRange));
    assert_eq!(handle.snapshot().await.market(), &btc_usd());
}

// a clone talks to the same thread, so a change made through one is seen
// through the other
#[tokio::test]
async fn clones_of_a_handle_share_one_market() {
    let handle = spawn_btc_usd();
    let other = handle.clone();

    other.reconfigure(market("BTC-USD", 8, 2, 3)).await.unwrap();

    assert_eq!(handle.snapshot().await.scale().exponent(), 9);
}

// jobs from one sender run in the order they were queued: every change is
// applied before the next, so the last one sent is the one left standing
#[tokio::test]
async fn jobs_run_in_the_order_they_were_sent() {
    let handle = spawn_btc_usd();

    for fee_account in 1..=100u128 {
        let mut next = btc_usd();
        next.fee_account = Some(Id::from(fee_account));
        handle.reconfigure(next).await.unwrap();
    }

    let fee_account = handle.snapshot().await.market().fee_account;
    assert_eq!(fee_account, Some(Id::from(100u128)));
}
