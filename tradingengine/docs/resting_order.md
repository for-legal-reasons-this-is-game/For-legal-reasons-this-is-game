# `src/resting_order.rs`

The order representation used inside one side of the book. It is deliberately narrower
than an incoming order: only a limit, good-til-cancelled order can rest, so it stores a
`Price` directly and has no `OrderKind`, `TimeInForce`, or `CoinId`.

## Construction

`RestingOrder::new` accepts the order's stable identity and book fields, but it does not
accept `qty_remaining` or `status`. A newly resting order has exactly one valid state:

- `qty_remaining == qty_original`
- `status == OrderStatus::Open`
- `qty_original > 0`

Deriving those fields inside the constructor prevents a caller from creating an open
order with only half its quantity remaining, or a filled order with quantity left.
`Quantity` permits zero because zero is meaningful later in an order's lifecycle;
`RestingOrder` tightens that rule by calling `require_positive` at its boundary.

There is no `CoinId`. A book belongs to one coin, so repeating that identity on every
order would create two sources of truth that could disagree.

## Filling

`fill(Quantity) -> Result<()>` is the only operation that changes fill state. It first
validates the complete operation, computes the next quantity and status in local
variables, and only then assigns both fields. Therefore an error cannot leave a partial
mutation behind.

The transitions are:

| remaining after fill | status |
|---|---|
| greater than zero | `PartiallyFilled` |
| zero | `Filled` |

A zero fill returns `QuantityNotPositive`. A fill larger than the remaining quantity
returns `FillExceedsRemaining`. Both leave the order unchanged.

There are no independent setters for `qty_remaining` and `status`; exposing either one
would allow the cross-field invariant to be broken between calls.

## Cancelling

`cancel() -> Result<()>` sets `status` to `Cancelled` and changes nothing else. A
partially filled order keeps whatever quantity it had left, which is correct: the
remaining quantity records what never traded, and cancelling does not retroactively fill
it.

It refuses to act on an order that is already terminal, returning `OrderNotLive`. Without
that guard, cancelling a filled order would overwrite `Filled` with `Cancelled` and
destroy the record of an execution that really happened.

This lives here rather than on the book for the same reason `fill` does. `status` is this
type's field, so every transition into it belongs to this type. `Cancelled` and `Rejected`
were declared in `order.rs` from the start but had no code path that could produce them —
this closes half of that gap, and `Rejected` stays unreachable until commands exist to
reject.

**`Book::remove` does not call this.** It removes the order and hands it back untouched,
because removal from a container and cancellation are different facts — a fully matched
order leaves by the same route and is `Filled`. The command layer calls this method on
the returned order when the reason really was a cancellation. See [book.md](book.md).

## Ownership and derives

The struct derives `Debug`, `Clone`, `PartialEq`, and `Eq`. It cannot be `Copy` because
`IdempotencyKey` owns a `String`. It has no `Hash` or ordering derive because the book
indexes orders by their `OrderId` and price rather than by the complete struct.

Copyable values are returned from accessors by value. The idempotency key is returned by
reference so reading it does not allocate or clone its string.

## Not here yet

- Removal from the book belongs to the book operation that updates its price level and
  cancel index together. This type only records that the order was cancelled.
- `Rejected` has no code path. It needs a command layer that can refuse an order before
  it ever rests.
- Snapshot restoration may eventually need a separate validated constructor for a
  partially filled order. The public constructor should not be weakened for that case.
- A filled order is transiently represented after `fill`; the book must remove it before
  processing the next command.
