// the engine holds 2 types of quantities in each trade as per market rule of
// pairs example: btc-usd
// base quantity (BTC) is the first part of the pair and quote quantity (usd) is the second
// the two are in tandem while opening an order based off of the price value
// when buying the client has the quote part of the market and recieves base in exchange
// when sell the client has the base part and recieves quote

use crate::error::{EngineError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaseQuantity(u128);

impl BaseQuantity {
    // quantities can be zero after being filled
    pub const ZERO: BaseQuantity = BaseQuantity(0);

    // minor units in case of quanity means a whole number including the
    // smallest decimal value a currency can hold 5 dollars are saved as 500
    // this scale is set only once per market
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
