# Engine work plan: slices

The design in [DATA_MODEL.md](DATA_MODEL.md) is written in layers: values, ids, enums, market, commands, order, state, matching, events. That shows how the pieces fit together, but the layers only work together at the very end.

This file cuts the same work into **slices** instead. A slice is one small feature that works end to end and has a test that proves it. You finish a slice, see it work, then start the next one.

For each slice:

- **Needs slices** lists the slices that must be done first.
- **Reads from DATA_MODEL** links the sections you need for the design.
- **Open questions** links the parts of the design that are not decided yet. Decide or ask before coding that part.
- **Done when** is the test that proves the slice works.

Local only, like DATA_MODEL.md (both are in `.git/info/exclude`).

The current sprint's plan is in [SPRINT.md](SPRINT.md), with exercises in [exercises/](exercises/README.md).

**Contract remake (#53, PR #56):** `engine_epoch`, snapshots, `Control` and `RESTATED` are gone; the engine's state lives in Redis and every response waits for its save; one instance owns at most 8 markets; `INTERNAL_ERROR` answers commands that were queued when the engine stopped. Slices 5, 6, T and 7 below are updated for it. See [SPRINT.md](SPRINT.md#what-the-contract-remake-changed).

---

## The picture

What exists today and what each slice adds:

```
 backend ──gRPC──►  service.rs  ──►  Engine (markets)  ──►  Book  ──►  fills
                    EMPTY            DOES NOT EXIST         WORKS
                    every method     no place to keep       insert, match,
                    "unimplemented"  markets or books       remove, modify
```

The order of the slices and what each one needs:

```
 1 SetMarket
   └─► 2 PlaceOrder (limit, GTC)
         ├─► 3 CancelOrder
         │     └─► 4 ModifyOrder
         ├─► 5 Duplicate commands   (wraps 2, 3, 4)
         └─► 6 Fees + TradeSettlement

 4 + 5 ─► T One thread per market ─► 7 Event stream   (7 also needs 6)

 Book only, any time, no gRPC needed:
   A IOC     B FOK     C Market sell ─► D Market buy by quote     E Self-trade
```

## Status

| Slice | Status |
|---|---|
| Book: insert, match, remove, modify | done |
| 1 SetMarket | done (PR #51, merged 2026-10-27) |
| 2 PlaceOrder | todo |
| 3 CancelOrder | todo |
| 4 ModifyOrder | todo |
| 5 Duplicate commands | todo |
| 6 Fees + TradeSettlement | todo |
| T One thread per market | todo |
| 7 Event stream | todo |
| A IOC | todo |
| B FOK | todo |
| C Market sell | todo |
| D Market buy by quote | todo |
| E Self-trade | todo |

---

## The main path

### 1. SetMarket

Give the engine a place to keep markets, and let the backend create one.

| | |
|---|---|
| **Needs slices** | none |
| **Reads from DATA_MODEL** | [Layer 3: the market](DATA_MODEL.md#layer-3-the-market), [Layer 0: Implemented](DATA_MODEL.md#implemented) (`QuoteScale` already exists), [Layer 6: engine state](DATA_MODEL.md#layer-6-engine-state) (`markets` map), [Wire conventions](DATA_MODEL.md#wire-conventions-proto) (`market` can be absent) |
| **Open questions** | What counts as a structural change for `HAS_OPEN_ORDERS` ([Layer 3](DATA_MODEL.md#layer-3-the-market)). The first version can skip it and only add new markets. |
| **Code** | new `src/engine.rs` with `Engine { markets: HashMap<String, MarketState> }` and `MarketState { market, scale: QuoteScale, book: Book }`. `service.rs::set_market` calls it, behind a `Mutex`. |
| **Already exists** | `TryFrom<&Market> for QuoteScale` in `wire.rs`, the `DecimalsOutOfRange` check |
| **Keep in mind** | each market will get its own thread in slice T. Put all per-market work in methods on `MarketState` that need nothing outside it, and keep `order_index` and the cache on `Engine`. See [One thread per market](DATA_MODEL.md#one-thread-per-market-proposed). |
| **Done when** | a test calls `engine.set_market(btc_usd)` and then finds the BTC-USD book. A market with `k` above 38 is rejected with `BAD_DEFINITION`. |

### 2. PlaceOrder: limit, GTC, base quantity only

The first real trade through the engine. Everything that is not a plain limit order returns `UNSUPPORTED` for now.

| | |
|---|---|
| **Needs slices** | 1 |
| **Reads from DATA_MODEL** | [Requests](DATA_MODEL.md#requests-proto-from-serviceproto) (`PlaceOrderRequest`), [Validation](DATA_MODEL.md#validation-rule-mapped-to-the-proto-fields) (only the rows for limit orders), [Parsed command](DATA_MODEL.md#parsed-command-decided), [Responses](DATA_MODEL.md#responses-proto-from-serviceproto), [Layer 2: enums](DATA_MODEL.md#layer-2-enums-all-proto-from-commonproto) (`i32` to `Side`, `UNSPECIFIED` is a rejection), [Layer 5: RestingOrder](DATA_MODEL.md#restingorder-proposed), [Wire conventions](DATA_MODEL.md#wire-conventions-proto) (absent versus zero) |
| **Open questions** | A missing required `Id` has no reject reason, `BAD_ORDER_PARAMS` is only proposed ([Requests](DATA_MODEL.md#requests-proto-from-serviceproto)). `OrderStatus::Open` in the code versus `New` in the proto ([Layer 2](DATA_MODEL.md#layer-2-enums-all-proto-from-commonproto)). |
| **Code** | `Engine::place_order`: look up the market, convert the request with `required()` and the `wire.rs` conversions, build a `RestingOrder`, call `book.insert`, compute `filled_quote` with `scale.quote_for`. `service.rs::place_order` calls it. |
| **Already exists** | `Book::insert`, `RestingOrder::new`, `required()`, `Price::try_from`, `QuoteScale::quote_for` |
| **Done when** | a test places a sell and a crossing buy through `Engine`, and the buy's response is `FILLED` with the right filled amounts. Unknown market gives `UNKNOWN_MARKET`, zero quantity `BAD_QUANTITY`, zero price `BAD_PRICE`. After this slice you can try it by hand with `grpcurl`. |

### 3. CancelOrder

| | |
|---|---|
| **Needs slices** | 2 |
| **Reads from DATA_MODEL** | [Validation](DATA_MODEL.md#validation-rule-mapped-to-the-proto-fields) (the cancel and modify table), [Layer 6: engine state](DATA_MODEL.md#layer-6-engine-state) (`order_index`, finished orders), [Layer 1: identity](DATA_MODEL.md#layer-1-identity) (`AccountHolder`) |
| **Open questions** | Telling `UNKNOWN_ORDER` apart from `ORDER_NOT_WORKING` needs a record of finished orders ([Layer 6](DATA_MODEL.md#layer-6-engine-state)). The first version can answer `UNKNOWN_ORDER` for both. |
| **Code** | `order_index: HashMap<OrderId, String>` in `Engine`, filled by place and emptied by cancel and fills. Cancel requests carry no market, so this is how the engine finds the book. |
| **Already exists** | `Book::remove`, `RestingOrder::cancel` |
| **Done when** | place then cancel leaves the book empty. Cancelling someone else's order gives `NOT_ORDER_OWNER`. An unknown id gives `UNKNOWN_ORDER`. |

### 4. ModifyOrder

| | |
|---|---|
| **Needs slices** | 3 (uses `order_index` to find the market) |
| **Reads from DATA_MODEL** | [Modify and queue position](DATA_MODEL.md#modify-and-queue-position-rule), [Requests](DATA_MODEL.md#requests-proto-from-serviceproto) (`ModifyOrderRequest`), [Validation](DATA_MODEL.md#validation-rule-mapped-to-the-proto-fields) (cancel and modify table) |
| **Open questions** | All three `new_*` fields absent: no-op or `BAD_ORDER_PARAMS` ([Requests](DATA_MODEL.md#requests-proto-from-serviceproto)). `new_trigger_price` is `UNSUPPORTED` until stops exist. |
| **Code** | mostly conversion: `Engine::modify_order` finds the market, converts the `new_*` fields, calls `Book::modify`, maps `QuantityNotAboveFilled` to `BAD_QUANTITY`. |
| **Already exists** | `Book::modify` with the queue rules and its tests |
| **Done when** | the queue-position tests pass again through `Engine`: a lower quantity keeps its place, a higher one goes to the back, a crossing price trades. |

### 5. Duplicate commands

Can be built any time after slice 2. It wraps every handler, so do it before the handlers get large.

| | |
|---|---|
| **Needs slices** | 2 (and applies to 3 and 4 once they exist) |
| **Reads from DATA_MODEL** | [Duplicate detection](DATA_MODEL.md#duplicate-detection-store-the-request-no-hash), [Layer 6: engine state](DATA_MODEL.md#layer-6-engine-state) (`CommandCache`), [Layer 1: identity](DATA_MODEL.md#layer-1-identity) (two kinds of duplicate) |
| **Open questions** | The `sent_at` rule ([Duplicate detection](DATA_MODEL.md#duplicate-detection-store-the-request-no-hash)). How long entries are kept (longer than the backend's whole retry period). **Contract (#53):** the cache is saved to Redis with the rest of the state, and an `INTERNAL_ERROR` answer is never stored in it. |
| **Code** | `cache: HashMap<CommandId, (CommandRequest, CommandResponse)>` in `Engine`, checked first in every handler. Store the request with `command_id` and `sent_at` cleared. |
| **Already exists** | `CommandId(u128)` |
| **Done when** | sending the same place twice creates **one** order and both get the same response. Reusing the `command_id` for a different request gives `DUPLICATE_COMMAND`. |

### 6. Fees and TradeSettlement

Turn each fill into the money movement the backend needs.

| | |
|---|---|
| **Needs slices** | 2 |
| **Reads from DATA_MODEL** | [Quote math](DATA_MODEL.md#quote-math-rule), [Layer 0: values](DATA_MODEL.md#layer-0-values) (`FeeBps`), [Layer 1: identity](DATA_MODEL.md#layer-1-identity) (`TradeId`, `AccountHolder`, `AccountId`), [Layer 5: RestingOrder](DATA_MODEL.md#restingorder-proposed) (pinned fees, accounts), [Layer 7: matching](DATA_MODEL.md#layer-7-matching) (fee by role), [TradeSettlement](DATA_MODEL.md#tradesettlement-proto) |
| **Open questions** | Reject `FeeBps` above `10_000` or not ([Layer 0](DATA_MODEL.md#layer-0-values)). ~~Whether `trade_id` must stay unique across restarts~~ **answered by #53:** a time-ordered unique id (`tb::id()` or UUIDv7), strictly increasing within one instance ([Layer 1](DATA_MODEL.md#layer-1-identity)). |
| **Code** | add `maker_fee_bps`, `taker_fee_bps`, `base_account`, `quote_account` to `RestingOrder`. Rename `UserId` to `AccountHolder` and add `AccountId` in `ids.rs`. For each `Fill`, compute the quote with `quote_for`, both fees with `fee_on`, and a time-ordered `TradeId` (48 bits of milliseconds, 80 random bits, previous + 1 within the same millisecond). Good moment to rename `qty_original` to `qty_total` and delete the unused `SeqNo`. |
| **Already exists** | `QuoteScale::quote_for`, `FeeBps::fee_on`, `TradeId(u128)` |
| **Done when** | Alice buys 0.5 BTC from Bob at $29,000 as taker at 10 bps, Bob is maker at 5 bps. The settlement has quote `1_450_000`, buyer fee `1_450`, seller fee `725`, `taker_side = Buy`. |

### T. One thread per market

Move each market onto its own thread. Until now `Engine` is one value behind a `Mutex`, which is correct but lets only one market work at a time.

| | |
|---|---|
| **Needs slices** | 4 (all per-market commands exist), 5 (the cache moves to the router). Easier if 1 to 4 kept per-market work on `MarketState`. |
| **Reads from DATA_MODEL** | [One thread per market](DATA_MODEL.md#one-thread-per-market-proposed), [Layer 6: engine state](DATA_MODEL.md#layer-6-engine-state) (`order_index`, `CommandCache`, sharding) |
| **Open questions** | **Decided by #53:** one instance owns at most 8 markets, a separate Router service decides which instance owns which market, and moving a market is not covered (so no silent-market eviction). Each batch is saved to Redis before its answers go out, and SIGTERM answers the queue with `INTERNAL_ERROR`. Still open: what one save holds, where the command cache lives, the reject reason for a 9th market, and the panic path with `panic = "abort"` ([SPRINT.md, PR B](SPRINT.md#pr-b-multithreading-not-mine)). Write the decisions in an issue first. |
| **Code** | one `tokio` task (or `std::thread`) per market that owns its `MarketState` and reads commands from an `mpsc` channel. Each command carries a `oneshot` sender for its response. A dispatcher (not the contract's Router service, which sits outside the engine) keeps the channels by market symbol, plus `order_index` and the cache, and updates `order_index` from each answer. `SetMarket` spawns the task. Each market task takes a batch, runs it, saves it, then answers. On SIGTERM it answers what is still queued with `INTERNAL_ERROR` and saves. A panic in a market task ends the whole process (`panic = "abort"`); how it answers the queue first is open. |
| **Already exists** | `MarketState` and its methods from slices 1 to 4 |
| **Done when** | all slice 1 to 5 tests still pass through the dispatcher. A new test sends orders to two markets at once and both books end up correct and independent. No answer leaves before its batch is saved. Queued commands get `INTERNAL_ERROR` on shutdown. |

### 7. Event stream

The biggest slice. Do it last on the main path.

| | |
|---|---|
| **Needs slices** | 2, 3, 4, 6 (it reports what they do), T (the sequencer collects events from every market thread) |
| **Reads from DATA_MODEL** | [Layer 8: events](DATA_MODEL.md#layer-8-events), [Envelope](DATA_MODEL.md#envelope-proto-out-of-21-from-eventsprotoeventitem), [ExecutionReport](DATA_MODEL.md#executionreport-proto), [Ordering](DATA_MODEL.md#ordering-rule), [Layer 1: identity](DATA_MODEL.md#layer-1-identity) (`Sequence`; `EngineEpoch` is removed), [Layer 7: matching](DATA_MODEL.md#layer-7-matching) (matching returns events) |
| **Open questions** | Which `command_id` goes on a report: the placing one or the one that caused the change ([RestingOrder](DATA_MODEL.md#restingorder-proposed)). How long history is retained (a fixed window, to be documented). |
| **Code** | a sequencer task owns the one `sequence: u64` for the instance, loaded from Redis at startup (1 on the very first start). Every market task sends its events to it through a channel, and it stamps each one, saves it to the event history, and passes it to `service.rs::subscribe_events`. Every subscription replays the whole retained history, then goes live: no snapshots, no resume position. A subscriber that falls behind is dropped, never waited for. |
| **Done when** | a test collects the events of one trade in the right order: the settlement, then the buyer's `TRADE` report, then the seller's, with increasing `sequence`. |

---

## Book-only slices

These only touch `book.rs` and its tests. No gRPC, no `Engine`. Do them whenever the main path is blocked on a question. Their results show up in the responses once slice 2 exists.

### A. IOC (immediate or cancel)

| | |
|---|---|
| **Needs slices** | none |
| **Reads from DATA_MODEL** | [Layer 2: enums](DATA_MODEL.md#layer-2-enums-all-proto-from-commonproto) (`TimeInForce`, `CancelReason`), [Layer 7: matching](DATA_MODEL.md#layer-7-matching) (IOC row), [Examples](DATA_MODEL.md#examples) (IOC and FOK) |
| **Code** | after matching, an IOC order does not rest. The leftover is cancelled as `IOC_REMAINDER`, or `IOC_NO_FILL` if nothing filled. |
| **Done when** | an IOC order never rests in the book, and the result says why the rest was cancelled. |

### B. FOK (fill or kill)

| | |
|---|---|
| **Needs slices** | none (but its "skip the same owner" part needs E) |
| **Reads from DATA_MODEL** | [Layer 7: matching](DATA_MODEL.md#layer-7-matching) (FOK row), [Examples](DATA_MODEL.md#examples) |
| **Code** | before touching the book, walk the opposite side and check the whole quantity is available at or better than the limit. |
| **Done when** | a FOK that cannot fill completely leaves the book exactly as it was (`assert_eq!(book, before)`). |

### C. Market sell

| | |
|---|---|
| **Needs slices** | none |
| **Reads from DATA_MODEL** | [Parsed command](DATA_MODEL.md#parsed-command-decided) (`OrderKind::Market`), [Incoming order](DATA_MODEL.md#incoming-order), [Layer 7: matching](DATA_MODEL.md#layer-7-matching) (market order on an empty book) |
| **Open questions** | Which `cancel_reason` for a market order on an empty book, and whether market orders behave as IOC ([Layer 7](DATA_MODEL.md#layer-7-matching)). |
| **Code** | match with no price limit. A market order never rests. |
| **Done when** | a market sell fills across several bid levels and never rests. On an empty book it fills nothing. |

### D. Market buy by quote

| | |
|---|---|
| **Needs slices** | C |
| **Reads from DATA_MODEL** | [Layer 7: matching](DATA_MODEL.md#layer-7-matching) (market buy by quote), [Quote math](DATA_MODEL.md#quote-math-rule), [Incoming order](DATA_MODEL.md#incoming-order) |
| **Code** | the buyer says how much quote to spend. Base taken at a level is `floor(remaining_quote * 10^k / price)`, capped by what the level holds. Stop when the remaining quote buys zero base. Needs the market's `QuoteScale`. |
| **Done when** | the $1,000 example from DATA_MODEL gives `3_448_275` satoshis for `99_999` cents. |

### E. Self-trade prevention

| | |
|---|---|
| **Needs slices** | none |
| **Reads from DATA_MODEL** | [Layer 7: matching](DATA_MODEL.md#layer-7-matching) (self-trade row), [Examples](DATA_MODEL.md#examples) (self-trade and its open question), [Layer 1: identity](DATA_MODEL.md#layer-1-identity) (`AccountHolder`) |
| **Open questions** | "Step over" or "decrement" ([Examples](DATA_MODEL.md#examples)). Decide with the team **before** coding this one. |
| **Code** | while matching, an order from the same account holder is skipped instead of filled. |
| **Done when** | Alice's buy never matches Alice's sell, and her resting sell is left unchanged. |

---

## Later

Not needed for a working exchange. Each links where it is described.

| Work | Reads from DATA_MODEL |
|---|---|
| `CancelOrders` (bulk cancel) | [Requests](DATA_MODEL.md#requests-proto-from-serviceproto) (`CancelOrdersRequest`, absent holder means everyone), [Layer 8: events](DATA_MODEL.md#layer-8-events) (`ADMIN` reason) |
| Market-data stream | [Market data](DATA_MODEL.md#market-data-proto-out-of-21-from-eventsprotomarketdataitem) |
| GTD expiry timer | [Layer 2: enums](DATA_MODEL.md#layer-2-enums-all-proto-from-commonproto) (GTD), [Wire conventions](DATA_MODEL.md#wire-conventions-proto) (timestamps) |
| Stop orders | [Layer 2: enums](DATA_MODEL.md#layer-2-enums-all-proto-from-commonproto) (`OrderType`, `TriggerDirection`) |
| Loading the state from Redis at startup (books, command cache, event history, `sequence`; required by #53, `how_to_use.md` "Startup") | [Layer 9: outside the engine](DATA_MODEL.md#layer-9-outside-the-engine), [Errors with no proto meaning](DATA_MODEL.md#errors-with-no-proto-meaning-decided) |

---

## How to work a slice

1. Read the linked DATA_MODEL sections, only those.
2. Answer or ask about its open questions.
3. Write the **Done when** test first. It will not compile yet, that is fine.
4. Write the engine code until the test passes.
5. Connect it in `service.rs` if the slice has a gRPC part.
6. `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
7. Mark it done in the [Status](#status) table.
