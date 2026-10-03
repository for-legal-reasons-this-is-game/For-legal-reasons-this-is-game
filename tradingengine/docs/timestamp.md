# Timestamp (`src/timestamp.rs`)

A `Timestamp` is a point in time. It is stored as a `u64` count of nanoseconds since 1 January 1970. That starting point is called the "Unix epoch".

## Why the engine needs time at all

The older data model said the engine never reads a clock. The contract changes that. A GTD order ("good till date") has an `expire_time`, and the engine must expire it. When the order is placed, `expire_time` must be in the future. Both of those need a time.

**Matching still does not read a clock.** The data model passes `now` into the match function as an argument, the same way a command is passed in. Replay the same commands with the same `now` values, and you get the same result. A `Timestamp` always comes from outside: from a request (`expire_time`, `sent_at`) or passed in as `now`.

**An example.** Alice places a GTD order that should expire at 12:00. The request carries `expire_time`. Later, a command arrives with `now` set to 12:01. The engine compares the two timestamps and sees the order has expired. If it read the real clock instead, a replay next week would see a different `now` and could reach a different result.

## Why `u64` nanoseconds

- **Same unit as `engine_epoch`.** The contract defines that as the time the process started, in Unix nanoseconds.
- **Comparing is plain integer comparison.** "Is `expire_time` after `now`?" is just `>`.
- **Range.** A `u64` of nanoseconds reaches the year 2554. Being unsigned, it cannot hold a time before 1970. An exchange needs neither.

## Two constructors

- **`from_unix_nanos`** never fails. Every `u64` is a valid time. `unix_nanos()` gives the number back out.
- **`from_unix_parts(seconds, nanos)`** takes the two fields of the contract's `google.protobuf.Timestamp`: `int64 seconds` and `int32 nanos`. It adds them into one number:
  ```
  seconds = 1_758_000_000, nanos = 123
  ->  1_758_000_000_000_000_123 nanoseconds
  ```
  It returns `TimestampOutOfRange` for:
  - negative seconds, which means before 1970,
  - nanos outside `0..=999_999_999`. Protobuf itself calls these invalid, because they are not less than one second,
  - a total too big for a `u64`, for example `seconds = i64::MAX`.

  It does not need the protobuf crate, so it can land before the gRPC layer does.

## The derives

`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord`.
- **`Ord`**, because expiry is a comparison.
- **No `Hash`.** Nothing is keyed by time yet. A GTD timer will more likely want a `BTreeMap<Timestamp, _>`, which needs `Ord`, not `Hash`.
