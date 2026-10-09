# The wire boundary (`src/wire.rs`)

This file is the only place in the engine that touches a protobuf message. It
converts between the contract's wire types and the Layer 0 values of
[DATA_MODEL.md](DATA_MODEL.md), and it is where the contract's rules about
*absent* fields are enforced.

Nothing else in the crate imports `proto`. Everything above this file works in
`Price`, `BaseQuantity`, `QuoteQuantity`, `FeeBps`, `Timestamp` and
`QuoteScale`, which already carry their own guarantees.

## Why this needs its own file

The wire has three shapes the engine cannot use directly.

- **`Uint128` and `Id`** are messages of two halves, `fixed64 high` and
  `fixed64 low`, because protobuf has no 128-bit integer. The value is
  `(high << 64) | low`. The `proto` crate already does that arithmetic; what it
  cannot do is say whether a given 128-bit number is a price, a count of
  satoshis or a count of cents. Deciding that, and applying the matching rule,
  is this file's job.
- **`google.protobuf.Timestamp`** is a signed `int64 seconds` plus a signed
  `int32 nanos`. The engine holds a `u64` of nanoseconds. Some wire values have
  no engine equivalent, so the conversion is checked.
- **Any message field can be absent.** That is the rule below.

## Absent is not zero

The contract states it twice, in `how_to_use.md` and in the wire conventions of
the data model: *absent `Uint128`/`Id` is not zero. Check absence, not zero.*

proto3 cannot distinguish a message field that was never set from one set to its
zero value. Both arrive as `None`. A scalar field — `uint32`, `string` — is the
opposite: it is never absent, and unset reads as `0` or `""`.

So a missing amount is a malformed request, not an order for nothing:

```
PlaceOrderRequest with no base_quantity  ->  RequiredFieldAbsent
PlaceOrderRequest with base_quantity 0   ->  converts, then fails
                                             require_positive
```

Those two must not collapse into one answer. Reading an absent field as zero
turns a malformed request into a plausible one, and the engine would then reject
it for the wrong reason, or in the case of a market-order `limit_price`, accept
something it should have refused.

`required` is the one function that applies this. Everything that needs it goes
through it, so the rule exists in exactly one place.

## Two forms of every inbound conversion

Whether a field is required is a property of the *request*, not of the type. The
same `Uint128` is mandatory in one message and optional in another.

```rust
// limit_price on a limit order: it must be there
let price = Price::try_from(req.limit_price)?;          // Option<Uint128>

// limit_price on a market order: absent on purpose
let protection = match req.protection_price {
    Some(p) => Some(Price::try_from(p)?),               // Uint128
    None => None,
};
```

So each type converts from both the message and `Option<message>`. The caller
picks the form that matches the field it is reading, and the compiler stops it
from forgetting the `Option` entirely.

The clearest case is the one the data model calls out: a `PlaceOrderRequest`
with no `limit_price` is a market order, or a mistake. A `limit_price` of `0` is
a bad price. Only the caller knows which message it is holding, so only the
caller can choose.

## What each conversion does

| Wire | Engine | Fails when |
|---|---|---|
| `Uint128` | `Price` | the value is zero (`PriceNotPositive`) |
| `Option<Uint128>` | `Price` | absent, or zero |
| `Uint128` | `BaseQuantity`, `QuoteQuantity` | never |
| `Option<Uint128>` | `BaseQuantity`, `QuoteQuantity` | absent |
| `Option<Id>` | `u128`, via `required_id` | absent |
| `uint32` | `FeeBps` | never |
| `Timestamp` | `Timestamp` | before 1970, nanos not below one second, or past the engine's range |
| `Option<Timestamp>` | `Timestamp` | absent, or any of the above |
| `&Market` | `QuoteScale` | the decimals give an exponent below 0 or above 38 (`DecimalsOutOfRange`) |

Outbound, every conversion is infallible: `Price`, `BaseQuantity` and
`QuoteQuantity` become `Uint128`, `FeeBps` becomes `u32`, and `Timestamp`
becomes a wire `Timestamp`. An engine value is always representable on the wire,
because the wire range is the wider of the two in every case.

- **Quantities do not reject zero**, here or anywhere. Zero is a real quantity
  on the wire: it is what a `BookDelta` carries to say a price level is gone. A
  *request* quantity must be positive, which is `require_positive`, one step
  further in. Keeping the two apart is why the quantity constructors are
  infallible — see [quantity.md](quantity.md).
- **`FeeBps` cannot fail and cannot tell "free" from "not sent".**
  `maker_fee_bps` and `taker_fee_bps` are plain `uint32`, so they are never
  absent, and proto3 reads an unset scalar as `0`. Zero is also a legal rate.
  Nothing at this boundary can recover the difference, and the engine has no
  business guessing — the backend owns the rate. See [fee.md](fee.md).
- **`Timestamp` splits back into seconds and nanos** without a check. The
  engine's range ends in the year 2554, so the whole seconds always fit an
  `i64` and the remainder is below one second and so below `i32::MAX`. This is
  what `EventItem.engine_time` will need at Layer 8.

## `Market` to `QuoteScale`, by name

`QuoteScale::new` takes its decimals positionally as `(price, base, quote)`.
`common.proto:Market` declares them in a different order: `base`, `quote`,
`price`. A call site that follows the message field order computes the wrong
exponent.

It is a quiet failure. The formula adds price and base, so swapping *those* two
changes nothing at all, and the BTC-USD market used in every example here has
`price_decimals == quote_decimals == 2`, which means misplacing the quote
decimals also lands on the right answer. A test suite built on that example
cannot see the mistake.

`TryFrom<&Market> for QuoteScale` reads the three fields by name, so this is the
one call site that has to be right. `market_decimals_are_read_by_name_not_by_position`
in `tests/wire.rs` uses base 6, quote 2, price 3 — three distinct values, so
the quote decimals cannot land in another slot and still give 7. Swapping price
and base does still give 7, but that is the swap the paragraph above says is
harmless.

When Layer 3 introduces the engine's own `Market` type, it should build its
`QuoteScale` through this conversion rather than calling `new` again.

## What is not here

- **The enums.** `Side`, `OrderType`, `TimeInForce` and the rest arrive as
  `i32`, and every conversion needs a fallback arm for an unknown number, with
  `UNSPECIFIED` always a rejection. That is Layer 2.
- **The typed identifiers.** `required_id` returns a bare `u128`, because
  `src/ids.rs` still holds the pre-#21 shapes: `OrderId(u64)`, `CoinId(u32)`,
  `IdempotencyKey`. Layer 1 replaces them with 128-bit `OrderId`, `CommandId`,
  `AccountHolder`, `AccountId` and `TradeId`, and those are what should wrap
  `required_id`.
- **The requests and responses themselves.** Nothing here reads a
  `PlaceOrderRequest`. Turning a whole message into a command, in the
  validation order the contract fixes, is Layer 4.
- **`oneof` arms.** An unknown arm arrives as `None` and the contract says to
  log it loudly and skip the message. `PlaceOrderRequest.amount` is the first
  one that matters, and it belongs with the command parsing at Layer 4.
