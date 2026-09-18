use crate::error::{EngineError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity(u128);

impl Quantity {
    pub const ZERO: Self = Self(0);

    pub const fn from_minor_units(minor_units: u128) -> Self {
        Quantity(minor_units)
    }

    pub fn minor_units(self) -> u128 {
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
        if other.0 > self.0 {
            Err(EngineError::QuantityNegative)
        } else {
            Ok(Quantity(self.0 - other.0))
        }
    }
}
