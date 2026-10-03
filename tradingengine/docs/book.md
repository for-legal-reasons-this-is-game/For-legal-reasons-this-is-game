# Book

The order book for one coin. It lives in `src/book.rs`. It holds the buy orders and the sell orders, sorted so the best one is always first. When a new order arrives, the book trades it against the other side, then keeps whatever is left.

## What it is

A book has three parts:
```rust
bids:   BTreeMap<Price, VecDeque<RestingOrder>>   // people who want to buy
asks:   BTreeMap<Price, VecDeque<RestingOrder>>   // people who want to sell
orders: HashMap<OrderId, (Side, Price)>           // finds any order from its id
```
Each price with orders on it is called a **level**. A level is a queue of the orders at that price.

There is no `CoinId` field. One book *is* one coin. Storing the coin would repeat the same fact on every order inside it.

The public methods are:
- `new`, `insert`, `remove`, `amend`
- `get_by_id`, `best_bid`, `best_ask`
- `bids`, `asks`, `len`, `is_empty`

## An example

Take the BTC-USD market. Base is BTC in satoshis (1 BTC = 100,000,000). Price is in cents per BTC.

The book holds these sell orders (asks):
```
$29,000   Carol 0.5 BTC, then Dave 0.3 BTC
$29,010   Erin  1.0 BTC
```
Alice sends a buy for 0.6 BTC at up to $29,005.
1. The cheapest ask is $29,000. It is at or below her limit, so she trades.
2. Carol was first at that price. Alice buys Carol's 0.5 BTC.
3. Alice still wants 0.1 BTC. Dave is next at $29,000. She buys 0.1 BTC from him. Dave keeps 0.2 BTC and stays first in line.
4. Alice is done. She never reaches Erin at $29,010.

Both trades happen at $29,000, not $29,005. The rest of this doc explains why each step works this way.

## Why BTreeMap: price priority for free

A `BTreeMap` keeps its keys sorted. A `HashMap` does not. `Price` derives `Ord` (see [price.md](price.md)), so the map sorts prices by itself. No code has to sort anything.
- **Best ask** is the first key. It is the cheapest price anyone will sell at.
- **Best bid** is the last key. It is the most anyone will pay.

`best_ask()` reads the first key with `next()`. `best_bid()` reads the last key with `next_back()`. This is the only way the two sides differ. It is also why `bids()` walks the map in reverse and `asks()` does not. For a buyer, higher is better. For a seller, lower is better.

## Why VecDeque: time priority for free

At one price, the oldest order trades first. This is called **FIFO**: first in, first out. It is a queue. Matching needs two things from it: add at the back, and take from the front.

A `VecDeque` does both in constant time, no matter how long the queue is. A `Vec` would not. `Vec::remove(0)` shifts every other element down one slot. With 1,000 orders at a price, taking the first one would move 999 others. That would make the busiest operation in the engine slow.

**Cancelling from the middle** of a level is slow in either one, because the order has to be found first. That is acceptable. Cancels are common, but levels are short. The faster option is a linked list with a handle to every order. It needs a lot of `unsafe` bookkeeping, which is not worth it.

## Matching on arrival

`insert` is not a plain "add to container". The new order trades against the other side while the prices cross. Only what is left rests.

```rust
pub fn insert(&mut self, order: RestingOrder) -> Result<Execution>

pub struct Execution { fills: Vec<Fill>, resting: Option<OrderId> }
pub struct Fill { pub maker: OrderId, pub taker: OrderId, pub price: Price, pub quantity: BaseQuantity }
```

The **maker** is the order that was already resting. The **taker** is the order that just arrived.

`Execution` has three methods:
- **`fills()`** returns every trade, in the order they happened.
- **`resting()`** is `Some(id)` if part of the order is now on the book. It is `None` if the order traded out completely. So a caller can tell "still working" from "done" without a second lookup.
- **`filled()`** adds up the quantity of all fills. It returns `Result`, because the sum is checked for overflow (a number too big for its type).

`Fill` derives `Debug`, `Clone`, `Copy`, `PartialEq` and `Eq`, and its fields are public. `Execution` derives `Debug`, `Clone`, `Default`, `PartialEq` and `Eq`.

### The loop decides nothing

Matching walks the other side in the order the containers already hold. Best price comes first from the map. Oldest comes first from the queue. Price-time priority is not coded in the loop. It is read off the structure.

This means the structure is load-bearing. Take `Ord` off `Price`, or swap `VecDeque` for `Vec`, and matching changes without one line of it being edited.

A buy stops when the best ask is above its limit. A sell stops when the best bid is below its limit. That comparison is the only place the two sides differ.

### The price is the maker's

A buy at $29,005 meets a resting ask at $29,000. The trade happens at **$29,000**. The resting order set the terms, and the new one accepted them. So the better price goes to the taker.

Almost every exchange does this, and it matters beyond fairness. If the taker's price were used, two orders far apart would print a trade at a price nobody offered. Say a buy at $35,000 hits an ask at $29,000. Printing $35,000 would be wrong. The printed price is what every client reads as "the market". Test `a_fill_happens_at_the_makers_price` checks this.

### Fills carry no TradeId

A `Fill` names the two orders, the price and the quantity. It does not give the trade an id.

Ids here are counters stamped at one fixed entry point (see `DATA_MODEL.md` and [ids.md](ids.md)). Replaying a log of commands must produce the same ids every time. The book holds no counter and must not grow one. The command layer turns each `Fill` into a `Trade` with an id and a `SeqNo`.

### Both orders move together

Each fill applies the same quantity to the maker and the taker, through `RestingOrder::fill`. That method changes `qty_remaining` and `status` in one step, so neither can be left half done.

- **A maker that reaches zero** is popped off the front of its level and removed from the index right away.
- **A maker with quantity left** keeps its place at the front. It was there first, and it still is. In the example, Dave keeps 0.2 BTC and stays first at $29,000.

## The order index

`DATA_MODEL.md` once called this "gap, and the important one". Here is why.

A cancel arrives with an order id and nothing else. Without an index, the book would have to scan every level on both sides to find it. That is the whole book, for one of the most common commands.

`HashMap<OrderId, (Side, Price)>` turns that into one hash lookup. The lookup gives the side and the price. Those point to the exact level. Only that one level is scanned.

**The cost.** The index is a second copy of the truth, and two copies can disagree. So every `insert` writes both structures, and every `remove` clears both.

## Why insert takes a whole RestingOrder

The order already knows its side and price. Passing them again as separate arguments would let them disagree with the order's own fields:
```rust
book.insert(order, Side::Sell, price);   // but order.side() is Buy. Which wins?
```
That is the same mistake `OrderKind` was shaped to prevent one layer down.

## Empty levels are removed

When the last order at a price leaves, the level is dropped from the map. It is not left behind as an empty queue.

If it stayed, `best_bid()` would keep reporting that price, even though nothing is there. That is a fake top of book. It would trigger a price update and mislead every client watching. Test `removing_the_last_order_at_a_price_drops_the_level` checks this. The matching loop drops empty levels the same way.

## What the book refuses, and nothing more

Every command touches the book, so it is on the **hot path**: the code that must be fastest. It checks as little as it can. The rule is:

> The book rejects only what would corrupt the book. Everything else is decided above it.

That leaves two errors. Both are about the book's own structures, not about the order:

| error | when | what breaks without it |
|---|---|---|
| `DuplicateOrderId` | the id is already in the index | The index holds one entry per id. A second insert would overwrite the first and strand the original order in its level. `remove` could never reach it again. |
| `OrderNotFound` | the id is not in the index | There is nothing to remove. |

Neither one judges whether the order *should* be there. Is it live? Does the user have the money? May this account trade this coin? Those questions belong to the command layer. It has the context to answer them, and it pays the cost once instead of on every book operation.

**Self-trade prevention is the exception.** It must live in this file. It is a decision made per fill, about which resting order the taker is about to hit. Nothing above the matching loop can make it. It is not built yet. See "Not done yet".

**A check that was removed.** An earlier `insert` also refused an order whose status was already finished. It came out, because no such order can reach `insert` today. `RestingOrder::new` always gives `Open`. Only `fill` and `cancel` move an order off `Open`. `Rejected` has no code path at all. The check guarded a state that could not happen, on the hot path, forever.

## Lookup counts

**`remove`** touches the hash map once. `HashMap::remove` returns the `(Side, Price)` and clears the entry in the same step, instead of a `get` followed by a `remove`. After that, each removal is: one B-tree lookup, one scan of one level, and one removal from the queue. The scan costs the most, and levels are short.

**`insert`** costs one hash lookup if the order fills completely, and two if it rests. The duplicate check must happen *before* matching. A rejected order must not have traded on its way to being rejected. But whether anything needs indexing is only known *after* matching. An earlier version claimed the slot up front with `Entry::Vacant` and did it in one lookup. That no longer works. It no longer matters either, because matching costs far more than one lookup.

## Removal does not set the status

`Book::remove` returns the order exactly as it was. It still says `Open` or `PartiallyFilled`. It does **not** call `RestingOrder::cancel`.

This is on purpose. Taking an order out of a container and declaring it cancelled are two different facts. The book knows the first. Only the caller knows the second. A fully matched order also leaves the book, and that one is `Filled`, not `Cancelled`.

So the command layer calls `RestingOrder::cancel` on what it gets back, when the reason really was a cancel. `RestingOrder` still owns the change (see [resting_order.md](resting_order.md)). The book just does not trigger it.

**Remember this.** A caller that forgets will hold an order that says `Open` after it has left the book. Test `remove_returns_the_order_without_touching_its_status` records this, so nobody mistakes it for a bug.

## amend

Issue #7 decided that an amended order loses its place in the queue. `amend(id, replacement)` takes the old order out, sends in the replacement, and returns both the old order and the replacement's `Execution`:
```rust
pub fn amend(&mut self, id: OrderId, replacement: RestingOrder)
    -> Result<(RestingOrder, Execution)>
```

**The replacement can trade.** It goes through `insert`, so it is treated as a new arrival and matches before resting. Say Alice has a bid at $28,900 and moves it up to $29,000, where Carol is selling. Alice trades right away. She does not sit at the back of a level she should never have reached. That is why the `Execution` is returned and not thrown away. Test `an_amended_order_matches_like_a_new_arrival` checks this.

**Why a separate method.** `insert` and `remove` say what they do to the container, not why. `amend` is the one method named for an intention, because the order of its steps carries meaning. Doing it by hand is the natural mistake:
```rust
let order = book.remove(id)?;   // came out of the book
book.insert(order)?;            // and goes straight back in
```
This puts the *same* order back, with the same id and `SeqNo`. It loses its queue place by accident, not by rule, and the code reads as if nothing happened. A replacement should be a fresh `RestingOrder::new` with a new id and `SeqNo`. Going to the back is the point of the decision, not a side effect.

**One extra check, for atomicity.** This is the one place the book spends a check on something other than corruption. `amend` makes sure the replacement's id is free *before* removing anything. Without it, the steps could not be undone. If `insert` failed with `DuplicateOrderId`, the old order would already be gone. Putting it back would not restore its place in the queue. One hash lookup makes the whole thing all-or-nothing. It only costs anything on amend, so the hot paths are untouched. Test `amend_rejects_a_replacement_id_already_in_the_book` checks this.

**Reusing the same id is allowed.** The slot is free by the time the insert runs. The replacement still goes to the back.

## No panics on the removal path

Between reading `(side, price)` from the index and taking the order out of its level, three lookups "cannot" fail:
- the level must exist,
- the order must be in it,
- its position must be valid.

Each is an internal promise. Each is written as `let ... else` that returns `OrderNotFound`, not as an `expect` that crashes.

**The trade-off.** An `expect` would announce corruption loudly, which has real value in an engine. Returning an error means real corruption would show up as a confusing "no such order" instead of a crash. The engine's standing rule is to report, not panic (see the overflow discussion in [quantity.md](quantity.md)), so this follows it. If a `BookCorrupted` error is ever wanted to tell the two cases apart, this is where it goes.

## What the iterators show

`bids()` and `asks()` yield `&RestingOrder` in match order: best price first, oldest first within a price. They flatten the levels, so callers never see a `VecDeque`. `get_by_id` uses the index to find one order without scanning. `len` and `is_empty` count entries in the index.

The `Level` and `BookSide` type aliases are **private**. `DATA_MODEL.md` names the structure, but naming it in a design is not a promise in the API. Keeping them private means the layout can change without breaking callers.

## Changing under #21

The contract in issue #21 changes several things here. See Layers 6 to 8 and "What changes in the #20 code" in [DATA_MODEL.md](DATA_MODEL.md). In short:
- **One book per market**, not per coin, held in a map keyed by market symbol.
- **`Fill` becomes `TradeSettlement`.** It gains a trade id, both holders, all four accounts, the quote amount, both fees, the taker side and the fee account.
- **`Execution` becomes `CommandResponse` plus a list of events.**
- **`Open` leaves `OrderStatus`.** A removed order would say `New` or `PartiallyFilled`.
- **Self-trade has a rule now.** Skip the colliding quantity on the incoming order and leave the resting one alone. If nothing is left to match, admit the order, then cancel it with reason `SELF_TRADE`. Whether "skip" means stepping over the own order or shrinking the incoming one is still open.
- **Level totals.** Each level may keep a running total of its quantity, for market data.

## Not done yet

- **Self-trade prevention is missing, and it matters now.** Issue #7 decided a user's own buy must not match their own sell. Matching does not check, so **today a user can trade with themselves**. `RestingOrder` carries `user_id`, so the book already has what the check needs. What is missing is the policy. The three usual answers behave differently:

  | policy | what happens |
  |---|---|
  | skip the maker | pass over it and match the next order at that price. The resting order stays. |
  | cancel the maker | pull the resting order out, then keep matching |
  | reject the taker | refuse the incoming order outright |

  Skipping is the least surprising and the cheapest. But it lets a user hold a resting order that blocks nothing while their own orders walk past it. Cancelling the maker is what most exchanges do. Rejecting the taker is the strictest, and the most annoying for anyone running two strategies on one account. The decision must be recorded on the ticket, not only here. `DATA_MODEL.md` now records a rule for #21 (see above).

- **No `TimeInForce` handling.** Only a limit order that may rest can be expressed. `Market`, `ImmediateOrCancel` and `FillOrKill` have no entry point, because `RestingOrder` has neither `OrderKind` nor `TimeInForce`.
  - **IOC** (immediate or cancel) means dropping the remainder instead of resting it.
  - **FOK** (fill or kill) must check the whole quantity is available *before* any fill. The current loop cannot do that, because it fills as it walks.
  - Issue #7 also decided a market order short of liquidity fills what it can and errors on the rest.

- **Seen idempotency keys.** `DATA_MODEL.md` listed the dedupe set as a gap. It grows forever, so it needs an eviction policy (a rule for dropping old entries) first. See the same note in [ids.md](ids.md).

- **Shrinking an order should keep its place.** `amend` sends every change to the back, as issue #7 decided. Real exchanges are usually less strict:

  | change | queue place |
  |---|---|
  | price | always lost. The order joins the back of a different level. |
  | quantity down | **kept** |
  | quantity up | lost |

  This is a fairness rule. Shrinking takes nothing from the orders behind you. There is now *less* ahead of them. Charging for it only pushes people to cancel and replace instead. Growing does take from them, because it claims fills ahead of orders that came before the increase. So it goes to the back. Some exchanges keep the place for the original amount and queue only the extra. Rules vary, so confirm against whichever exchange we model.

  Doing it properly needs three things that do not exist yet:
  1. **An in-place reduction.** The order must be changed where it sits, not removed. That is a mutable path the book deliberately does not have.
  2. **A new `RestingOrder` method.** Say an order of 10 has 4 filled, and is reduced to 6. That leaves `qty_original = 6` and `qty_remaining = 2`. Today only `fill` lowers `qty_remaining`, and it changes `status` as it goes. A reduction must not.
  3. **A decision on the edge case.** Reducing to 3 when 4 has already traded gives a negative remainder. Exchanges usually treat that as a cancel, since the order is finished, not as an error.

  Raise it on issue #7 first. The current behaviour is a team decision, not an oversight.

- **`amend` does not build the replacement.** It takes one. Deciding what the new order looks like is the command layer's job: which fields the user may change, and what its new id and `SeqNo` are. Only the command layer knows where ids come from.
