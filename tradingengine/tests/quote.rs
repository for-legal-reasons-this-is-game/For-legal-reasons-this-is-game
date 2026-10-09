use tradingengine::error::EngineError;
use tradingengine::price::Price;
use tradingengine::quantity::{BaseQuantity, QuoteQuantity};
use tradingengine::quote::QuoteScale;

fn price(minor_units: u128) -> Price {
    Price::from_minor_units(minor_units).unwrap()
}

fn base(minor_units: u128) -> BaseQuantity {
    BaseQuantity::from_minor_units(minor_units)
}

fn quote(minor_units: u128) -> QuoteQuantity {
    QuoteQuantity::from_minor_units(minor_units)
}

fn scale(price_decimals: u32, base_decimals: u32, quote_decimals: u32) -> QuoteScale {
    QuoteScale::new(price_decimals, base_decimals, quote_decimals).unwrap()
}

#[test]
fn exponent_is_price_plus_base_minus_quote() {
    // 2 + 8 - 2 = 8
    let btc_usd = scale(2, 8, 2);
    assert_eq!(btc_usd.exponent(), 8);

    let no_decimals = scale(0, 0, 0);
    assert_eq!(no_decimals.exponent(), 0);
}

// `Market` declares its decimals as base, quote, price. `new` takes them as
// price, base, quote. Every other test here uses decimals where two of the
// three are equal, so passing them in the wrong order still gives the right
// answer and nothing fails. These three are all different, so the quote
// decimals cannot be mistaken for either of the others.
//
// Only the quote slot matters: the formula adds price and base, so swapping
// those two cannot change the result.
#[test]
fn three_different_decimals_pin_the_argument_order() {
    // 3 + 6 - 2 = 7
    assert_eq!(scale(3, 6, 2).exponent(), 7);

    // quote_decimals in the price slot: 2 + 6 - 3 = 5
    assert_eq!(scale(2, 6, 3).exponent(), 5);

    // quote_decimals in the base slot: 3 + 2 - 6 is below zero
    assert_eq!(
        QuoteScale::new(3, 2, 6),
        Err(EngineError::DecimalsOutOfRange)
    );
}

#[test]
fn a_quote_with_three_different_decimals() {
    // price 12.345 at 3 decimals, base 2.0 units at 6 decimals,
    // quote in cents at 2 decimals. 12.345 * 2.0 = 24.69, so 2469 cents.
    let market = scale(3, 6, 2);
    assert_eq!(
        market.quote_for(price(12_345), base(2_000_000)),
        Ok(quote(2_469))
    );
}

#[test]
fn rejects_a_negative_exponent() {
    // 2 + 2 - 5 = -1, which is not allowed
    let result = QuoteScale::new(2, 2, 5);
    assert_eq!(result, Err(EngineError::DecimalsOutOfRange));
}

#[test]
fn accepts_the_largest_exponent_that_fits() {
    let biggest = scale(38, 0, 0);
    assert_eq!(biggest.exponent(), 38);
}

#[test]
fn rejects_an_exponent_past_128_bits() {
    // 10^39 does not fit in a u128
    let result = QuoteScale::new(39, 0, 0);
    assert_eq!(result, Err(EngineError::DecimalsOutOfRange));
}

#[test]
fn rejects_decimals_that_overflow_u32() {
    let result = QuoteScale::new(u32::MAX, 1, 0);
    assert_eq!(result, Err(EngineError::DecimalsOutOfRange));
}

#[test]
fn half_a_bitcoin_at_29_000_dollars() {
    // BTC-USD: 8 base decimals (satoshi), 2 quote decimals (cents),
    // and the price is in cents, so 2 price decimals.
    let btc_usd = scale(2, 8, 2);

    let price_29_000_dollars = price(2_900_000);
    let half_a_bitcoin = base(50_000_000);

    let result = btc_usd.quote_for(price_29_000_dollars, half_a_bitcoin);

    // $14,500.00 is 1_450_000 cents
    assert_eq!(result, Ok(quote(1_450_000)));
}

#[test]
fn rounds_down_never_to_nearest() {
    let one_decimal = scale(1, 0, 0);

    // 15 / 10 = 1.5, rounded down to 1
    assert_eq!(one_decimal.quote_for(price(15), base(1)), Ok(quote(1)));

    // 19 / 10 = 1.9, rounded down to 1
    assert_eq!(one_decimal.quote_for(price(19), base(1)), Ok(quote(1)));
}

#[test]
fn can_round_all_the_way_to_zero() {
    // 3 * 3 / 10 = 0.9, rounded down to 0.
    // The backend's minimum order size should stop this from happening.
    let one_decimal = scale(1, 0, 0);
    let result = one_decimal.quote_for(price(3), base(3));
    assert_eq!(result, Ok(QuoteQuantity::ZERO));
}

#[test]
fn zero_base_quotes_zero() {
    let btc_usd = scale(2, 8, 2);
    let result = btc_usd.quote_for(price(1), BaseQuantity::ZERO);
    assert_eq!(result, Ok(QuoteQuantity::ZERO));
}

#[test]
fn product_past_128_bits_still_divides_exactly() {
    // 10^30 * 10^30 = 10^60, which is too big for a u128.
    // But 10^60 / 10^38 = 10^22, which fits.
    let ten_to_the_30 = 10u128.pow(30);
    let ten_to_the_22 = 10u128.pow(22);

    let result = scale(38, 0, 0).quote_for(price(ten_to_the_30), base(ten_to_the_30));

    assert_eq!(result, Ok(quote(ten_to_the_22)));
}

#[test]
fn product_past_128_bits_rounds_down() {
    // (10^30 + 1) * (10^30 + 1) / 10^38 is 10^22 plus a tiny fraction.
    // The fraction is dropped.
    let a_bit_more_than_ten_to_the_30 = 10u128.pow(30) + 1;
    let ten_to_the_22 = 10u128.pow(22);

    let result = scale(38, 0, 0).quote_for(
        price(a_bit_more_than_ten_to_the_30),
        base(a_bit_more_than_ten_to_the_30),
    );

    assert_eq!(result, Ok(quote(ten_to_the_22)));
}

#[test]
fn result_of_exactly_u128_max_fits() {
    // u128::MAX * 10^38 / 10^38 = u128::MAX
    let ten_to_the_38 = 10u128.pow(38);

    let result = scale(38, 0, 0).quote_for(price(u128::MAX), base(ten_to_the_38));

    assert_eq!(result, Ok(quote(u128::MAX)));
}

#[test]
fn result_past_u128_max_is_overflow() {
    // u128::MAX * (10^38 + 1) / 10^38 is a little more than u128::MAX
    let a_bit_more_than_ten_to_the_38 = 10u128.pow(38) + 1;

    let result = scale(38, 0, 0).quote_for(price(u128::MAX), base(a_bit_more_than_ten_to_the_38));

    assert_eq!(result, Err(EngineError::Overflow));
}

#[test]
fn no_division_overflows_on_a_large_product() {
    // With exponent 0 we divide by 1, so u128::MAX * 2 does not fit
    let result = scale(0, 0, 0).quote_for(price(u128::MAX), base(2));
    assert_eq!(result, Err(EngineError::Overflow));
}

#[test]
fn divisor_near_the_top_bit_divides_correctly() {
    // This checks the tricky step in the long division, where the
    // remainder briefly needs more than 128 bits.
    // (2^127 + 1) * 10^38 / 10^38 = 2^127 + 1
    let big = 2u128.pow(127) + 1;
    let ten_to_the_38 = 10u128.pow(38);

    let result = scale(38, 0, 0).quote_for(price(big), base(ten_to_the_38));

    assert_eq!(result, Ok(quote(big)));
}
