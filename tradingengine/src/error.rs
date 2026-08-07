#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    PriceNotPositive,
    QuantityNotPositive,
    QuantityNegative,
    IdempotencyKeyInvalid,
    FillExceedsRemaining,
    Overflow,
}

pub type Result<T> = std::result::Result<T, EngineError>;

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PriceNotPositive => write!(f, "price must be greater than zero"),
            Self::QuantityNegative => write!(f, "quantity must be greater or equal to zero"),
            Self::QuantityNotPositive => write!(f, "quantity must be greater than zero"),
            Self::IdempotencyKeyInvalid => write!(
                f,
                "idempotency key must be 1 to {} characters of [A-Za-z0-9_-]",
                crate::ids::IdempotencyKey::MAX_LEN
            ),
            Self::FillExceedsRemaining => {
                write!(f, "fill quantity must not exceed remaining quantity")
            }
            Self::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}

impl std::error::Error for EngineError {}
