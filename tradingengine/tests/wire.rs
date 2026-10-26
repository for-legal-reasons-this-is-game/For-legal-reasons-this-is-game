use std::time::{Duration, SystemTime, UNIX_EPOCH};

use proto::{Id, Market, Timestamp as WireTimestamp, Uint128};

use tradingengine::error::EngineError;
use tradingengine::fee::FeeBps;
use tradingengine::price::Price;
use tradingengine::quantity::{BaseQuantity, QuoteQuantity};
use tradingengine::quote::QuoteScale;
use tradingengine::wire::{required, required_id};

fn wire(value: u128) -> Uint128 {
    Uint128::from(value)
}

fn market(base_decimals: u32, quote_decimals: u32, price_decimals: u32) -> Market {
    Market {
        symbol: "BTC-USD".to_string(),
        base_decimals,
        quote_decimals,
        price_decimals,
        fee_account: Some(Id::from(1u128)),
    }
}

#[test]
fn an_absent_field_is_not_a_zero() {
    fn base_from_field(field: Option<Uint128>) -> Result<BaseQuantity, EngineError> {
        Ok(BaseQuantity::from(required(field)?))
    }
    fn price_from_field(field: Option<Uint128>) -> Result<Price, EngineError> {
        Price::try_from(required(field)?)
    }

    let absent: Option<Uint128> = None;
    let zero = Some(wire(0));

    assert_eq!(
        base_from_field(absent),
        Err(EngineError::RequiredFieldAbsent)
    );
    assert_eq!(base_from_field(zero), Ok(BaseQuantity::ZERO));

    // A price rejects both, but for two different reasons.
    assert_eq!(
        price_from_field(absent),
        Err(EngineError::RequiredFieldAbsent)
    );
    assert_eq!(price_from_field(zero), Err(EngineError::PriceNotPositive));
}

#[test]
fn required_names_the_absence_whatever_the_payload() {
    assert_eq!(required(Some(7u32)), Ok(7));
    assert_eq!(required::<u32>(None), Err(EngineError::RequiredFieldAbsent));
}

#[test]
fn a_price_survives_the_round_trip() {
    for minor_units in [1u128, 2_900_000, u64::MAX as u128, u128::MAX] {
        let price = Price::try_from(wire(minor_units)).unwrap();
        assert_eq!(price.minor_units(), minor_units);
        assert_eq!(u128::from(Uint128::from(price)), minor_units);
    }
}

#[test]
fn a_price_is_read_from_both_halves() {
    // Not just the low 64 bits: value = (high << 64) | low.
    let value = (7u128 << 64) | 42;
    let message = wire(value);
    assert_eq!((message.high, message.low), (7, 42));
    assert_eq!(Price::try_from(message).unwrap().minor_units(), value);
}

#[test]
fn a_zero_price_is_rejected_by_its_own_rule() {
    assert_eq!(Price::try_from(wire(0)), Err(EngineError::PriceNotPositive));
}

#[test]
fn quantities_accept_zero_and_round_trip() {
    for minor_units in [0u128, 1, 50_000_000, u128::MAX] {
        let base = BaseQuantity::from(wire(minor_units));
        assert_eq!(base.minor_units(), minor_units);
        assert_eq!(u128::from(Uint128::from(base)), minor_units);

        let quote = QuoteQuantity::from(wire(minor_units));
        assert_eq!(quote.minor_units(), minor_units);
        assert_eq!(u128::from(Uint128::from(quote)), minor_units);
    }
}

#[test]
fn a_zero_quantity_converts_but_is_not_a_valid_order_size() {
    let quantity = BaseQuantity::from(wire(0));
    assert_eq!(
        quantity.require_positive(),
        Err(EngineError::QuantityNotPositive)
    );
}

#[test]
fn an_id_is_read_from_both_halves() {
    let value = (0xDEAD_BEEFu128 << 64) | 0xFEED;
    assert_eq!(required_id(Some(Id::from(value))), Ok(value));
    assert_eq!(required_id(None), Err(EngineError::RequiredFieldAbsent));
}

#[test]
fn a_fee_rate_round_trips_and_accepts_zero() {
    for bps in [0u32, 5, 10, 10_000, u32::MAX] {
        let rate = FeeBps::from(bps);
        assert_eq!(rate.bps(), bps);
        assert_eq!(u32::from(rate), bps);
    }
    // A scalar is never absent, so zero is the only thing "not sent" can be.
    assert_eq!(FeeBps::from(0), FeeBps::ZERO);
}

#[test]
fn a_timestamp_round_trips_through_seconds_and_nanos() {
    for (seconds, nanos) in [(0i64, 0i32), (1_758_000_000, 123), (0, 999_999_999)] {
        let wire_value = WireTimestamp { seconds, nanos };
        let time = SystemTime::try_from(wire_value).unwrap();
        assert_eq!(
            time,
            UNIX_EPOCH + Duration::new(seconds as u64, nanos as u32)
        );

        let back = WireTimestamp::from(time);
        assert_eq!((back.seconds, back.nanos), (seconds, nanos));
    }
}

#[test]
fn a_timestamp_the_system_clock_cannot_hold_is_rejected() {
    fn convert(wire_value: WireTimestamp) -> Result<SystemTime, EngineError> {
        Ok(SystemTime::try_from(wire_value)?)
    }

    // i64::MIN seconds cannot be negated, so SystemTime cannot represent it.
    let too_far_in_the_past = WireTimestamp {
        seconds: i64::MIN,
        nanos: 0,
    };
    assert_eq!(
        convert(too_far_in_the_past),
        Err(EngineError::TimestampOutOfRange)
    );
}

#[test]
fn an_absent_timestamp_is_rejected_when_required() {
    let absent: Option<WireTimestamp> = None;
    assert_eq!(required(absent), Err(EngineError::RequiredFieldAbsent));
}

#[test]
fn market_decimals_are_read_by_name_not_by_position() {
    // base 6, quote 2, price 3  ->  k = 3 + 6 - 2 = 7. All three differ, so the
    // quote decimals cannot land in another slot and still give 7.
    let scale = QuoteScale::try_from(&market(6, 2, 3)).unwrap();
    assert_eq!(scale.exponent(), 7);
}

#[test]
fn the_btc_usd_market_gives_the_contracts_example() {
    // base 8 (satoshi), quote 2 (cents), price 2  ->  k = 8
    let scale = QuoteScale::try_from(&market(8, 2, 2)).unwrap();
    assert_eq!(scale.exponent(), 8);

    // Half a bitcoin at $29,000 is $14,500.00.
    let price = Price::try_from(wire(2_900_000)).unwrap();
    let base = BaseQuantity::from(wire(50_000_000));
    assert_eq!(
        scale.quote_for(price, base),
        Ok(QuoteQuantity::from_minor_units(1_450_000))
    );
}

#[test]
fn a_market_the_formula_cannot_serve_is_rejected() {
    // base 2, quote 5, price 2  ->  k = 2 + 2 - 5, below zero
    assert_eq!(
        QuoteScale::try_from(&market(2, 5, 2)),
        Err(EngineError::DecimalsOutOfRange)
    );

    // k above 38 does not fit a u128
    assert_eq!(
        QuoteScale::try_from(&market(0, 0, 39)),
        Err(EngineError::DecimalsOutOfRange)
    );
}

#[test]
fn a_whole_order_priced_from_wire_values() -> Result<(), EngineError> {
    let scale = QuoteScale::try_from(&market(8, 2, 2))?;

    let limit_price: Option<Uint128> = Some(wire(2_900_000));
    let base_quantity: Option<Uint128> = Some(wire(12_345));
    let taker_fee_bps: u32 = 10;

    let price = Price::try_from(required(limit_price)?)?;
    let base = BaseQuantity::from(required(base_quantity)?).require_positive()?;

    // 2_900_000 * 12_345 / 10^8 = 358.005, floored to 358 cents.
    let quote = scale.quote_for(price, base)?;
    assert_eq!(quote, QuoteQuantity::from_minor_units(358));

    // 10 bps of 358 is 0.358, floored to 0.
    let fee = FeeBps::from(taker_fee_bps).fee_on(quote)?;
    assert_eq!(fee, QuoteQuantity::ZERO);

    assert_eq!(u128::from(Uint128::from(quote)), 358);
    assert_eq!(u128::from(Uint128::from(fee)), 0);
    Ok(())
}
