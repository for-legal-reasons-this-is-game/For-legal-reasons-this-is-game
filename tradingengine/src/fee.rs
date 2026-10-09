use crate::error::{EngineError, Result};
use crate::quantity::QuoteQuantity;
use crate::wide::mul_div_floor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeBps(u32);

impl FeeBps {
    pub const ZERO: FeeBps = FeeBps(0);

    /// How many basis points make 100%.
    pub const DENOMINATOR: u32 = 10_000;

    pub fn from_bps(bps: u32) -> FeeBps {
        FeeBps(bps)
    }

    pub fn bps(self) -> u32 {
        self.0
    }

    pub fn fee_on(self, quote: QuoteQuantity) -> Result<QuoteQuantity> {
        let rate = self.0 as u128;
        let denominator = FeeBps::DENOMINATOR as u128;

        match mul_div_floor(quote.minor_units(), rate, denominator) {
            Some(fee) => Ok(QuoteQuantity::from_minor_units(fee)),
            None => Err(EngineError::Overflow),
        }
    }
}
