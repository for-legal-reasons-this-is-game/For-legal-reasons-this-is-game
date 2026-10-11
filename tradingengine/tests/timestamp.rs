use tradingengine::error::EngineError;
use tradingengine::timestamp::Timestamp;

#[test]
fn nanos_round_trip() {
    assert_eq!(Timestamp::from_unix_nanos(42).unix_nanos(), 42);
}

#[test]
fn parts_combine_into_nanos() {
    assert_eq!(
        Timestamp::from_unix_parts(1_758_000_000, 123),
        Ok(Timestamp::from_unix_nanos(1_758_000_000_000_000_123))
    );
}

#[test]
fn the_epoch_itself_is_valid() {
    assert_eq!(
        Timestamp::from_unix_parts(0, 0),
        Ok(Timestamp::from_unix_nanos(0))
    );
}

#[test]
fn rejects_before_1970() {
    assert_eq!(
        Timestamp::from_unix_parts(-1, 0),
        Err(EngineError::TimestampOutOfRange)
    );
}

#[test]
fn rejects_negative_nanos() {
    assert_eq!(
        Timestamp::from_unix_parts(1, -1),
        Err(EngineError::TimestampOutOfRange)
    );
}

#[test]
fn rejects_a_whole_second_of_nanos() {
    assert_eq!(
        Timestamp::from_unix_parts(1, 1_000_000_000),
        Err(EngineError::TimestampOutOfRange)
    );
}

#[test]
fn rejects_past_u64_nanos() {
    assert_eq!(
        Timestamp::from_unix_parts(i64::MAX, 0),
        Err(EngineError::TimestampOutOfRange)
    );
}

#[test]
fn later_compares_above_earlier() {
    // GTD validation asks "is expire_time after now?"
    assert!(Timestamp::from_unix_nanos(2) > Timestamp::from_unix_nanos(1));
}
