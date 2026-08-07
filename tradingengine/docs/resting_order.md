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

## Ownership and derives

The struct derives `Debug`, `Clone`, `PartialEq`, and `Eq`. It cannot be `Copy` because
`IdempotencyKey` owns a `String`. It has no `Hash` or ordering derive because the book
indexes orders by their `OrderId` and price rather than by the complete struct.

Copyable values are returned from accessors by value. The idempotency key is returned by
reference so reading it does not allocate or clone its string.

## Not here yet

- Cancellation and removal from the book belong to the book operation that updates its
  price level and cancel index together.
- Snapshot restoration may eventually need a separate validated constructor for a
  partially filled order. The public constructor should not be weakened for that case.
- A filled order is transiently represented after `fill`; the book must remove it before
  processing the next command.
