use proto::{ConfigRejectReason, Id, Market, SetMarketRequest, SetMarketResponse};

use tradingengine::engine::Engine;

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

fn set(engine: &mut Engine, market: Market) -> SetMarketResponse {
    engine.set_market(SetMarketRequest {
        market: Some(market),
    })
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

#[test]
fn a_new_engine_has_no_markets() {
    let engine = Engine::new();

    assert!(engine.market("BTC-USD").is_none());
}

#[test]
fn set_market_creates_an_empty_book() {
    let mut engine = Engine::new();

    let response = set(&mut engine, btc_usd());

    assert_accepted(&response);
    let state = engine.market("BTC-USD").expect("BTC-USD was set");
    assert_eq!(state.market(), &btc_usd());
    assert_eq!(state.scale().exponent(), 8);
    assert!(state.book().is_empty());
}

#[test]
fn markets_are_kept_apart_by_symbol() {
    let mut engine = Engine::new();

    assert_accepted(&set(&mut engine, btc_usd()));
    assert_accepted(&set(&mut engine, market("ETH-USD", 18, 2, 2)));

    assert_eq!(engine.market("BTC-USD").unwrap().scale().exponent(), 8);
    assert_eq!(engine.market("ETH-USD").unwrap().scale().exponent(), 18);
    assert!(engine.market("SOL-USD").is_none());
}

// which definitions are bad is tested on MarketInfo in tests/market.rs,
// here only that a bad one comes back as BAD_DEFINITION and adds nothing
#[test]
fn a_scale_above_38_is_a_bad_definition() {
    let mut engine = Engine::new();

    // k = 9 + 30 - 0 = 39, and 10^39 does not fit in a u128
    let response = set(&mut engine, market("BTC-USD", 30, 0, 9));

    assert_bad_definition(&response);
    assert!(engine.market("BTC-USD").is_none());
}

#[test]
fn an_absent_market_is_a_bad_definition() {
    let mut engine = Engine::new();

    let response = engine.set_market(SetMarketRequest { market: None });

    assert_bad_definition(&response);
}

#[test]
fn setting_a_market_again_replaces_its_definition() {
    let mut engine = Engine::new();
    assert_accepted(&set(&mut engine, btc_usd()));

    // the book is empty, so even the decimals may change
    let mut changed = market("BTC-USD", 8, 2, 3);
    changed.fee_account = Some(Id::from(2u128));
    assert_accepted(&set(&mut engine, changed.clone()));

    let state = engine.market("BTC-USD").unwrap();
    assert_eq!(state.market(), &changed);
    assert_eq!(state.scale().exponent(), 9);
}

#[test]
fn a_rejected_change_keeps_the_old_definition() {
    let mut engine = Engine::new();
    assert_accepted(&set(&mut engine, btc_usd()));

    assert_bad_definition(&set(&mut engine, market("BTC-USD", 30, 0, 9)));

    let state = engine.market("BTC-USD").unwrap();
    assert_eq!(state.market(), &btc_usd());
    assert_eq!(state.scale().exponent(), 8);
}
