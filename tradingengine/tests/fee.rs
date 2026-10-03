use tradingengine::error::EngineError;
use tradingengine::fee::FeeBps;
use tradingengine::quantity::QuoteQuantity;

fn quote(minor_units: u128) -> QuoteQuantity {
    QuoteQuantity::from_minor_units(minor_units)
}

#[test]
fn bps_round_trips() {
    assert_eq!(FeeBps::from_bps(25).bps(), 25);
}

#[test]
fn zero_rate_charges_nothing() {
    assert_eq!(
        FeeBps::ZERO.fee_on(quote(1_000_000)),
        Ok(QuoteQuantity::ZERO)
    );
}

#[test]
fn ten_bps_of_ten_thousand_dollars_is_ten_dollars() {
    // $10,000.00 in cents at 0.10%
    assert_eq!(
        FeeBps::from_bps(10).fee_on(quote(1_000_000)),
        Ok(quote(1_000))
    );
}

#[test]
fn rounds_down_never_to_nearest() {
    // 1 bps of 19_999 is 1.9999, floored to 1
    assert_eq!(FeeBps::from_bps(1).fee_on(quote(19_999)), Ok(quote(1)));
    // and of 9_999 is 0.9999, floored to 0
    assert_eq!(
        FeeBps::from_bps(1).fee_on(quote(9_999)),
        Ok(QuoteQuantity::ZERO)
    );
}

#[test]
fn full_rate_is_the_whole_amount() {
    assert_eq!(
        FeeBps::from_bps(FeeBps::DENOMINATOR).fee_on(quote(u128::MAX)),
        Ok(quote(u128::MAX))
    );
}

#[test]
fn large_amount_does_not_overflow_the_intermediate() {
    // u128::MAX * 5_000 is too big for a u128 before the division.
    // 5_000 bps is 50%, so the fee is half of u128::MAX, rounded down.
    let half_of_the_biggest_amount = u128::MAX / 2;

    let result = FeeBps::from_bps(5_000).fee_on(quote(u128::MAX));

    assert_eq!(result, Ok(quote(half_of_the_biggest_amount)));
}

#[test]
fn rate_above_100_percent_can_overflow() {
    assert_eq!(
        FeeBps::from_bps(20_000).fee_on(quote(u128::MAX)),
        Err(EngineError::Overflow)
    );
}
