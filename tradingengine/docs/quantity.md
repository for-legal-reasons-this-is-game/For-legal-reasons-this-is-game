# `src/quantity.rs`

An order or trade size, stored as a `u128` count of minor units at the crate's `SCALE`
decimal places — the same representation as `Price`, for the same reason (see
[price.md](price.md)).

## The difference from `Price`: non-negative, not strictly positive

`Price` rejects zero. `Quantity` allows it, because zero is a normal value in the
lifecycle: "filled so far" starts at zero and "remaining" ends there.

But an *order* for zero units is nonsense. So the invariant splits in two:

- **`from_minor_units`** is infallible — `u128` cannot even represent a negative, and
  zero gets through on purpose.
- **`require_positive`** is asked for explicitly at the places where a size of nothing
  is not a legal input — placing an order, applying a fill.

The crate still has two error variants because two different checks fire them:
`QuantityNotPositive` belongs to `require_positive`, and `QuantityNegative` to
`checked_sub`, whose result would otherwise fall below zero. Each `Display` message
("greater than zero" versus "greater or equal to zero") states exactly what its check
enforces.

## `Quantity::ZERO`

An associated constant, so callers write `Quantity::ZERO` rather than
`Quantity::from_minor_units(0)` — the name states the intent, not just the value.

The same constant on `Price` would be a hole in the invariant, which is why one type
has it and the other never will.

## Checked arithmetic

Plain `+` and `-` on Rust integers behave **differently depending on build mode**:

| build | `u128::MAX + 1` |
|---|---|
| debug (`cargo test`) | panics |
| release | silently wraps to zero |

That asymmetry is the trap. Tests run in debug, so overflow panics loudly and looks
handled. Production runs in release, where it wraps around and keeps going — in an
engine, two enormous sizes adding up to almost nothing.

So both operations are checked:

- **`checked_add`** uses `u128::checked_add`, which does the addition and the detection
  together and returns `Option`. `.ok_or(EngineError::Overflow)` turns `None` into the
  domain error.

  This cannot be hand-rolled after the fact. An earlier version did `let tmp = a + b`
  and then tried to detect the overflow by subtracting back — but the overflow had
  already happened on the first line, and modular wrapping is *consistent*, so
  `(a + b) - a == b` holds even when both operations wrapped. The check could never
  fire.

- **`checked_sub`** guards against the result falling **below zero**. In `u128` that is
  not a negative number but an underflow: plain `-` on `3 - 10` panics in debug and
  wraps to an astronomical value in release. The explicit `other > self` guard turns it
  into a domain error instead — named `QuantityNegative` because "the result would have
  been negative" is the fact the caller cares about.

## The derives

`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord`.

`Ord` is here for a different reason than on `Price`. Nothing sorts quantities — there
is no map keyed by them. It is for plain comparison: `filled > quantity` is the guard
that stops an order over-filling.

**No `Hash`**, unlike `Price`. Nothing keys a map by quantity; "give me the orders whose
size is 5" is not a question the engine asks. Deriving a trait with no caller is a
promise nobody needs.

## Not here yet

- **`Display` and decimal parsing**, for the same boundary reason as `Price`.
- **Lot alignment.** A quantity must be a whole multiple of its coin's lot size — one
  share, or some fraction of a coin. That belongs with the coin specification, which
  does not exist yet.
- **Multiplication.** `price × quantity` produces a notional, which needs a rounding
  policy decided deliberately rather than inherited from whatever integer division does.
