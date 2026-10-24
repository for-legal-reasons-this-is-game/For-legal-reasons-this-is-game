# Quantities (`src/quantity.rs`)

This file has two amount types, `BaseQuantity` and `QuoteQuantity`. Each is a `u128` count of minor units, the smallest unit of its currency. They use a whole number for the same reason `Price` does (see [price.md](price.md)).

## Why there are two

An exchange counts two different things. It counts how much of the coin changes hands, and how much money pays for it. Both are stored as plain integers, so without separate types Rust can't tell them apart.

**An example.** Take the BTC-USD market:
- **Base** is BTC, counted in satoshis. 1 BTC is 100,000,000 satoshis. This is the market's `base_decimals`, 8.
- **Quote** is USD, counted in cents. $1 is 100 cents. This is the market's `quote_decimals`, 2.

Alice buys half a bitcoin at $29,000:
```
base amount  = 50_000_000   (half a BTC, in satoshis)
quote amount = 1_450_000    ($14,500.00, in cents)
```
With a single Quantity type, this line compiles fine:
```rust
let total = base_amount.checked_add(quote_amount);   // 51_450_000 of... what?
```
The result is nonsense. It is like adding metres to seconds. With two types, the same line fails to compile. The compiler catches the mistake before the code ever runs.

The contract uses both kinds everywhere:
- An order is sized in base. The one exception is a market buy, which is sized in quote.
- Every fill has a base amount and a quote amount.
- Fees are always quote.

**The cost.** The two types have identical code, written out twice. Each one reads on its own with nothing hidden. But if you change one, you must change the other the same way. Everything below applies to both. The examples say `BaseQuantity`.

## Zero is allowed

`Price` rejects zero. A quantity allows it, because zero is a normal value. When Alice places her order, "filled so far" starts at zero. When it is fully filled, "remaining" ends at zero.

But an order for zero BTC is nonsense. So the check is split in two:
- **`from_minor_units`** never fails. `u128` cannot hold a negative, and zero gets through on purpose.
- **`require_positive`** is called on purpose, at the places where "nothing" is not a legal size. Examples are placing an order and applying a fill. It returns `QuantityNotPositive` for zero.

```rust
BaseQuantity::from_minor_units(0)                      // fine, a zero amount
BaseQuantity::from_minor_units(0).require_positive()   // Err(QuantityNotPositive)
```

`minor_units()` gives the number back out. `is_zero()` asks whether it is zero.

## Two error variants

There are two quantity errors, because two different checks raise them:

| variant | raised by | message |
|---|---|---|
| `QuantityNotPositive` | `require_positive` | "quantity must be greater than zero" |
| `QuantityNegative` | `checked_sub`, when the result would go below zero | "quantity must be greater or equal to zero" |

Each message says exactly what its check enforces.

## `ZERO`

Each type has a constant, so you write `BaseQuantity::ZERO` instead of `BaseQuantity::from_minor_units(0)`. The name says what you mean, not just the value.

`Price` has no `ZERO`, and never will. On `Price` it would be a hole in the "always above zero" rule.

## Checked arithmetic

"Overflow" means a result is too big to fit in the type. Plain `+` and `-` on Rust integers handle overflow differently depending on how you build:

| build | `u128::MAX + 1` |
|---|---|
| debug (`cargo test`) | panics (crashes) |
| release | silently wraps around to zero |

That difference is the trap. Tests run in debug, so overflow crashes loudly and looks handled. Production runs in release. There it wraps around and keeps going. In an engine, two huge sizes would add up to almost nothing.

So both operations are checked:

- **`checked_add`** uses `u128::checked_add`. That does the addition and the overflow check in one step, and returns `None` on overflow. A `match` turns `None` into `EngineError::Overflow`.

  You cannot check afterwards by hand. An earlier version did `let tmp = a + b`, then tried to spot the overflow by subtracting back. But the overflow had already happened on the first line. Wrapping is consistent, so `(a + b) - a == b` is still true after both steps wrap. The check could never fire.

- **`checked_sub`** stops the result going below zero. In `u128` there is no negative number. Instead, plain `3 - 10` panics in debug and wraps to a huge value in release. `checked_sub` checks `other > self` first and returns `QuantityNegative` instead. The name says what the caller cares about: the result would have been negative.

```rust
qty(10).checked_sub(qty(3))    // Ok(7)
qty(3).checked_sub(qty(3))     // Ok(0), exactly zero is fine
qty(3).checked_sub(qty(10))    // Err(QuantityNegative)
```

## The derives

`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord`.

`Ord` is here for a different reason than on `Price`. Nothing sorts quantities, and no map is keyed by them. It is for plain comparison. `filled > quantity` is the check that stops an order from being over-filled.

**No `Hash`**, unlike `Price`. No map is keyed by quantity. "Give me the orders whose size is 5" is not a question the engine asks. Deriving a trait nobody uses is a promise nobody needs.

## Not done yet

- **`Display` and decimal parsing.** Same reason as `Price`: they belong at the API edge.
- **No lot or minimum-size check, on purpose.** A "lot" is the smallest allowed size step. Under the contract, lot sizes and minimum sizes are the backend's rule and never go on the wire.
- **Multiplication lives elsewhere.** Price times base is `QuoteScale::quote_for`. Quote times fee rate is `FeeBps::fee_on`. Both round down, as the contract requires. See [quote.md](quote.md) and [fee.md](fee.md).
