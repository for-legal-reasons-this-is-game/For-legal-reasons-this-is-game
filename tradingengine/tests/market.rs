use proto::{Id, Market};

use tradingengine::error::EngineError;
use tradingengine::market::MarketInfo;

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

fn without_fee_account(mut market: Market) -> Market {
    market.fee_account = None;
    market
}

#[test]
fn a_new_market_starts_with_an_empty_book() {
    let info = MarketInfo::new(btc_usd()).unwrap();

    assert_eq!(info.market(), &btc_usd());
    assert_eq!(info.scale().exponent(), 8);
    assert!(info.book().is_empty());
}

#[test]
fn a_scale_above_38_is_out_of_range() {
    // k = 9 + 30 - 0 = 39, and 10^39 does not fit in a u128
    assert_eq!(
        MarketInfo::new(market("BTC-USD", 30, 0, 9)),
        Err(EngineError::DecimalsOutOfRange)
    );
}

#[test]
fn a_scale_of_exactly_38_is_accepted() {
    let info = MarketInfo::new(market("BTC-USD", 30, 0, 8)).unwrap();

    assert_eq!(info.scale().exponent(), 38);
}

#[test]
fn a_negative_scale_is_out_of_range() {
    // k = 0 + 2 - 6 = -4, the example from DATA_MODEL Layer 3
    assert_eq!(
        MarketInfo::new(market("BTC-USD", 2, 6, 0)),
        Err(EngineError::DecimalsOutOfRange)
    );
}

#[test]
fn an_empty_symbol_is_rejected() {
    assert_eq!(
        MarketInfo::new(market("", 8, 2, 2)),
        Err(EngineError::MarketSymbolEmpty)
    );
}

#[test]
fn an_absent_fee_account_is_rejected() {
    assert_eq!(
        MarketInfo::new(without_fee_account(btc_usd())),
        Err(EngineError::RequiredFieldAbsent)
    );
}

#[test]
fn reconfigure_with_an_empty_book_may_change_the_decimals() {
    let mut info = MarketInfo::new(btc_usd()).unwrap();

    let mut changed = market("BTC-USD", 8, 2, 3);
    changed.fee_account = Some(Id::from(2u128));
    assert_eq!(info.reconfigure(changed.clone()), Ok(()));

    assert_eq!(info.market(), &changed);
    assert_eq!(info.scale().exponent(), 9);
}

#[test]
fn a_rejected_reconfigure_leaves_the_market_as_it_was() {
    let mut info = MarketInfo::new(btc_usd()).unwrap();
    let before = info.clone();

    assert_eq!(
        info.reconfigure(market("BTC-USD", 30, 0, 9)),
        Err(EngineError::DecimalsOutOfRange)
    );
    assert_eq!(
        info.reconfigure(without_fee_account(btc_usd())),
        Err(EngineError::RequiredFieldAbsent)
    );

    assert_eq!(info, before);
}
