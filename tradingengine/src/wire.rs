use proto::{Id, Market, TimestampError, Uint128};

use crate::error::{EngineError, Result};
use crate::fee::FeeBps;
use crate::price::Price;
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::quote::QuoteScale;

// some incoming field from backend comes as and option the required()
// makes sure these fields are not empty and if they are they are rejecte
// a field make come in as some(0) and is accepted here but is rejected
// down the line by the responsible field for example price(0) is
// rejected but quantity(0) is sometimes accepted
pub fn required<T>(field: Option<T>) -> Result<T> {
    field.ok_or(EngineError::RequiredFieldAbsent)
}

pub fn required_id(field: Option<Id>) -> Result<u128> {
    Ok(required(field)?.into())
}

impl TryFrom<Uint128> for Price {
    type Error = EngineError;

    fn try_from(value: Uint128) -> Result<Price> {
        Price::from_minor_units(value.into())
    }
}

impl From<Price> for Uint128 {
    fn from(price: Price) -> Uint128 {
        price.minor_units().into()
    }
}

// Zero cna be inserted in case of this value and is checked down the line
// if the zero value is correct or not
impl From<Uint128> for BaseQuantity {
    fn from(value: Uint128) -> BaseQuantity {
        BaseQuantity::from_minor_units(value.into())
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

// Timestamps use std::time::SystemTime and the conversions prost-types already has.
// This only turns its error into ours so `SystemTime::try_from(...)?` works in the engine.
impl From<TimestampError> for EngineError {
    fn from(_: TimestampError) -> EngineError {
        EngineError::TimestampOutOfRange
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
