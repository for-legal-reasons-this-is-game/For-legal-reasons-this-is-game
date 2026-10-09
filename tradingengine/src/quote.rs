use crate::error::{EngineError, Result};
use crate::price::Price;
use crate::quantity::{BaseQuantity, QuoteQuantity};
use crate::wide::mul_div_floor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuoteScale {
    exponent: u32,
    divisor: u128,
}

impl QuoteScale {
    pub const MAX_EXPONENT: u32 = 38;

    pub fn new(price_decimals: u32, base_decimals: u32, quote_decimals: u32) -> Result<QuoteScale> {
        // Add first. Very large decimals could overflow a u32.
        let sum = match price_decimals.checked_add(base_decimals) {
            Some(sum) => sum,
            None => return Err(EngineError::DecimalsOutOfRange),
        };

        // Then subtract. A negative exponent is not allowed.
        if quote_decimals > sum {
            return Err(EngineError::DecimalsOutOfRange);
        }
        let exponent = sum - quote_decimals;

        // 10^exponent must fit in a u128.
        if exponent > QuoteScale::MAX_EXPONENT {
            return Err(EngineError::DecimalsOutOfRange);
        }

        let divisor = 10u128.pow(exponent);
        Ok(QuoteScale { exponent, divisor })
    }

    pub fn exponent(self) -> u32 {
        self.exponent
    }

    pub fn quote_for(self, price: Price, base: BaseQuantity) -> Result<QuoteQuantity> {
        match mul_div_floor(price.minor_units(), base.minor_units(), self.divisor) {
            Some(amount) => Ok(QuoteQuantity::from_minor_units(amount)),
            None => Err(EngineError::Overflow),
        }
    }
}
