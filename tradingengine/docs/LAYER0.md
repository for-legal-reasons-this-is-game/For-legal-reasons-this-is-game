# Layer 0: the values

A description of the code that implements Layer 0 of [DATA_MODEL.md](DATA_MODEL.md),
the design for issue #21. Layer 0 is the bottom of the engine: the single numbers
every other layer is built out of, and the two pieces of arithmetic the contract
defines on them.

Each type has its own per-file document. This one says how they fit together, which
contract rule each one carries, and what is deliberately missing.

## What is here

| Module | Types | Document |
|---|---|---|
| `src/price.rs` | `Price` | [price.md](price.md) |
| `src/quantity.rs` | `BaseQuantity`, `QuoteQuantity` | [quantity.md](quantity.md) |
| `src/fee.rs` | `FeeBps`, `FeeBps::fee_on` | [fee.md](fee.md) |
| `src/quote.rs` | `QuoteScale`, `QuoteScale::quote_for` | [quote.md](quote.md) |
| `src/timestamp.rs` | `Timestamp` | [timestamp.md](timestamp.md) |
| `src/wide.rs` | `mul_div_floor`, private to the crate | inside [quote.md](quote.md) |

`src/error.rs` gains three variants for this layer: `Overflow`, `DecimalsOutOfRange`
and `TimestampOutOfRange`. See [error.md](error.md).

## Why every number gets its own type

A `Price`, a count of satoshis and a count of cents are all a `u128` underneath. On
the wire they are all the same message, `common.proto:Uint128`. Nothing in the bytes
says which is which.

So the engine wraps each one in its own Rust type with a private field. A newtype is
a struct holding exactly one value, and because the field is private the only way in
is the constructor. The compiler then refuses to pass a quantity where a price belongs.
That matters most for the two quantities: `BaseQuantity` counts the coin being traded
and `QuoteQuantity` counts the money, and mixing them silently would misprice a trade
rather than crash.

This replaces the older `Money`, `SCALE` and `ONE` from #20. There is no crate-wide
scale any more. Scale belongs to each market, which is why `QuoteScale` exists.

## Where the contract rules live

Every rule below is from `proto/how_to_use.md` or a `.proto` comment. The table says
which line of code carries it.

| Rule | Where |
|---|---|
| A price must be above zero, else `BAD_PRICE` | `Price::from_minor_units` returns `PriceNotPositive` |
| A requested quantity must be above zero, else `BAD_QUANTITY` | `BaseQuantity::require_positive`, `QuoteQuantity::require_positive` |
| `quote = price * base_qty / 10^(price_decimals + base_decimals - quote_decimals)` | `QuoteScale::quote_for` |
| `fee = quote * rate_bps / 10_000` | `FeeBps::fee_on` |
| Multiply in a 256-bit intermediate, then divide | `wide::mul_div_floor`, used by both |
| Round down. Never up, never to nearest | the single `/` inside `mul_div_floor` |
| Fees are charged in quote, on both sides | `fee_on` takes and returns `QuoteQuantity` |
| The engine never picks a fee rate | `FeeBps::from_bps` cannot fail and does not judge |
| Integers only, no floats anywhere | no `f32`/`f64` in the crate |
| Decimals out of range is `BAD_DEFINITION` | `QuoteScale::new` returns `DecimalsOutOfRange` |
| A timestamp is `int64 seconds` plus `int32 nanos` | `Timestamp::from_unix_parts` |

## Which constructors can fail, and why

The split is not arbitrary. A constructor rejects a value only when *every* caller
would have to reject it anyway.

- **`Price::from_minor_units` fails on zero.** A price of zero is never meaningful,
  at any layer, so no caller ever wants one.
- **The quantity constructors do not fail.** Zero is a real, legal quantity: it is
  what is left after an order fills completely, and it is what a removed book level
  holds. Only a *request* has to be positive, so that check is a separate step,
  `require_positive`, which the command layer calls once on the way in.
- **`FeeBps::from_bps` does not fail.** The backend owns the rate. Zero is a legal
  rate meaning "free", and the engine has no basis to second-guess any number.
- **`QuoteScale::new` fails on bad decimals**, because an unusable scale must not be
  constructible at all. Once it exists, `quote_for` can only fail on a real overflow.
- **`Timestamp::from_unix_nanos` does not fail; `from_unix_parts` does.** Every `u64`
  of nanoseconds is a valid time. The two wire fields are signed and can describe
  times the engine cannot hold, so that conversion is checked.

The arithmetic methods — `checked_add`, `checked_sub`, `quote_for`, `fee_on` — all
return `Result` and never panic. The engine reports, it does not crash.

## The 256-bit arithmetic

Both formulas have the same shape, `a * b / d`, where `a * b` can need more than 128
bits even when the answer comfortably fits. `wide::mul_div_floor` is the one place
that happens: a schoolbook multiply into two `u128` halves, then bitwise long division.
It is hand-written because the crate has no dependencies and this is the only wide
operation the contract asks for. It is private, and reached only through `quote_for`
and `fee_on`.

It returns `None` — which both callers turn into `EngineError::Overflow` — when the
divisor is zero, or when the quotient would not fit in 128 bits. The second check is
`high >= d`, which is exactly the condition for a result past `u128::MAX`.

**How it was checked.** Beyond the unit tests, the implementation was compared against
Python's arbitrary-precision integers over 424,330 generated triples: uniformly random
`u128` values, the realistic `price * qty / 10^k` and `quote * bps / 10_000` shapes, a
cross product of boundary values, and values placed deliberately on the fits/overflows
line. 148,578 of those were overflow cases. It agreed on every one, both for the floor
result and for the decision to refuse. Worth redoing if the function is ever touched,
since nothing about it is obvious by reading.

## Open questions carried forward

These are recorded in the data model and are not settled by this code.

- **A fee rate above 100%.** `FeeBps` accepts any `u32`, so 20,000 bps (200%) is
  constructible. Whether to reject it as `BAD_ORDER_PARAMS` is open. If it is decided,
  the check belongs in place validation, not in the constructor — the engine applies
  rates, it does not own them.
- **`FeeBps` cannot tell "free" from "not sent".** `maker_fee_bps` and `taker_fee_bps`
  are plain `uint32`, and proto3 reads an unset scalar as `0`. Nothing at this layer
  can recover the difference.
- **A quote can round down to zero.** With a small enough price and quantity,
  `quote_for` returns `Ok(0)`. The contract leaves this to the backend's minimum order
  size. Matching will log it rather than reject it.

## What Layer 0 does not do

- **It does not touch the wire.** Nothing here depends on the `proto` crate. Every
  constructor takes plain Rust numbers, and `from_unix_parts` takes the two
  `google.protobuf.Timestamp` fields as an `i64` and an `i32` rather than the message.
  So the conversion from `Uint128`, `Id` and `Timestamp` into these types does not
  exist yet. It is the first thing the gRPC layer needs, and it is the natural place
  for the contract's "absent is not zero" rule, which no type here can enforce on its
  own.
- **It does not read a clock.** A `Timestamp` always arrives from outside — from a
  request, or as the `now` argument the data model passes into matching. That is what
  keeps a replay of the same commands deterministic.
- **It holds no state and no identity.** Ids, `MarketSymbol` and the enums are
  Layer 1 and Layer 2, and `src/ids.rs` and `src/order.rs` still carry their pre-#21
  shapes. See "What changes in the #20 code" in [DATA_MODEL.md](DATA_MODEL.md).

## One trap worth knowing

`QuoteScale::new` takes its decimals as `(price, base, quote)`. `common.proto:Market`
declares them as `base`, `quote`, `price`. The two orders disagree, so a call site
that follows the message field order computes the wrong exponent.

It is easy to miss, because the formula adds price and base: swapping *those* two
changes nothing, and the BTC-USD example used throughout these documents has
`price_decimals == quote_decimals == 2`, so misplacing the quote decimals also
produces the right answer. `three_different_decimals_pin_the_argument_order` in
`tests/quote.rs` uses three distinct values so that a transposition fails.

**proposed:** when Layer 3 introduces `Market`, give it a `QuoteScale` accessor that
reads the fields by name, and let that be the only call site.
