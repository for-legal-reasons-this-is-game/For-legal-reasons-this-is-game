# `src/book.rs`

The order book for one coin: two price-ordered sides, plus an index that finds an order
from nothing but its id.

```rust
bids:   BTreeMap<Price, VecDeque<RestingOrder>>
asks:   BTreeMap<Price, VecDeque<RestingOrder>>
orders: HashMap<OrderId, (Side, Price)>
```

There is no `CoinId` field. One book *is* one coin, so storing it would repeat the
container's identity on every order inside it.

## Why `BTreeMap` — price priority for free

A `BTreeMap` keeps its keys sorted; a `HashMap` does not. Because `Price` derives `Ord`
(see [price.md](price.md)), the map does the price half of price-time priority on its
own, and no code has to sort anything:

- **best ask** is the first key — the cheapest thing anyone will sell you.
- **best bid** is the last key — the most anyone will pay.

Both are `next()` / `next_back()` on the key iterator. The asymmetry is the only thing
about the two sides that differs, and it is why `bids()` reverses and `asks()` does not:
for a buyer, higher is better; for a seller, lower is.

## Why `VecDeque` — time priority for free

Within one price, the oldest order matches first. That is a queue, and the two operations
matching needs are "add at the back" and "take from the front".

`VecDeque` does both in O(1). A `Vec` would make `remove(0)` shift every remaining
element down one slot, turning the busiest operation in the engine into O(n).

Cancelling from the *middle* of a level is O(n) in either structure, because the order
has to be found first. That is acceptable: cancels are frequent but levels are short, and
the alternative — an intrusive linked list with a node handle per order — buys speed with
a large amount of unsafe bookkeeping.

## The order index

`DATA_MODEL.md` calls this one **"gap, and the important one"**, and it is worth being
precise about why.

A cancel arrives carrying an order id and nothing else. Without an index, honouring it
means scanning every price level of both sides until the order turns up — the whole book,
for one of the most frequent commands there is.

`HashMap<OrderId, (Side, Price)>` turns that into one hash lookup, which yields the side
and price, which locate the exact level. Only that one level is then scanned.

The price of the index is that it is a **second copy of the truth**, and two copies can
disagree. Every `insert` writes both structures and every `remove` clears both.

## Why `insert` takes a whole `RestingOrder`

The order already knows its own side and price. Passing them alongside it as separate
arguments would create the possibility of them disagreeing with the order's own fields,
which is the exact mistake `OrderKind` was shaped to prevent one layer down.

## Empty levels are removed

When the last order at a price is removed, the level is dropped from the map rather
than left as an empty `VecDeque`. If it stayed, `best_bid()` would keep reporting a price
at which nothing is actually for sale — a phantom top-of-book that would drive a price
update and mislead every client watching. `removing_the_last_order_at_a_price_drops_the_level`
pins this down.

## What the book refuses to do — and nothing more

The book sits on the hot path. Every command touches it, so it validates as little as it
can get away with. The rule is narrow:

> The book rejects only what would corrupt the book. Everything else is decided above it.

That leaves exactly two rejections, and both are about its own structures rather than
about the order:

| variant | when | what breaks without it |
|---|---|---|
| `DuplicateOrderId` | the id is already in the index | the index holds one entry per id, so a second insert overwrites the first and strands the original order in its level — unreachable by remove, permanently |
| `OrderNotFound` | the id is not in the index | nothing to remove; the lookup itself failed |

Neither is a judgement about whether the order *should* be here. That question — is it
live, does the user have the balance, is this a self-trade — belongs to the command layer
above, which has the context to answer it and pays the cost once rather than per book
operation.

An earlier version of `insert` also rejected an order whose status was terminal. It came
out. Nothing in the crate can currently produce such an order before an insert:
`RestingOrder::new` always yields `Open`, only `fill` and `RestingOrder::cancel` move it off `Open`,
and `Rejected` has no code path at all. The check defended a state that could not be
reached, on the hot path, forever.

## One hash lookup, not two

`insert` and `remove` each touch the hash map **once**.

`insert` claims the index slot through `Entry::Vacant` — the same lookup that detects a
duplicate also writes the entry, instead of `contains_key` followed by `insert`. `remove`
uses `HashMap::remove`, which returns the `(Side, Price)` it needs *and* clears the entry
in one pass, instead of `get` followed by `remove`.

What remains per removal: one hash lookup, one B-tree descent, one scan of a single price
level, one deque removal. The scan dominates and levels are short.

## Removal does not set the status

`Book::remove` returns the order exactly as it was, still reporting `Open` or
`PartiallyFilled`. It does **not** call `RestingOrder::cancel`.

That is deliberate, and it is the same rule as above. Removing an order from a container
and declaring an order cancelled are two different facts. The book knows the first. Only
the caller knows the second — the same removal is also how a fully matched order leaves,
and that one is `Filled`, not `Cancelled`.

So the command layer calls `RestingOrder::cancel` on what it gets back, when the reason
for the removal was in fact a cancellation. `RestingOrder` still owns the transition (see
[resting_order.md](resting_order.md)); the book simply is not the one to trigger it.

**The consequence to remember:** a caller that ignores this has an order claiming to be
`Open` after it has left the book. `remove_returns_the_order_without_touching_its_status`
records the contract so it is not mistaken for an oversight.

## `amend`

Issue #7 settled that an amendment loses its place in the queue. `amend(id, replacement)`
takes the old order out, puts the replacement in at the **back** of its level, and returns
what it displaced.

The naming matters here. `insert` and `remove` say what they do to the container and
nothing about why; `amend` is the one operation named for an intention, because it is the
one whose *ordering* carries meaning. Doing it by hand is the natural mistake:

```rust
let order = book.remove(id)?;   // came out of the book
book.insert(order)?;            // ... and goes straight back in
```

That re-inserts the *same* order — same id, same `SeqNo` — which loses queue position by
accident rather than by rule, and reads as though nothing happened. A replacement should
be a fresh `RestingOrder::new` with a new id and `SeqNo`; going to the back is the point
of the decision, not a side effect.

`amend` is also the one place a check is spent on something other than corruption. It
verifies the replacement's id is free **before** removing anything, because the two-step
version cannot be undone: if the insert bounced on `DuplicateOrderId`, the old order would
already be gone, and putting it back would not restore its position in the queue. One hash
lookup buys atomicity, and only on amend — the hot paths are untouched.
`amend_rejects_a_replacement_id_already_in_the_book` holds it in place.

Reusing the id being replaced is allowed: the slot is free by the time the insert happens,
and the replacement still goes to the back.

## No panics on the removal path

Between reading `(side, price)` from the index and removing the order from its level,
there are three lookups that "cannot" fail — the level must exist, the order must be in
it, the position must be valid. Each is an internal invariant, and each is written as a
`let ... else` returning `OrderNotFound` rather than an `expect`.

That is a deliberate trade. An `expect` would announce corruption loudly, which has real
merit in an engine; returning an error instead means genuine corruption would surface as
a puzzling "no such order" rather than a crash. The engine's standing rule is that it
reports rather than panics (see the overflow argument in [quantity.md](quantity.md)), so
that is what this follows. If a `BookCorrupted` variant is ever wanted to tell the two
cases apart, this is where it goes.

## What the iterators expose

`bids()` and `asks()` yield `&RestingOrder` in match order — best price first, oldest
first within a price. They flatten the levels away, so callers never touch a `VecDeque`.

The `BTreeMap` and `VecDeque` aliases are **private**. `DATA_MODEL.md` names the
structure, but naming it in the design is not the same as promising it in the API; keeping
the aliases private means the layout can change without breaking a caller.

## Not here yet

- **No matching.** The book stores and orders; it does not cross a bid with an ask. That
  is step 8, along with commands and events.
- **No mutable access to resting orders.** Matching has to fill the order at the front of
  a level, which needs `front_mut` or `pop_front`. That path is deliberately absent until
  step 8 defines what shape matching actually wants, rather than guessing now.
- **Seen idempotency keys.** `DATA_MODEL.md` layer 3 lists the dedupe set as a gap. It
  grows without bound, so it needs an eviction policy before it can be added — see the
  same note in [ids.md](ids.md).
- **Self-trade prevention.** Motheraudio's answer on issue #7 is that a user's own buy
  should not match their own sell. `RestingOrder` carries `user_id`, so the book already
  holds what the check needs, but the rule itself belongs in matching.
- **A quantity *decrease* should keep its place in the queue.** `amend` currently sends
  every change to the back, which is what issue #7 settled, but it is stricter than real
  venues. The usual rule is:

  | change | queue priority |
  |---|---|
  | price | always lost — the order joins the back of a different level |
  | quantity down | **kept** |
  | quantity up | lost |

  The asymmetry is a fairness rule. Shrinking takes nothing from the orders behind you —
  there is now *less* volume ahead of them — so there is no reason to charge for it, and
  charging only pushes people to cancel-and-replace instead. Growing does take from them,
  claiming fills ahead of orders that arrived before the increase, so it goes to the back.
  (Some venues keep priority for the original quantity and queue only the increment;
  exact rules vary by venue and should be confirmed against whichever one we model.)

  Doing it properly needs three things that do not exist yet:

  1. **An in-place reduction** — mutating the order where it sits rather than removing it,
     which is the mutable path listed above as deliberately absent.
  2. **A new `RestingOrder` operation.** Reducing 10 to 6 with 4 already filled leaves
     `qty_original = 6` and `qty_remaining = 2`. `fill` is currently the only thing that
     lowers `qty_remaining`, and it sets `status` as it goes — a reduction must not.
  3. **A decision on the degenerate case.** Reducing to 3 when 4 has already traded gives
     a negative remainder. Venues generally treat that as a cancel, the order being
     finished, rather than as an error.

  Raise it on issue #7 before implementing: the current behaviour is a team decision, not
  an oversight.
- **`amend` does not build the replacement.** It takes one. Deciding what the amended
  order should look like — which fields the user may change, what its new id and `SeqNo`
  are — is the command layer's job, since only it knows where ids come from.
