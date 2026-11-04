# Sprint plan: #52 PlaceOrder and multithreading

Sprint issues, created 2026-10-27:

| Issue | Title | Body |
|---|---|---|
| [#52](https://github.com/for-legal-reasons-this-is-game/For-legal-reasons-this-is-game/issues/52) | Implement PlaceOrderRequest and multi threading | Multithreading needs to be drawn up with Yousef; Michal helps with how it is handled through Kubernetes. |
| [#53](https://github.com/for-legal-reasons-this-is-game/For-legal-reasons-this-is-game/issues/53) | Modify the contract | Resolve #37. Reminder: a `RejectReason` for runtime errors. |

#52 is the engine work. #53 changed the contract; it was merged as PR #56 ("Modify contract", `aefaeca`), so #52 is now built against the new contract.

Related: [SLICES.md](SLICES.md) (slice 2 and slice T), [DATA_MODEL.md](DATA_MODEL.md), and the exercises for this sprint in [exercises/](exercises/README.md).

Local only, like the other docs here.

---

## Status

| Step | Status |
|---|---|
| PR #51 SetMarket | merged 2026-10-27 |
| PR #54 CI job for engine tests | merged |
| #53 contract remake | merged (PR #56). What it changes for the engine: [below](#what-the-contract-remake-changed) |
| `RUNTIME_ERROR` versus panic | answered by #53: `INTERNAL_ERROR` only for queued commands, see [below](#1-internal_error-versus-panicking-answered) |
| Exercises for PR A | ready: [exercises/](exercises/README.md), 00 to 07 |
| PR A: PlaceOrder | todo |
| PR B: multithreading | not mine, see [below](#pr-b-multithreading-not-mine) |

---

## What the contract remake changed

Source: `git show aefaeca`, files `proto/*.proto`, `proto/how_to_use.md`, `proto/devops.md`.

| Change | Before | Now | Hits |
|---|---|---|---|
| `REJECT_REASON_INTERNAL_ERROR = 10` | none | Only for a command still **queued** when the engine panics or gets SIGTERM, so it was **not processed**. Never cached. | PR B, the error mapping in PR A |
| `engine_epoch` | on `EventItem`, `MarketDataItem`, `SubscribeEventsRequest` | **Removed** everywhere, and from the backend's `trades` table | slice 6 (`trade_id`), slice 7 |
| Snapshots | `Control`, `EXEC_TYPE_RESTATED`, `after_sequence` | **Removed.** `SubscribeEventsRequest {}`: every subscription replays the whole retained history, then goes live | slice 7 |
| Engine state | in memory, lost on restart | Books, command cache, event history and the last `sequence` are **in Redis**. On restart the engine loads them before it answers anything | PR B, slice 5, slice 7 |
| Write before send | Redis log written on the side, "never block matching" | **Nothing leaves the engine until the state it depends on is in Redis**: run a batch, save once, then answer | PR B |
| `sequence` | one per process, starts at 1 per epoch | One per **instance**, continues across restarts | PR B (sequencer) |
| `trade_id` | proposed: epoch in the high 64 bits, a counter in the low 64 | Time-ordered unique id (`tb::id()` or UUIDv7), strictly increasing within one instance | slice 6 |
| Topology | open: which pod owns which market | Several instances, **at most 8 markets each**, every market owned by exactly one. A separate **Router** service forwards commands and merges the streams. An event consumer is the only reader | PR B |
| Instance identity | none | The pod name (`engine-0`, ...) from the downward API, used as the prefix of its Redis keys. `StatefulSet` is still marked TBD in `devops.md` | PR B |
| Shutdown | none | SIGTERM or panic: stop taking commands, answer the queue with `INTERNAL_ERROR`, save to Redis, exit | PR B |
| Restart, backend side | cancel and refund every working order | Nothing to do: orders keep working, don't send `SetMarket` again | none in the engine |

Not changed: `PlaceOrderRequest`, `PlaceOrderResponse` (only a comment), the validation table, `SetMarket`, matching, the market data stream except the removed epoch. **PR A is not affected beyond the error mapping.**

---

## Clear up first

### 1. `INTERNAL_ERROR` versus panicking (answered)

#53 answers the postponed question: `INTERNAL_ERROR` is **not** a mapping target for errors that happen while a command runs. It is only the answer to commands that were queued and never ran.

- `Overflow` stays `BAD_QUANTITY`, as in the [validation table](DATA_MODEL.md#validation-rule-mapped-to-the-proto-fields).
- A poisoned lock or a broken invariant is still a panic.
- New: the panic must not end the process before the queued commands are answered and the state is saved. That is PR B's problem, see [below](#pr-b-multithreading-not-mine). Exercise: [ex05](exercises/exercises/ex05_reject_mapping.rs).

### 2. What #53 changes (settled)

See the table above. PlaceOrder can start now; nothing in its messages changed.

---

## PR A: PlaceOrder, single-threaded

This is [slice 2](SLICES.md#2-placeorder-limit-gtc-base-quantity-only). Start it now. It does not need the threading design: keep the per-market work in `MarketInfo` methods, and slice T moves it onto a task later.

**Practice first:** exercises 00 to 07 cover every step below.

### Steps

1. **Branch** `place_order` from `main`.
2. **Tests first:**
   - a sell, then a crossing buy: the buy's response is `FILLED` with the right filled amounts
   - unknown market: `UNKNOWN_MARKET`
   - zero quantity: `BAD_QUANTITY`
   - zero price: `BAD_PRICE`
   - market, IOC, FOK and stop orders: `UNSUPPORTED`
   - the `HAS_OPEN_ORDERS` test postponed from #51: place a real order, then reconfigure the market's decimals
   - `Overflow` (a quote over 128 bits) is `BAD_QUANTITY`, never `INTERNAL_ERROR`
3. **Conversion in `wire.rs`** (ex01, ex02):
   - `i32` to `Side`, `OrderType`, `TimeInForce`, rejecting `UNSPECIFIED` and unknown numbers ([Layer 2](DATA_MODEL.md#layer-2-enums-all-proto-from-commonproto))
   - `required()` for the ids and the price ([Wire conventions](DATA_MODEL.md#wire-conventions-proto))
   - only the `base_quantity` arm of `amount`
4. **Validation in the contract's order** (ex03): one function, one early return per row of the table.
5. **`MarketInfo::place(order) -> Execution`**, calling `book.insert` (ex07).
6. **`Engine::place_order(request) -> PlaceOrderResponse`** (ex04, ex06): `status`, `filled_base`, and `filled_quote` as the **sum of `quote_for` per fill**. Rounding is per fill ([Quote math](DATA_MODEL.md#quote-math-rule)), so one `quote_for` on the total can give a different number.
7. **Error mapping** (ex05): every `EngineError` variant listed by name, no `_` arm, the undefined ones panic.
8. **Wire it in `service.rs`**, and add a test in `tests/service.rs`.

### Decisions for this PR

| Question | Proposal | Source |
|---|---|---|
| The engine's `OrderStatus::Open` has no proto value | Report it as `New` | [Layer 2](DATA_MODEL.md#layer-2-enums-all-proto-from-commonproto) |
| A required `Id` or a known enum is missing | `BAD_ORDER_PARAMS`, confirm with the team | [Requests](DATA_MODEL.md#requests-proto-from-serviceproto) |
| `DuplicateOrderId` from `Book::insert` | Still open: `UNSUPPORTED`, or a panic (the backend never reuses an order id). Pick one and say so in the PR | [Layer 1](DATA_MODEL.md#layer-1-identity) |
| `INTERNAL_ERROR` | Never produced by PR A. It needs a queue, which only exists after PR B | #53 |
| Fees and accounts in the request | Not stored yet, they come with slice 6. Say so in the PR. | [Slice 6](SLICES.md#6-fees-and-tradesettlement) |
| Duplicate `command_id` | Not in this PR, that is slice 5 | [Slice 5](SLICES.md#5-duplicate-commands) |

### Done when

All the tests above pass, through `Engine` and through the gRPC handler, and `cargo clippy --all-targets -- -D warnings` is clean.

---

## PR B: multithreading (not mine)

Someone else owns slice T. What matters for PR A: keep all per-market work in `MarketInfo` methods that need nothing outside the market, so PR B can move a market onto its own task without a rewrite.

Contract points found while reading #53, worth passing to the PR B owner:

- **Panics with `panic = "abort"`.** `how_to_use.md` wants a panic to answer the queue with `INTERNAL_ERROR` and save to Redis before exiting. With `abort` only a panic hook runs, and saving would persist the half-updated book of the market that panicked.
- **A 9th `SetMarket`.** One instance owns at most 8 markets, but `ConfigRejectReason` has no reason for "instance full".
- **Write before send.** No response may leave before its batch is saved to Redis, which also affects where the command cache (slice 5) lives.

---

## Order for the week

1. Exercises 00 to 07 (`cd exercises && cargo run`).
2. PR A, PlaceOrder.
