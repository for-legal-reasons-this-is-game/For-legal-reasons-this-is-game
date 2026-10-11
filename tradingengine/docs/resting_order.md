# RestingOrder

A `RestingOrder` is an order that is sitting in the book, waiting for someone to trade with it. It lives in `src/resting_order.rs`. It is narrower than an incoming order on purpose: only a limit order that is good until cancelled can rest.

## What it is

A limit order says "buy or sell, but only at this price or better". "Good until cancelled" means it stays in the book until it trades or someone cancels it.

Because only that one kind of order can rest, the type is simple:
- It stores a `Price` directly. There is no "maybe a price" case.
- It has no `OrderKind` and no `TimeInForce`. Every resting order is the same kind.
- It has no `CoinId`. See "Why no coin" below.

The fields are: `id`, `user_id`, `side`, `price`, `qty_original`, `qty_remaining`, `status`, `seq_no` and `idempotency_key`. The two quantities are `BaseQuantity`, the amount of the coin.

## An example

Take the BTC-USD market. Base is BTC, counted in satoshis (1 BTC is 100,000,000 satoshis). Price is in cents per BTC.

Alice wants to sell 1 BTC at $29,000. Her order rests like this:
```
price         = 2_900_000      ($29,000.00, in cents)
qty_original  = 100_000_000    (1 BTC, in satoshis)
qty_remaining = 100_000_000
status        = Open
```
Bob buys 0.4 BTC from her. Now:
```
qty_original  = 100_000_000    (never changes)
qty_remaining =  60_000_000    (0.6 BTC left)
status        = PartiallyFilled
```

## Making one

`RestingOrder::new` takes the order's id, user, side, price, original quantity, sequence number and idempotency key. It returns `Result<RestingOrder>`.

It does **not** take `qty_remaining` or `status`. A brand new resting order has exactly one valid state:
- `qty_remaining` equals `qty_original`.
- `status` is `OrderStatus::Open`.
- `qty_original` is greater than zero.

The constructor sets those fields itself. So a caller cannot build an `Open` order with only half its quantity left, or a `Filled` order that still has quantity.

**Why the zero check lives here.** `BaseQuantity` allows zero, because zero means something later in an order's life (a fully filled order has zero left). But an order that starts at zero makes no sense. So `new` calls `require_positive` on `qty_original`. A zero quantity returns `QuantityNotPositive`.

## Why no coin

A book belongs to one coin. If every order also stored its coin, there would be two places saying which coin it is. Two copies of the same fact can disagree. Leaving it out means there is only one.

## Filling

`fill(BaseQuantity) -> Result<()>` is the only method that changes how much is filled.

It works in two steps:
1. **Check and compute.** It checks the fill, then works out the new remaining quantity and the new status in local variables.
2. **Write.** Only then does it store both fields.

So if anything fails, nothing has been changed. An error can never leave the order half updated.

The status after a fill depends on what is left:

| remaining after fill | status |
|---|---|
| more than zero | `PartiallyFilled` |
| zero | `Filled` |

Two fills are refused. Both leave the order unchanged:
- **A zero fill** returns `QuantityNotPositive`.
- **A fill bigger than what is left** returns `FillExceedsRemaining`. If Alice has 0.6 BTC left, a fill of 0.7 BTC is refused.

**No setters.** There is no way to set `qty_remaining` or `status` on their own. If there were, a caller could change one and forget the other:
```rust
order.set_qty_remaining(zero);   // does not exist
// status still says Open, but nothing is left
```
The two fields must always agree. Only `fill` changes them, and it changes both at once.

## Cancelling

`cancel() -> Result<()>` sets `status` to `Cancelled`. It changes nothing else.

A partly filled order keeps the quantity it had left. That is correct. The remaining quantity records what never traded. Cancelling does not pretend it was filled. If Alice cancels with 0.6 BTC left, `qty_remaining` stays 60_000_000.

**It refuses finished orders.** If the order is already terminal (`Filled`, `Cancelled` or `Rejected`), `cancel` returns `OrderNotLive`. Without this check, cancelling a filled order would overwrite `Filled` with `Cancelled`. That would erase the record of a trade that really happened. Cancelling twice is refused the same way.

**Why it lives here and not on the book.** `status` is this type's field. So every change to it belongs to this type, just like `fill`.

`Cancelled` and `Rejected` were declared in `order.rs` from the start, but nothing could produce them. `cancel` fixes half of that. `Rejected` is still unreachable until there are commands that can reject.

**`Book::remove` does not call `cancel`.** It takes the order out and hands it back untouched. Removing an order from the book and cancelling it are different facts. A fully matched order also leaves the book, and it is `Filled`, not `Cancelled`. The command layer calls `cancel` on the returned order when the reason really was a cancel. See [book.md](book.md).

## Derives and accessors

The struct derives `Debug`, `Clone`, `PartialEq` and `Eq`.
- **No `Copy`.** It does not derive `Copy`, so duplicating an order takes an explicit `.clone()`. Every field is now `Copy`, including `IdempotencyKey`, which stores its text in a fixed 64-byte buffer. So adding `Copy` would compile. It has simply not been added.
- **No `Hash` and no ordering.** The book finds orders by their `OrderId` and price, never by the whole struct, so neither is needed.

Every field has a read-only accessor with the same name. Small values (`id`, `user_id`, `side`, `price`, the quantities, `status`, `seq_no`) are returned by value. `idempotency_key()` returns a reference, so reading it does not copy the 64-byte buffer.

## Changing under #21

The contract in issue #21 reshapes this type. See the `RestingOrder` section of [DATA_MODEL.md](DATA_MODEL.md). In short:
- `user_id`, `idempotency_key` and `seq_no` are dropped.
- New fields arrive: `account_holder`, `base_account` and `quote_account`, `command_id`, `order_type`, `lifetime`, filled base and quote totals, and pinned `maker_fee_bps` and `taker_fee_bps`.
- A GTD order (good until a set date) can rest too, not only GTC.
- `Open` and `Rejected` leave `OrderStatus`. A resting order is `New` or `PartiallyFilled`.

## Not done yet

- **Removal from the book** is the book's job, because it must update the price level and the order index together. This type only records that the order was cancelled.
- **`Rejected` has no code path.** It needs a command layer that can refuse an order before it ever rests.
- **Restoring from a snapshot** may later need a separate, checked constructor for a partly filled order. The public `new` should not be loosened for that.
- **A filled order lingers briefly.** After `fill` brings it to zero, it is still in its level for a moment. The book must remove it before handling the next command.
