#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    PriceNotPositive,
    QuantityNotPositive,
    QuantityNegative,
    IdempotencyKeyInvalid,
    FillExceedsRemaining,
    DuplicateOrderId,
    OrderNotFound,
    OrderNotLive,
    Overflow,
    DecimalsOutOfRange,
    TimestampOutOfRange,
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
            Self::DuplicateOrderId => write!(f, "an order with this id is already in the book"),
            Self::OrderNotFound => write!(f, "no order with this id is in the book"),
            Self::OrderNotLive => write!(f, "the order is already in a terminal state"),
            Self::Overflow => write!(f, "arithmetic overflow"),
            Self::DecimalsOutOfRange => write!(
                f,
                "price_decimals + base_decimals - quote_decimals must be between 0 and {}",
                crate::quote::QuoteScale::MAX_EXPONENT
            ),
            Self::TimestampOutOfRange => write!(
                f,
                "timestamp must be a valid time between 1970 and the year 2554"
            ),
        }
    }
}

impl std::error::Error for EngineError {}
