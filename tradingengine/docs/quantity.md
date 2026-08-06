# `src/quantity.rs`

An order or trade size, stored as an `i64` count of minor units at the crate's `SCALE`
decimal places — the same representation as `Price`, for the same reason (see
[price.md](price.md)).

## The difference from `Price`: non-negative, not strictly positive

`Price` rejects zero. `Quantity` allows it, because zero is a normal value in the
lifecycle: "filled so far" starts at zero and "remaining" ends there.

But an *order* for zero units is nonsense. So the invariant splits in two:

- **`from_minor_units`** rejects negatives only. Zero gets through.
- **`require_positive`** is asked for explicitly at the places where a size of nothing
  is not a legal input — placing an order, applying a fill.

That split is why the crate has two error variants rather than one. A single
`QuantityNotPositive` would render as "must be greater than zero", which is a message
the constructor would be *lying* with, since it permits zero.

## `Quantity::ZERO`

An associated constant, so callers write `Quantity::ZERO` rather than
`Quantity::from_minor_units(0).unwrap()`.

It is the one legitimate way to build a `Quantity` without going through the
constructor, and that is fine precisely because zero is valid here. The same constant on
`Price` would be a hole in the invariant.

## Checked arithmetic

Plain `+` and `-` on Rust integers behave **differently depending on build mode**:

| build | `i64::MAX + 1` |
|---|---|
| debug (`cargo test`) | panics |
| release | silently wraps to `i64::MIN` |

That asymmetry is the trap. Tests run in debug, so overflow panics loudly and looks
handled. Production runs in release, where it wraps to a huge negative number and keeps
going — in an engine, a filled order suddenly having quintillions of units left.

So both operations are checked:

- **`checked_add`** uses `i64::checked_add`, which does the addition and the detection
  together and returns `Option`. `.ok_or(EngineError::Overflow)` turns `None` into the
  domain error.

  This cannot be hand-rolled after the fact. An earlier version did `let tmp = a + b`
  and then tried to detect the overflow by subtracting back — but the overflow had
  already happened on the first line, and two's complement wrapping is *consistent*, so
  `(a + b) - a == b` holds even when both operations wrapped. The check could never
  fire.

- **`checked_sub`** guards against the result going **negative**, which is a different
  failure from overflow. `i64::checked_sub` would happily return `-7` for `3 - 10`;
  that fits in an `i64` perfectly well. The subtraction itself cannot overflow here,
  since both operands are non-negative.

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
