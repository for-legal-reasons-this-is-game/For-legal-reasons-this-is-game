# `src/order.rs`

Four enums describing what an order *is*. No struct here holds them yet — that is
`RestingOrder`, the next step. All data, with two small methods.

## The central decision: the price lives inside the variant

```rust
pub enum OrderKind {
    Market,
    Limit { price: Price },
}
```

A Rust enum variant can carry its own fields, which a C or Java enum cannot. That is the
whole point of this type, so it is worth being explicit about what the alternative would
have cost:

```rust
// the design not taken
struct Order {
    kind: OrderKind,          // Market | Limit
    price: Option<Price>,     // Some(..) only when kind is Limit
}
```

Both designs can express a limit order at 29,000. Only the second can express a **market
order carrying a limit price** — `kind: Market, price: Some(29_000)`. That state is
meaningless, but it is constructable, so every function touching an order has to decide
what to do about it. In practice one of them forgets, and the forgetting is silent.

With the price inside the variant there is no such state to forget about. The compiler
also forces the question at every use, because reading the price requires a match:

```rust
match kind {
    OrderKind::Limit { price } => /* the price is here, and it exists */,
    OrderKind::Market => /* there is no price, and there is nowhere to look for one */,
}
```

`Option<Price>` moves that decision to runtime and makes it optional. The enum makes it
compile-time and mandatory. This is the same move as the private field on `Price`, one
level up: instead of validating a bad state, arrange for it not to exist.

## Why two enums where the diagram had one

`DATA_MODEL.md` records the diagram's single enum as `Market | Limit | IOC | FOK`. Those
four are not four kinds of one thing. They are two independent questions:

| question | answers |
|---|---|
| what price will I accept? | `Market`, `Limit { price }` |
| how long may the order live? | `GoodTilCancelled`, `ImmediateOrCancel`, `FillOrKill` |

Two axes with two and three answers give **six** legal orders — a market FOK and a limit
FOK are both real and behave differently. A single flat enum of four cannot name six
things. It was not a list of options; it was two lists accidentally concatenated.

Splitting them means the type system carries the full combination, and no code has to
reconstruct "was that a limit order?" from a variant that was really about duration.

## `Side::opposite`

An incoming buy matches against resting **sells**. That flip is needed everywhere the
matcher looks at the book, and writing the two-arm match at each site invites getting it
backwards exactly once — a bug that does not crash, it just trades against the wrong
side of the book.

One definition, `opposite()`, and the tests pin down that applying it twice returns the
original.

## `OrderStatus::is_terminal`

```rust
pub const fn is_terminal(self) -> bool {
    matches!(self, Self::Filled | Self::Cancelled | Self::Rejected)
}
```

`matches!` is a macro that expands to a `match` returning a bool — it is the concise form
of "is the value one of these variants".

Terminal means the order will never transition again and may leave the book. The reason
this is a method rather than a check callers write for themselves is that **the set is a
policy**. When cancel-on-disconnect or expiry arrives, the set grows, and it has to grow
in one place. A dozen open-coded `== Filled || == Cancelled` comparisons would each need
finding.

Note that `New` and `Open` are distinct: `New` is accepted but not yet placed, `Open` is
resting in the book. A market order that fully executes never becomes `Open` at all.

## The derives

`Debug, Clone, Copy, PartialEq, Eq` on all four, and deliberately nothing else.

| derive | why |
|---|---|
| `Debug` | `assert_eq!` will not compile without it |
| `Clone` | prerequisite for `Copy` |
| `Copy` | three are fieldless; `OrderKind` holds one `Price`, which is itself `Copy` |
| `PartialEq`, `Eq` | comparing sides and statuses is routine |

**No `Hash`** — nothing keys a map by a side or a status.

**No `Ord`, and `OrderStatus` is the one that matters.** Deriving it would make
`status < OrderStatus::Filled` compile, and the comparison would use *declaration order*
— an ordering that looks like the order lifecycle and is not one. `Cancelled` and
`Filled` are both endings; neither comes "before" the other. A reader would take such an
expression for a legitimate progress check. This is the same argument as `Ord` on
`OrderId` in [ids.md](ids.md): a derive with no caller is not free, because it silently
authorises operations that read as sensible and are wrong.

**No `Default`.** There is no such thing as a default side, and an order's status is
something the constructor decides deliberately from its fill state — the same reason
`Price` must never derive it.

Both methods are `const fn`, which costs nothing and keeps them usable in constant
contexts if that is ever wanted.

## Not here yet

- **Nothing validates transitions.** `OrderStatus` is a set of names; it does not know
  that `Filled` cannot go back to `Open`. That check needs the quantities to compare
  against, so it belongs on `RestingOrder`, where status has to agree with
  `qty_remaining` and `qty_original`.
- **`TimeInForce` has no behaviour.** What IOC and FOK actually *do* — cancel the
  remainder, or reject the whole order unless it fills at once — lives in the matcher.
  This file only names them.
- **`OrderKind` has no accessor.** Callers match. If a `limit_price() -> Option<Price>`
  ever appears it will reintroduce the `Option` this type exists to avoid, so it should
  have to justify itself.
- **Market orders have no price protection.** A market order into a thin book executes
  at whatever is there. A slippage band or a reference-price collar is a real
  requirement, but it needs the coin specification, which does not exist yet.
- **No `Display` or wire encoding**, for the same boundary reason as `Price` and
  `Quantity`: these cross the API as strings, and that belongs at the transport edge.
