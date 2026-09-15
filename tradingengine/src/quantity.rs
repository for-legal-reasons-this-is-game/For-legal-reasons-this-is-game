use crate::error::{EngineError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity(i64);

impl Quantity {
    pub const ZERO: Self = Self(0);

    pub fn from_minor_units(minor_units: i64) -> Result<Self> {
        if minor_units < 0 {
            Err(EngineError::QuantityNegative)
        } else {
            Ok(Quantity(minor_units))
        }
    }

    pub fn minor_units(self) -> i64 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }

    pub fn require_positive(self) -> Result<Self> {
        if self.is_zero() {
            Err(EngineError::QuantityNotPositive)
        } else {
            Ok(self)
        }
    }

    pub fn checked_add(self, other: Self) -> Result<Self> {
        let sum = self.0.checked_add(other.0).ok_or(EngineError::Overflow)?;
        Ok(Quantity(sum))
    }

    pub fn checked_sub(self, other: Self) -> Result<Self> {
        let difference = self.0 - other.0;
        if difference < 0 {
            Err(EngineError::QuantityNegative)
        } else {
            Ok(Quantity(difference))
        }
    }
}
