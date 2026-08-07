# `src/error.rs`

One error enum for the whole crate, plus a `Result` alias.

## Why one enum rather than one per module

The API layer eventually needs a single exhaustive `match` to turn engine failures into
HTTP responses or gRPC statuses. Several error types would mean several conversions and
a real chance of a variant being missed when one is added.

The cost is that this enum knows about concepts from across the crate. That is worth
paying here.

## Why every fallible operation returns `Result`

**The engine must never panic on input a client can influence.** If a malformed price
can take the process down, anyone who can reach the API can halt the market for
everyone. Failures come back as values so the caller can reject the command and carry
on.

## The variants

| variant | raised by | message |
|---|---|---|
| `PriceNotPositive` | `Price::from_minor_units` | price must be greater than zero |
| `QuantityNegative` | `Quantity::from_minor_units`, `checked_sub` | quantity must be greater or equal to zero |
| `QuantityNotPositive` | `Quantity::require_positive` | quantity must be greater than zero |
| `IdempotencyKeyInvalid` | `IdempotencyKey::new` | idempotency key must satisfy its length and charset rules |
| `FillExceedsRemaining` | `RestingOrder::fill` | fill quantity must not exceed remaining quantity |
| `Overflow` | `Quantity::checked_add` | arithmetic overflow |

**Why quantity needs two variants.** `Quantity` allows zero but rejects negatives, so
"you passed -5" and "this place needs a positive number" are different rules. One
variant covering both would produce a message that contradicts whichever rule it was
not written for.

`Price` does not have this problem — it rejects zero *and* negative, so a single
variant is accurate for both.

**Why `Overflow` is worded generically.** It is not addition-specific and not
quantity-specific: `checked_sub` can raise it, and `Price` and `Money` arithmetic will.
A message has to be true for every path that produces it.

## Naming rule

Variants say **what is wrong**, not that something is wrong. `PriceNotPositive`, not
`InvalidPrice`. The test: a good variant name writes the user-facing message for you.
"Price must be greater than zero" falls out of the first; from the second you would
have to read the code to find out what was invalid.

## The `Result` alias

```rust
pub type Result<T> = std::result::Result<T, EngineError>;
```

Lets signatures read `-> Result<Price>` instead of `-> Result<Price, EngineError>`.

The right-hand side must spell out `std::result::Result` — writing `Result<T, EngineError>`
there would refer to the alias being defined and be circular.

**Consequence worth knowing:** inside this crate `Result` now means this alias, which
takes *one* type parameter. Anything returning a different error type must write the
full path. The `Display` impl below is exactly that case — its method returns
`std::fmt::Result`, a third unrelated `Result` that takes none.

## The derives

`Debug` for test output and `unwrap`. `Clone` because errors get copied into logs and
responses. `PartialEq` and `Eq` so tests can write
`assert_eq!(result, Err(EngineError::Overflow))` rather than matching — the test files
rely on this heavily.

## `Display` and `Error`

`Debug` prints the Rust structure, for developers. `Display` prints the sentence a user
should see, for API responses. Keeping them separate matters once error text is part of
an API contract.

The `match` in `Display` is **exhaustive with no `_` arm on purpose**: adding a variant
without a message becomes a compile error rather than silently inheriting a useless
catch-all string.

`impl std::error::Error for EngineError {}` is empty and is not decoration — it marks
the type as a standard error, which is what makes `Box<dyn Error>` and cross-type `?`
conversions work. It requires `Debug` and `Display` to already exist, which is why it
comes last.

## Not here yet

- **Stable error codes.** The API will want a machine-readable identifier per variant
  (RFC 7807 `type`, or a gRPC status). Deriving one from the variant name is fragile.
- **Client-fault vs engine-fault.** Everything here is a 4xx. Genuine internal failures
  must not be reported as the caller's mistake, and the API layer cannot tell them apart
  without help.
- **`From` impls.** Nothing converts a foreign error yet, because the crate has no
  dependencies. `serde` at the transport boundary will change that.
