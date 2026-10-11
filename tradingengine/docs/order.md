# `src/order.rs`: what an order is

This file holds four small enums that describe an order: its side, its kind, how long it lives, and its status. They are just data, with two small helper methods. `RestingOrder` is the struct that holds them.

> **Changing under #21.** The trading contract changes three of these enums. `OrderKind::Market` gains a `protection_price`. `TimeInForce` gains `Gtd { expire_time }`. `OrderStatus` drops `Open` and `Rejected` and adds `Expired` and `PendingTrigger`. See the "Today / After #21" table in [DATA_MODEL.md](DATA_MODEL.md). This doc describes the code as it is now.

## The four enums

| enum | variants |
|---|---|
| `Side` | `Buy`, `Sell` |
| `OrderKind` | `Market`, `Limit { price: Price }` |
| `TimeInForce` | `GoodTilCancelled`, `ImmediateOrCancel`, `FillOrKill` |
| `OrderStatus` | `New`, `Open`, `PartiallyFilled`, `Filled`, `Cancelled`, `Rejected` |

## The key idea: the price lives inside the variant

```rust
pub enum OrderKind {
    Market,
    Limit { price: Price },
}
```

In Rust, one variant of an enum can carry its own data. A C or Java enum cannot do this. This is the whole point of the type.

**An example.** Alice wants to buy BTC, but pay no more than $29,000. That is a limit order:

```rust
OrderKind::Limit { price }   // price = 2_900_000 cents per BTC
```

Bob just wants BTC now, at whatever the book offers. That is a market order. It has no price:

```rust
OrderKind::Market
```

### The design not taken

The obvious alternative is a flag plus an optional price:

```rust
struct Order {
    kind: OrderKind,          // Market or Limit
    price: Option<Price>,     // meant to be Some only when kind is Limit
}
```

Both designs can say "limit order at $29,000". Only this one can also say **a market order with a limit price**: `kind: Market, price: Some(2_900_000)`. That makes no sense. But it can be built, so every function that touches an order must decide what to do with it. Sooner or later one forgets, and nothing warns you.

With the price inside the variant, that state cannot exist. The compiler also makes you ask the question every time, because the only way to read the price is a `match`:

```rust
match kind {
    OrderKind::Limit { price } => { /* the price is here, and it exists */ }
    OrderKind::Market => { /* there is no price, and nowhere to look for one */ }
}
```

`Option<Price>` pushes the check to runtime and makes it easy to skip. The enum makes it a compile-time check you cannot skip. It is the same idea as the private field on `Price`, one level up: don't check for a bad state, make it impossible to build.

## Why two enums where the diagram had one

The original diagram in [DATA_MODEL.md](DATA_MODEL.md) had one enum: `Market | Limit | IOC | FOK`. Those four are not four kinds of one thing. They answer two separate questions:

| question | answers |
|---|---|
| What price will I accept? (`OrderKind`) | `Market`, `Limit { price }` |
| How long may the order live? (`TimeInForce`) | `GoodTilCancelled`, `ImmediateOrCancel`, `FillOrKill` |

The time-in-force answers mean:

- **GoodTilCancelled (GTC).** Rest in the book until filled or cancelled.
- **ImmediateOrCancel (IOC).** Fill what you can right now, cancel the rest.
- **FillOrKill (FOK).** Fill the whole order right now, or do nothing at all.

Two answers times three answers gives **six** valid orders. A market FOK and a limit FOK are both real, and they behave differently. A flat enum of four cannot name six things. It was really two lists glued together.

With two enums, the type carries the full combination. No code has to work out "was that a limit order?" from a variant that was really about duration.

## `Side::opposite`

An incoming buy matches against resting **sells**. An incoming sell matches against resting **buys**. The matcher needs this flip every time it looks at the book.

Writing the two-arm `match` at every call site invites getting it backwards once. That bug does not crash. It just trades against the wrong side of the book.

So there is one definition, `opposite()`. The tests check that `Buy.opposite()` is `Sell`, and that calling it twice gives back the original side.

## `OrderStatus::is_terminal`

```rust
pub const fn is_terminal(self) -> bool {
    matches!(self, Self::Filled | Self::Cancelled | Self::Rejected)
}
```

`matches!` is a macro that turns into a `match` returning `true` or `false`. It is the short way to write "is this value one of these variants".

**Terminal** means the order will never change status again and may leave the book. `Filled`, `Cancelled` and `Rejected` are terminal. `New`, `Open` and `PartiallyFilled` are not.

**Why a method, not a check each caller writes?** Because the set is a policy. When cancel-on-disconnect or expiry arrives, the set grows, and it must grow in one place. A dozen hand-written `== Filled || == Cancelled` checks would each need to be found and fixed.

`RestingOrder::cancel` uses this method. It refuses to cancel an order that is already terminal.

### `New` versus `Open`

They are different. `New` means accepted but not yet placed. `Open` means resting in the book. A market order that fills completely at once never becomes `Open` at all.

## Derives

All four enums derive `Debug, Clone, Copy, PartialEq, Eq`, and nothing else, on purpose.

| derive | why |
|---|---|
| `Debug` | `assert_eq!` will not compile without it |
| `Clone` | required before you can have `Copy` |
| `Copy` | three enums have no data. `OrderKind` holds one `Price`, which is itself `Copy`. |
| `PartialEq`, `Eq` | comparing sides and statuses is routine |

- **No `Hash`.** Nothing uses a side or a status as a map key.
- **No `Ord`, and `OrderStatus` is the one that matters.** `Ord` would make `status < OrderStatus::Filled` compile. The comparison would follow the order the variants are written in the source. That looks like the order lifecycle, but it is not one. `Cancelled` and `Filled` are both endings. Neither comes "before" the other. A reader would mistake that line for a real progress check. It is the same argument as `Ord` on `OrderId` in [ids.md](ids.md): an unused derive is not free, because it quietly allows operations that look sensible and are wrong.
- **No `Default`.** There is no such thing as a default side. An order's status is always set on purpose. `RestingOrder::new` sets `Open`, and `fill` sets `PartiallyFilled` or `Filled` from what is left. `Price` never derives `Default` for the same reason.

Both methods are `const fn`. It costs nothing and keeps them usable at compile time if that is ever wanted.

## Tests

`tests/order.rs` checks that:

- `opposite()` flips the side, and applying it twice returns the original
- a `Limit` carries its price and matches back out as `Limit`
- a `Market` is never equal to any `Limit`
- two limits at different prices are not equal
- `Filled`, `Cancelled` and `Rejected` are terminal. `New`, `Open` and `PartiallyFilled` are not.
- the three `TimeInForce` variants are all distinct

## Not done yet

- **`OrderStatus` does not check status changes.** It is just a set of names. It does not know that `Filled` can never go back to `Open`. That check needs the quantities, so it belongs on `RestingOrder`, where the status must agree with the remaining and original quantity. `RestingOrder` already does part of this: `fill` sets the status from the quantity left, and `cancel` refuses a terminal order.
- **`TimeInForce` does nothing yet.** What IOC and FOK actually do, cancel the rest or reject unless fully filled, lives in the matcher. This file only names them.
- **`OrderKind` has no getter.** Callers `match`. A `limit_price() -> Option<Price>` would bring back the `Option` this type exists to avoid, so it would need a good reason.
- **Market orders have no price protection.** A market order into a thin book fills at whatever prices are there, however bad. A limit on how far the price may move is a real need, but it needs the market's settings, which do not exist yet. Under #21 this becomes `protection_price` on `Market`.
- **No `Display` or wire format.** Like `Price` and the quantity types, these cross the API as strings. That conversion belongs at the network edge, not here.
