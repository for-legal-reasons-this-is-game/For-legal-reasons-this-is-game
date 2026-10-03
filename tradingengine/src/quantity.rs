use crate::error::{EngineError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaseQuantity(u128);

impl BaseQuantity {
    pub const ZERO: BaseQuantity = BaseQuantity(0);

    pub fn from_minor_units(minor_units: u128) -> BaseQuantity {
        BaseQuantity(minor_units)
    }

    pub fn minor_units(self) -> u128 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn require_positive(self) -> Result<BaseQuantity> {
        if self.is_zero() {
            return Err(EngineError::QuantityNotPositive);
        }
        Ok(self)
    }

    pub fn checked_add(self, other: BaseQuantity) -> Result<BaseQuantity> {
        match self.0.checked_add(other.0) {
            Some(sum) => Ok(BaseQuantity(sum)),
            None => Err(EngineError::Overflow),
        }
    }

    pub fn checked_sub(self, other: BaseQuantity) -> Result<BaseQuantity> {
        if other.0 > self.0 {
            return Err(EngineError::QuantityNegative);
        }
        Ok(BaseQuantity(self.0 - other.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QuoteQuantity(u128);

impl QuoteQuantity {
    pub const ZERO: QuoteQuantity = QuoteQuantity(0);

    pub fn from_minor_units(minor_units: u128) -> QuoteQuantity {
        QuoteQuantity(minor_units)
    }

    pub fn minor_units(self) -> u128 {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn require_positive(self) -> Result<QuoteQuantity> {
        if self.is_zero() {
            return Err(EngineError::QuantityNotPositive);
        }
        Ok(self)
    }

    pub fn checked_add(self, other: QuoteQuantity) -> Result<QuoteQuantity> {
        match self.0.checked_add(other.0) {
            Some(sum) => Ok(QuoteQuantity(sum)),
            None => Err(EngineError::Overflow),
        }
    }

    pub fn checked_sub(self, other: QuoteQuantity) -> Result<QuoteQuantity> {
        if other.0 > self.0 {
            return Err(EngineError::QuantityNegative);
        }
        Ok(QuoteQuantity(self.0 - other.0))
    }
}
