use proto::place_order_request::Amount;
use proto::{Id, Market, TimestampError, Uint128};

use crate::error::{EngineError, Result};
use crate::fee::FeeBps;
use crate::order::Side;
use crate::price::Price;
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::quote::QuoteScale;

/// Makes sure an optional field from the backend is set, and rejects it if it's empty.
///
/// A field can come in as `Some(0)`. That is accepted here, and checked later by the type
/// the field belongs to: for example, a zero [`Price`] is rejected, but a zero
/// [`BaseQuantity`] is sometimes accepted.
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

/// Zero is allowed here. Whether a zero quantity is correct is checked later.
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

/// Turns a [`TimestampError`] into an [`EngineError`].
///
/// Timestamps use [`SystemTime`](std::time::SystemTime) and the conversions that
/// `prost-types` already has. This only turns their error into ours, so that
/// `SystemTime::try_from(...)?` works in the engine.
impl From<TimestampError> for EngineError {
    fn from(_: TimestampError) -> EngineError {
        EngineError::TimestampOutOfRange
    }
}

/// Reads the decimals by name.
///
/// [`QuoteScale::new`] takes them by position, as (price, base, quote), while [`Market`]
/// declares them as base, quote, price. So this is the one call site that has to get the
/// order right.
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

/// turns the proto side into our [`Side`]
///
/// `check_place_order` already rejects `Unspecified`
pub fn side_from_proto(side: proto::Side) -> Result<Side> {
    match side {
        proto::Side::Buy => Ok(Side::Buy),
        proto::Side::Sell => Ok(Side::Sell),
        proto::Side::Unspecified => Err(EngineError::RequiredFieldAbsent),
    }
}

/// takes the `base_quantity` out of an orders amount
///
/// a `quote_quantity` or no amount is an error. zero is let through because the order
/// rejects that itself
pub fn base_quantity_from_proto(amount: Option<Amount>) -> Result<BaseQuantity> {
    match amount {
        Some(Amount::BaseQuantity(quantity)) => Ok(BaseQuantity::from(quantity)),
        Some(Amount::QuoteQuantity(_)) => Err(EngineError::BadOrderParams(
            "this order needs base_quantity, not quote_quantity",
        )),
        None => Err(EngineError::RequiredFieldAbsent),
    }
}
