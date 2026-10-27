#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    PriceNotPositive,
    QuantityNotPositive, //quantity can be zero when matching hence 2 error types
    QuantityNegative,
    FillExceedsRemaining,
    QuantityNotAboveFilled,
    DuplicateOrderId,
    OrderNotFound,
    OrderNotLive,
    Overflow,
    DecimalsOutOfRange,
    TimestampOutOfRange,
    RequiredFieldAbsent,
    MarketSymbolEmpty,
    HasOpenOrders,
}

pub type Result<T> = std::result::Result<T, EngineError>;

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PriceNotPositive => write!(f, "price must be greater than zero"),
            Self::QuantityNegative => write!(f, "quantity must be greater or equal to zero"),
            Self::QuantityNotPositive => write!(f, "quantity must be greater than zero"),
            Self::FillExceedsRemaining => {
                write!(f, "fill quantity must not exceed remaining quantity")
            }
            Self::QuantityNotAboveFilled => {
                write!(f, "new quantity must be above the quantity already filled")
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
            Self::TimestampOutOfRange => {
                write!(
                    f,
                    "timestamp is outside the range the system clock can hold"
                )
            }
            Self::RequiredFieldAbsent => {
                write!(f, "a required message field was not set")
            }
            Self::MarketSymbolEmpty => write!(f, "the market symbol must not be empty"),
            Self::HasOpenOrders => write!(
                f,
                "the decimals cannot change while the market has orders in its book"
            ),
        }
    }
}

impl std::error::Error for EngineError {}
