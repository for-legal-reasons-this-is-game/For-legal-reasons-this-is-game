use crate::error::{EngineError, Result};

/// A price in minor units: the smallest unit of a currency, like cents for EUR and USD,
/// or satoshis for BTC. See [`Price::from_minor_units`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(u128);
impl Price {
    pub fn from_minor_units(minor_units: u128) -> Result<Self> {
        if minor_units > 0 {
            Ok(Price(minor_units))
        } else {
            Err(EngineError::PriceNotPositive)
        }
    }

    pub fn minor_units(self) -> u128 {
        self.0
    }
}
