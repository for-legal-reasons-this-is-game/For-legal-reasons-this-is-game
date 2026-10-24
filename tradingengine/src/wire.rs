use proto::{Id, Market, Timestamp as WireTimestamp, Uint128};

use crate::error::{EngineError, Result};
use crate::fee::FeeBps;
use crate::price::Price;
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::quote::QuoteScale;
use crate::timestamp::Timestamp;

// proto3 delivers a message field that was never sent and one sent as its zero
// value identically, as None, so absence is its own rejection.
pub fn required<T>(field: Option<T>) -> Result<T> {
    field.ok_or(EngineError::RequiredFieldAbsent)
}

// Raw, because the typed identifiers are Layer 1 and still pre-#21.
pub fn required_id(field: Option<Id>) -> Result<u128> {
    Ok(required(field)?.into())
}

impl TryFrom<Uint128> for Price {
    type Error = EngineError;

    fn try_from(value: Uint128) -> Result<Price> {
        Price::from_minor_units(value.into())
    }
}

impl TryFrom<Option<Uint128>> for Price {
    type Error = EngineError;

    fn try_from(field: Option<Uint128>) -> Result<Price> {
        required(field)?.try_into()
    }
}

impl From<Price> for Uint128 {
    fn from(price: Price) -> Uint128 {
        price.minor_units().into()
    }
}

// Infallible: a zero quantity is meaningful on the wire, and only a request
// quantity has to be positive. That check is `require_positive`.
impl From<Uint128> for BaseQuantity {
    fn from(value: Uint128) -> BaseQuantity {
        BaseQuantity::from_minor_units(value.into())
    }
}

impl TryFrom<Option<Uint128>> for BaseQuantity {
    type Error = EngineError;

    fn try_from(field: Option<Uint128>) -> Result<BaseQuantity> {
        Ok(required(field)?.into())
    }
}

impl From<BaseQuantity> for Uint128 {
    fn from(quantity: BaseQuantity) -> Uint128 {
        quantity.minor_units().into()
    }
}

impl From<Uint128> for QuoteQuantity {
    fn from(value: Uint128) -> QuoteQuantity {
        QuoteQuantity::from_minor_units(value.into())
    }
}

impl TryFrom<Option<Uint128>> for QuoteQuantity {
    type Error = EngineError;

    fn try_from(field: Option<Uint128>) -> Result<QuoteQuantity> {
        Ok(required(field)?.into())
    }
}

impl From<QuoteQuantity> for Uint128 {
    fn from(quantity: QuoteQuantity) -> Uint128 {
        quantity.minor_units().into()
    }
}

impl From<u32> for FeeBps {
    fn from(bps: u32) -> FeeBps {
        FeeBps::from_bps(bps)
    }
}

impl From<FeeBps> for u32 {
    fn from(rate: FeeBps) -> u32 {
        rate.bps()
    }
}

impl TryFrom<WireTimestamp> for Timestamp {
    type Error = EngineError;

    fn try_from(value: WireTimestamp) -> Result<Timestamp> {
        Timestamp::from_unix_parts(value.seconds, value.nanos)
    }
}

impl TryFrom<Option<WireTimestamp>> for Timestamp {
    type Error = EngineError;

    fn try_from(field: Option<WireTimestamp>) -> Result<Timestamp> {
        required(field)?.try_into()
    }
}

impl From<Timestamp> for WireTimestamp {
    fn from(timestamp: Timestamp) -> WireTimestamp {
        const NANOS_PER_SECOND: u64 = 1_000_000_000;
        let nanos = timestamp.unix_nanos();

        // Both casts are lossless. The engine's range stops at the year 2554,
        // so the seconds fit an i64, and the remainder is below one second.
        WireTimestamp {
            seconds: (nanos / NANOS_PER_SECOND) as i64,
            nanos: (nanos % NANOS_PER_SECOND) as i32,
        }
    }
}

// Reads the decimals by name. `QuoteScale::new` takes them positionally as
// (price, base, quote) and `Market` declares them base, quote, price, so this
// is the one call site that has to be right.
impl TryFrom<&Market> for QuoteScale {
    type Error = EngineError;

    fn try_from(market: &Market) -> Result<QuoteScale> {
        QuoteScale::new(
            market.price_decimals,
            market.base_decimals,
            market.quote_decimals,
        )
    }
}
