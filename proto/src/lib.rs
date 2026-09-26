pub mod trading {
    pub mod v1 {
        tonic::include_proto!("trading.v1");
    }
}

pub use trading::v1::*;

// seconds: i64 plus nanos: i32. prost-types provides conversions to and from std::time::SystemTime:
//- Going from SystemTime to Timestamp always succeeds, so it uses From.
//- Going from Timestamp to SystemTime can fail for example if the nanoseconds are out of range, so it uses TryFrom. handle the error by rejecting or logging the message.
pub use prost_types::{Timestamp, TimestampError};

impl From<u128> for Uint128 {
    fn from(v: u128) -> Self {
        Self {
            high: (v >> 64) as u64,
            low: v as u64,
        }
    }
}

impl From<Uint128> for u128 {
    fn from(v: Uint128) -> Self {
        ((v.high as u128) << 64) | v.low as u128
    }
}

impl From<u128> for Id {
    fn from(v: u128) -> Self {
        Self {
            high: (v >> 64) as u64,
            low: v as u64,
        }
    }
}

impl From<Id> for u128 {
    fn from(v: Id) -> Self {
        ((v.high as u128) << 64) | v.low as u128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uint128_roundtrip() {
        for v in [
            0u128,
            1,
            u64::MAX as u128,
            (u64::MAX as u128) + 1,
            u128::MAX,
        ] {
            assert_eq!(u128::from(Uint128::from(v)), v);
        }
    }

    #[test]
    fn uint128_halves() {
        let v = Uint128::from((7u128 << 64) | 42);
        assert_eq!((v.high, v.low), (7, 42));
    }

    #[test]
    fn id_roundtrip() {
        for v in [0u128, u128::MAX, 0xDEAD_BEEF << 64] {
            assert_eq!(u128::from(Id::from(v)), v);
        }
    }
}
