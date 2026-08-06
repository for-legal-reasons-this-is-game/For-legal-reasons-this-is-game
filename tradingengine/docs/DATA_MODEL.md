# Engine data model

Derived from the matching-engine diagram. Every row is marked:

- **drawn** — stated in the diagram
- **gap** — the diagram needs it but does not name it; proposal given
- **decided** — a fork that has been settled; see Decisions below

The engine is sharded one single-threaded actor per coin (MEC 1..n). That single
fact removes a field from almost every structure below: nothing inside a shard needs
to say which coin it is, because the shard *is* the coin.

---

## Decisions

**Order type splits into two fields.** `OrderKind { Market, Limit { price } }` for the
price instruction, `TimeInForce { GTC, IOC, FOK }` for lifetime. The diagram's single
four-variant enum merges two independent axes, which makes "market IOC" and "limit FOK"
inexpressible. Splitting them also makes a market order carrying a limit price
unrepresentable rather than merely invalid.

**The engine never sees a balance.** The backend reserves funds before the command
arrives and settles them from `TradeExecuted` / `OrderCancelled`. No `Account`,
`Balance` or `Position` type enters this crate. Engine state is the book plus the
dedupe set, nothing more. Matches `docs/BACKEND.md`.

**Sequence numbers are per-coin, assigned inside the MEC at dequeue.** Not at the
router, not in a transport handler. Three reasons, and they come from the gRPC
discussion in issue #6:

- A single counter ahead of the fan-out serializes every command through one point,
  discarding the independence the per-coin sharding exists to provide, and preventing
  the MECs from ever becoming separate processes.
- gRPC guarantees ordering only *within* a stream. Across shards there is no
  transport-level ordering to preserve, so a global number would assert an ordering the
  wire never promised.
- Assigning at the router means thread-race scheduling picks the sequence numbers,
  which is the nondeterminism trap the ticket warns about. Assigning at dequeue makes
  the shard's own inbox the authority.

A system-wide order for audit, if ever needed, comes from the append order of the event
log — already the source of truth — not from an ingress counter.

**`PriceUpdated` fires on top-of-book change**, carrying best bid and best ask. It
therefore fires on a resting order or a cancel that moves the top, not only on trades.
This is what gives clients a live mid-price.

**`seq_no` orders, `ts` describes.** The legend says seq no replaces wall-clock, and
`Trade` also carries `ts`. Both are kept, with distinct jobs: `seq_no` is the ordering
and replay authority; `ts` is display and audit metadata, stamped once at ingress and
carried in. The engine never reads a clock.

---

## Layer 0 — values

| Type | Status | Notes |
|---|---|---|
| `Money` | **drawn** — "integer minor units (no floats)" | Signed: balances, notionals, P&L. Not implemented. |
| `Price` | **drawn** — the `BTreeMap` key | Strictly positive; must be a whole multiple of the coin's tick. Integer keys are what make the map ordering exact. |
| `Quantity` | **drawn** | Must be a whole multiple of the coin's lot. |
| `Tick`, `Lot` | **gap** | Named as keywords, but no structure holds them. They are per-coin, so they need a home — see `CoinSpec`. |
| `CoinSpec { coin, symbol, tick, lot }` | **gap** | Without it there is nowhere to validate a price against the tick grid. One per MEC. |
| `Timestamp` | **decided** | Kept alongside `seq_no`, but only as display/audit metadata. Stamped at ingress, carried in, never read from a clock inside the engine. |

## Layer 1 — identity

| Type | Status | Notes |
|---|---|---|
| `UserId` | **drawn** | Comes in on the command. Backend uses UUIDs. |
| `OrderId` | **drawn** — `RestingOrder.id` | |
| `SeqNo` | **decided** | Monotonic and deterministic. **Per-coin**, assigned inside the MEC at dequeue — not at the router, not in a transport handler. |
| `IdempotencyKey` | **drawn** | Dedupes retried commands. |
| `CoinId` | **drawn** — "Coin" on the command | Used by the router to pick a MEC. Deliberately absent from `RestingOrder`. |
| `TradeId` | **gap** | `Trade` has no id. Needed to reference an execution from a ledger entry or a support query. |

## Layer 2 — the order

Drawn: `RestingOrder { id, user_id, side, price, qty_remaining, seq_no, idempotency_key }`

| Field | Status | Notes |
|---|---|---|
| `qty_original` | **gap** | The DB box sets order status to `FILLED`/`PARTIAL`, which cannot be computed from `qty_remaining` alone — "30 remaining" is meaningless without knowing it started at 100. |
| `status` | **gap** | `OPEN`, `PARTIAL`, `FILLED` appear in the DB boxes but no enum is listed. Also needs `CANCELLED` and `REJECTED`. |
| order type | correctly absent | Only resting orders live here, and only Limit GTC rests. Market, IOC and FOK never become a `RestingOrder`. |

`Side { Buy, Sell }` — **drawn**.

`OrderKind { Market, Limit { price } }` and `TimeInForce { GTC, IOC, FOK }` —
**decided.** The diagram's single `Market | Limit | IOC | FOK` enum is split into the
two independent axes it was conflating: what price I will accept, and how long the
order lives. Encoding the price inside the `Limit` variant means a market order
carrying a limit price cannot be constructed at all.

## Layer 3 — the book

| Structure | Status | Notes |
|---|---|---|
| `BookSide = BTreeMap<Price, VecDeque<RestingOrder>>` | **drawn** | Bids iterate descending, asks ascending; FIFO within a price. That is price-time priority exactly. |
| `Book { bids, asks }` | **drawn** | One per coin. |
| `orders: HashMap<OrderId, (Side, Price)>` | **gap, and the important one** | `CancelOrder` arrives with an order id. Without this index, cancelling means scanning every price level of both sides. Cancels are frequent. |
| Top-of-book | **drawn** | Best bid / best ask, read before matching. |
| Seen idempotency keys | **gap** | Something must remember which keys were already processed, or the dedupe rule cannot be enforced. Unbounded growth — needs an eviction policy. |

## Layer 4 — commands

Drawn: `PlaceOrder | CancelOrder | AmendOrder`

| Command | Status | Payload |
|---|---|---|
| `PlaceOrder` | **drawn** | The left-hand box: user_id, side, order_type, limit_price (absent for Market), quantity, coin, idempotency_key. Plus `seq_no` stamped at ingress, plus the "liquidity held" assertion from the authorized-command box. |
| `CancelOrder` | **gap** | Not specified. Needs order_id, user_id (so one user cannot cancel another's order), idempotency_key. |
| `AmendOrder` | **gap, and unclear** | Not specified at all. What is amendable — price, quantity, both? On a real exchange amending price forfeits time priority, and so does increasing quantity; reducing quantity usually keeps it. |

## Layer 5 — events

Drawn: `OrderAccepted | OrderRejected | TradeExecuted | PriceUpdated | OrderCancelled`

| Event | Status | Notes |
|---|---|---|
| `OrderAccepted` | **drawn** | |
| `OrderRejected` | **drawn** | **gap:** no rejection reason. The client cannot act on a bare rejection. Needs an enum: unknown coin, bad tick, bad lot, insufficient held liquidity, duplicate key, market halted. |
| `TradeExecuted` | **drawn** | Carries `Trade { taker, maker, price, qty, ts, seq }`. **gap:** are `taker`/`maker` order ids or user ids? Settlement needs the user, the book needs the order — probably both. |
| `OrderCancelled` | **drawn** | |
| `PriceUpdated` | **decided** | Fires on **top-of-book change**, carrying `best_bid: Option<Price>` and `best_ask: Option<Price>`. So it fires on a resting order or a cancel that moves the top, not only on trades — which is what gives clients a live mid-price. Both fields are optional because a side can be empty. |
| `OrderAmended` | **gap** | `AmendOrder` is a command with no corresponding event, so an amend cannot be replayed. |

`Match fn: (BookState, Command) -> (BookState, Vec<Event>)` — **drawn.** Pure and
replayable. This signature is the contract the whole design hangs on: no IO, no clock,
no randomness, so the same inputs always produce the same outputs.

## Layer 6 — outside the engine

Drawn as DB responsibilities, listed here so the boundary is explicit: append-only
event log (source of truth), market depth / tape, balances projection, per-user order
history and status, held liquidity.

**Held liquidity** is the boundary worth naming, and it is **decided**: the engine
never sees a balance. The backend reserves funds before the command reaches the engine
("Authorized command: liquidity held") and settles them from the events afterwards
("Liquidity settled"). The engine matches orders and emits events; the balances
projection reacts. No `Account`, `Balance` or `Position` type enters this crate.

One consequence to design for: the engine can therefore never reject an order for
insufficient funds. That check has to happen upstream, before the command is admitted,
and the "authorized command" box is where it lives.

---

## Not in the diagram at all

- **Fees.** No maker/taker fee, no fee event, no fee field on `Trade`. If fees are ever
  charged, `Trade` grows and the taker/maker distinction starts to matter economically.
- **Self-trade prevention.** Nothing stops a user's buy from matching their own sell.
  Real venues prevent it; decide whether this one cares.
- **Market order against an empty book.** A market order cannot rest. Reject it, or
  fill what it can and drop the residual?
- **Halts.** No market state — nothing can stop trading on a coin.
- **Snapshots.** The book is "rebuilt from event log". With no snapshot, restart time
  grows without bound as the log grows.
