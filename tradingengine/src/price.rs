use crate::error::{EngineError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(i64);

impl Price {
    pub fn from_minor_units(minor_units: i64) -> Result<Self> {
        if minor_units > 0 {
            Ok(Price(minor_units))
        } else {
            Err(EngineError::PriceNotPositive)
        }
    }

    pub fn minor_units(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
#[path = "price_test.rs"]
mod price_test;
