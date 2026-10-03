use crate::error::{EngineError, Result};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn from_unix_nanos(nanos: u64) -> Timestamp {
        Timestamp(nanos)
    }

    pub fn unix_nanos(self) -> u64 {
        self.0
    }

    pub fn from_unix_parts(seconds: i64, nanos: i32) -> Result<Timestamp> {
        // Negative seconds means before 1970.
        if seconds < 0 {
            return Err(EngineError::TimestampOutOfRange);
        }

        // The extra nanoseconds must be less than one whole second.
        if nanos < 0 || nanos as u64 >= NANOS_PER_SECOND {
            return Err(EngineError::TimestampOutOfRange);
        }

        let seconds = seconds as u64;
        let nanos = nanos as u64;

        // seconds * 1_000_000_000 can overflow a u64.
        let whole_seconds_in_nanos = match seconds.checked_mul(NANOS_PER_SECOND) {
            Some(value) => value,
            None => return Err(EngineError::TimestampOutOfRange),
        };

        match whole_seconds_in_nanos.checked_add(nanos) {
            Some(total) => Ok(Timestamp(total)),
            None => Err(EngineError::TimestampOutOfRange),
        }
    }
}
