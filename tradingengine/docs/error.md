# `src/error.rs`: errors

This file holds `EngineError`, the one error type for the whole crate, plus a short `Result` alias. Every operation that can fail returns one of its variants instead of crashing.

> **Changing under #21.** The trading contract moves command outcomes, like "price must be positive", into its own reason codes: `RejectReason`, `CancelReason` and `ConfigRejectReason`. `EngineError` then stays only for internal rule breaks. See the "Today / After #21" table in [DATA_MODEL.md](DATA_MODEL.md). This doc describes the code as it is now.

## Why every fallible operation returns `Result`

**The engine must never panic on input a client can influence.** A panic is Rust crashing on purpose. If a bad price could crash the process, anyone who can reach the API could stop the market for everyone.

**An example.** Alice sends a buy order with a price of 0 cents.

- **With a panic,** the engine process dies. Bob's orders, and everyone else's, stop too.
- **With `Result`,** `Price::from_minor_units(0)` returns `Err(EngineError::PriceNotPositive)`. The engine rejects Alice's order and carries on.

## Why one enum, not one per module

Later, the API layer must turn every engine failure into an HTTP or gRPC response. With one enum, that is a single `match` that covers every case. With several error types, it would be several conversions, and a new variant could easily be missed.

**The cost.** This one enum knows about ideas from all over the crate: prices, quantities, the book, timestamps. That is worth it here.

## The variants

| variant | raised by | message |
|---|---|---|
| `PriceNotPositive` | `Price::from_minor_units` | price must be greater than zero |
| `QuantityNotPositive` | `require_positive` on `BaseQuantity` or `QuoteQuantity` | quantity must be greater than zero |
| `QuantityNegative` | `checked_sub` on `BaseQuantity` or `QuoteQuantity` | quantity must be greater or equal to zero |
| `IdempotencyKeyInvalid` | `IdempotencyKey::new` | idempotency key must be 1 to 64 characters of [A-Za-z0-9_-] |
| `FillExceedsRemaining` | `RestingOrder::fill` | fill quantity must not exceed remaining quantity |
| `DuplicateOrderId` | `Book::insert`, `Book::amend` | an order with this id is already in the book |
| `OrderNotFound` | `Book::remove`, and so `Book::amend` | no order with this id is in the book |
| `OrderNotLive` | `RestingOrder::cancel` | the order is already in a terminal state |
| `Overflow` | `checked_add` on either quantity, `QuoteScale::quote_for`, `FeeBps::fee_on` | arithmetic overflow |
| `DecimalsOutOfRange` | `QuoteScale::new` | price_decimals + base_decimals - quote_decimals must be between 0 and 38 |
| `TimestampOutOfRange` | `Timestamp::from_unix_parts` | timestamp must be a valid time between 1970 and the year 2554 |

Two messages are built from constants, so the number in the text always matches the code:

- the idempotency key message uses `IdempotencyKey::MAX_LEN`, which is 64
- the decimals message uses `QuoteScale::MAX_EXPONENT`, which is 38

## Why quantity has two variants

A quantity may be zero, but never negative. Some places, like the size of a new order, also need it to be more than zero. Those are two different rules:

- **`QuantityNegative`.** Bob's order has 0.3 BTC left (30_000_000 satoshis). Subtracting 0.5 BTC (50_000_000) would go below zero. `checked_sub` refuses.
- **`QuantityNotPositive`.** Alice places an order for 0 satoshis. Zero is a valid quantity, but not a valid order size. `require_positive` refuses.

One variant for both would give a message that is wrong for one of the two cases.

`Price` does not have this problem. It rejects zero and negative alike, so one variant is accurate for both.

## Why `Overflow` has a general message

Overflow means a result is too big to fit in the integer type. It is not only about adding, and not only about quantities. The quote and fee maths raise it too. A message must be true for every path that produces it, so it just says "arithmetic overflow".

## Naming rule

A variant name says **what is wrong**, not just that something is wrong. `PriceNotPositive`, not `InvalidPrice`.

The test: a good name writes the user's message for you. "Price must be greater than zero" falls straight out of `PriceNotPositive`. From `InvalidPrice`, you would have to read the code to find out what was invalid.

## The `Result` alias

```rust
pub type Result<T> = std::result::Result<T, EngineError>;
```

This lets a signature say `-> Result<Price>` instead of `-> Result<Price, EngineError>`.

The right-hand side must spell out `std::result::Result`. Writing `Result<T, EngineError>` there would point at the alias being defined, which is circular.

**Watch out.** Inside this crate, `Result` now means this alias, and it takes **one** type parameter. Anything that returns a different error type must write the full path. The `Display` impl below is exactly that case. Its method returns `std::fmt::Result`, which is a third, unrelated `Result` that takes no type parameters at all.

## Derives

`EngineError` derives `Debug, Clone, PartialEq, Eq`.

- **`Debug`** for test output and for `unwrap`.
- **`Clone`** because errors get copied into logs and responses.
- **`PartialEq` and `Eq`** so tests can write `assert_eq!(result, Err(EngineError::Overflow))` instead of a `match`. The test files rely on this a lot.

## `Display` and `Error`

`Debug` prints the Rust structure, for developers: `PriceNotPositive`. `Display` prints the sentence a user should see, for API responses: `price must be greater than zero`. Keeping the two apart matters once error text is part of the API contract.

The `match` inside `Display` has **no `_` catch-all arm, on purpose**. So adding a variant without a message is a compile error. Without this, a new variant would quietly get some vague default text.

`impl std::error::Error for EngineError {}` is empty, but it is not decoration. It marks the type as a standard Rust error. That is what lets it be used as `Box<dyn Error>` and converted by `?` across error types. It needs `Debug` and `Display` to exist first, which is why it comes last.

## Not done yet

- **Stable error codes.** The API will want a machine-readable code per variant, like an RFC 7807 `type` or a gRPC status. Deriving one from the variant name would break the day a variant is renamed.
- **Client fault versus engine fault.** Every variant here is treated as the caller's mistake, an HTTP 4xx. A real internal failure must not be reported as the caller's fault, and the API layer cannot tell the two apart without help.
- **`From` impls.** Nothing converts another crate's error into `EngineError` yet, because the crate has no dependencies. Adding `serde` at the network edge will change that.
