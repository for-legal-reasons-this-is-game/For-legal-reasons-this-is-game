# The Event Consumer, explained

This document explains the **event consumer**: what it is for, how it behaves, and how its code fits
together. You can read it without knowing the code. If you do want to read the code, every section
says which file and function it is about, and section 10 explains the short variable names (`ts`,
`l`, `r`, `o`, `fb`, ...) that make the code hard to read without a guide.

The consumer lives in `backend/src/consumer/` (the logic) and `backend/src/bin/consumer.rs` (the
program's entry point). It is a second binary built from the same crate as the backend, so it shares
the backend's libraries and database schema.

## Table of contents

1. [What the consumer is](#1-what-the-consumer-is)
2. [Vocabulary](#2-vocabulary)
3. [Inputs and outputs](#3-inputs-and-outputs)
4. [How it behaves](#4-how-it-behaves)
5. [How the code is organised](#5-how-the-code-is-organised)
6. [The flow of the code, call by call](#6-the-flow-of-the-code-call-by-call)
7. [Deep dives](#7-deep-dives)
8. [Data: who writes what](#8-data-who-writes-what)
9. [What happens when something goes wrong](#9-what-happens-when-something-goes-wrong)
10. [Reading the short variable names](#10-reading-the-short-variable-names)
11. [Tests](#11-tests)
12. [Limits, open decisions, and what is not built](#12-limits-open-decisions-and-what-is-not-built)
13. [Running it](#13-running-it)
14. [A complete example with numbers](#14-a-complete-example-with-numbers)

---

## 1. What the consumer is

The trading engine matches orders. It does not touch money and it does not keep long-term records.
Whenever something happens (an order is accepted, two orders match, an order is cancelled), the
engine emits an **event**. The **consumer** is the one service that reads those events and turns
them into permanent facts:

- **Money moves in TigerBeetle**, the ledger database that is the source of truth for balances.
- **Trades and order states are recorded in Postgres**, so the API can show order history and
  trade history to users.

```
  engine instances (match orders, emit events)
        │   gRPC stream "SubscribeEvents"
        ▼
     router            merges the streams of all engine instances
        │   gRPC stream "SubscribeEvents" (same shape)
        ▼
    CONSUMER  ───────►  TigerBeetle   (moves the money)
        │
        └─────────────►  Postgres     (records trades, updates orders)
```

Two properties drive the whole design:

1. **The consumer is the only writer of trades and of the engine-owned columns of `orders`**
   (status, filled amounts). The backend never writes those, so nothing else can disagree with it.
2. **Every event may arrive many times.** The stream has no "resume from here" feature. Every time
   the consumer subscribes (after a restart, after a dropped connection, or when the router
   reconnects to an engine) it receives the engine's whole retained history again, and then live
   events. So the consumer is built so that handling the same event twice does nothing the second
   time. This is called being *idempotent*, and it is the main thing to understand about this code.

The consumer never talks to an engine directly. It talks to the router, which sits behind the same
`ENGINE_URL` the backend uses.

## 2. Vocabulary

| Term | Meaning |
|---|---|
| **Engine** | The trading engine. Holds order books, matches orders, emits events. Never touches money. |
| **Router** | Sits between the backend/consumer and the engine instances. For events, it subscribes to every engine instance, merges the streams and forwards the events unchanged. Events from different instances interleave; events from one instance keep their order. (The router does not exist in the repo yet.) |
| **Event** (`EventItem`) | One message on the stream. It has a `sequence` number, an `engine_time`, a `market_symbol` and a `body` that is either a trade settlement or an execution report. |
| **Trade settlement** (`TradeSettlement`) | "These two orders matched." It carries both order ids, price, quantities, fees and the accounts involved. This is the event that makes money move. |
| **Execution report** (`ExecutionReport`) | "This order is now in this state." It carries the order's complete current state: status and how much is filled. It does not move money by itself. |
| **Market** | A trading pair such as `BTC-USD`. |
| **Base / quote asset** | In `BTC-USD`, BTC is the base asset and USD is the quote asset. A trade swaps base for quote. |
| **Ledger** | In TigerBeetle, every asset is a ledger number. Accounts and transfers belong to one ledger. Here the ledger number equals the `ledger_id` in Postgres. |
| **Account** | A TigerBeetle account. Each user has one per asset. Its id is a UUID stored in the Postgres `accounts` table. |
| **Hold account** | One TigerBeetle account **per order**, whose id equals the order id. When an order is placed, the backend moves the funds the order might spend into it. A buy reserves the **quote** asset (money to pay with). A sell reserves the **base** asset (the thing to sell). |
| **Transfer** | A movement of an amount from a *debit account* (money leaves) to a *credit account* (money arrives). |
| **Linked chain** | Several transfers sent together that succeed or fail as a unit. If one fails, none are applied. |
| **Replay** | The engine re-sending its retained history to a new subscriber. |
| **Idempotent** | Doing it twice has the same effect as doing it once. |
| **Maker / taker** | In a match, the order that was already resting in the book is the maker; the one that arrived and matched is the taker. They can pay different fee rates. |
| **bps** | Basis points. 1 bps = 0.01%. 30 bps = 0.30%. |
| **Terminal status** | An order status that is final: `FILLED`, `CANCELLED` or `EXPIRED`. The other statuses (`NEW`, `PARTIALLY_FILLED`, `PENDING_TRIGGER`) are "working". |
| **Batch** | A group of events handled together, to save database round trips. |
| **Advisory lock** | A Postgres lock identified by a number, held by one connection. The consumer uses it so that only one consumer runs. |

## 3. Inputs and outputs

### Environment variables

| Variable | Required | Default | Meaning |
|---|---|---|---|
| `ENGINE_URL` | yes | – | gRPC address of the event source (the router). Same variable the backend uses. |
| `DATABASE_URL` | yes | – | Postgres connection string. |
| `TB_CLUSTER_ID` | yes | – | TigerBeetle cluster id (a number). |
| `TB_IP_ADRESSES` | yes | – | TigerBeetle replica addresses as **IPs** (`ip:port,ip:port`). The spelling `ADRESSES` is the repository's. In Kubernetes the backend's startup script turns `TB_HOSTS` (DNS names) into this; the consumer does not resolve names itself. |
| `BATCH_MAX_ITEMS` | no | `500` | A batch closes when it holds this many events. |
| `BATCH_MAX_WAIT_MS` | no | `50` | A batch closes this many milliseconds after its first event. |
| `QUEUE_DEPTH` | no | `4` | How many batches can wait between two stages (section 7.8). |

A missing required variable, or a number that cannot be parsed, stops the program with a clear
message and exit code 1.

### What it reads

- **The event stream** from `ENGINE_URL` (`SubscribeEvents`, an empty request).
- **Postgres**: `trades` (which trades are already recorded), `orders` (what the backend wrote for
  each order: owner, accounts, limit price, fee rates) and `markets` (ledgers and fee account).

### What it writes

- **TigerBeetle**: transfers (trade legs and refunds), always in batches.
- **Postgres**: rows in `trades`, and the engine-owned columns of `orders`.
- **Standard output / error**: log lines starting with `CONSUMER:`.

### Exit codes

- `0`: stopped on request (SIGTERM or Ctrl-C) after finishing the batches already in progress.
- `1`: something failed that the consumer must not skip (see section 9). Kubernetes restarts it.

## 4. How it behaves

### 4.1 Startup

1. Read the configuration (`Config::from_env`).
2. Connect to Postgres (a pool of up to 10 connections).
3. Take the **single-consumer lock** (a Postgres advisory lock on a dedicated connection). If
   another consumer holds it, exit with an error. This protects against two consumers settling the
   same events at once.
4. Create the TigerBeetle client and the gRPC client for the router (with keepalive pings every 20
   seconds, and a 10-second timeout for the reply).
5. Start the pipeline (section 7.8) and the subscription loop.

### 4.2 Steady state

The consumer subscribes to the event stream and reads events in **batches**. A batch closes when it
has `BATCH_MAX_ITEMS` events, or when `BATCH_MAX_WAIT_MS` has passed since its first event,
whichever comes first. The consumer waits for the first event as long as it takes, so a quiet
stream never produces empty batches.

Each batch passes through three steps, in this order:

1. **Prepare**: ask Postgres what it knows about the events, check everything, and decide which
   TigerBeetle transfers and which Postgres writes the batch needs. (No writes happen here.)
2. **Money**: send the transfers to TigerBeetle in one request.
3. **Commit**: write the trades and order updates to Postgres in one transaction.

These three steps run as a **pipeline**: while batch *n* is being committed to Postgres, batch
*n+1* is already moving money, and batch *n+2* is being prepared. Batches always stay in the order
the events arrived.

### 4.3 Replays (the normal case, not an error)

On every subscription the engine sends its whole retained history first. Most of it is already
recorded. The consumer handles this without any "where did I stop?" memory. Each kind of work is
protected by something that refuses to do it twice:

| Work | What refuses a repeat |
|---|---|
| A trade settlement | Prepare asks Postgres which `trade_id`s are already in `trades` and skips those entirely (it does not even validate them). |
| The money for a trade | The transfer ids are computed from the trade, so a repeat has the same ids. TigerBeetle answers "exists" and moves nothing. |
| A trade row | `trades.trade_id` is the primary key; the insert uses `ON CONFLICT DO NOTHING`. |
| An order update | The `UPDATE` only applies if the order is not already finished and its filled amounts do not go down. An old report matches no row. |
| A refund | The refund's transfer id is computed from the order id; a repeat gets "exists". |

So after a restart, history is re-read quickly, only the really new events change anything, and
nothing is ever applied twice. The same applies mid-run, when the router reconnects to one engine
and that engine's history arrives again in the middle of the stream.

### 4.4 When the stream breaks

If subscribing fails, or the stream ends or errors, the consumer keeps the events it already read
(they form a last, partial batch and are processed), waits, and subscribes again. It never gives up.
The wait starts at 200 ms and doubles each time up to 30 s, with a little randomness (up to half the
wait is subtracted) so that instances do not retry in lockstep. It goes back to 200 ms as soon as a
non-empty batch arrives.

### 4.5 When something is wrong with an event or a system

The consumer **stops** (exit code 1) instead of skipping. Skipping a trade would leave it unsettled
forever; stopping lets an operator fix the cause, and the restart replays the history until it
reaches the same event again. Section 9 lists each case.

### 4.6 Shutdown

On SIGTERM or Ctrl-C the consumer stops reading the stream, **finishes the batches already in the
pipeline**, releases the lock and exits with code 0. A batch that was only half read is dropped; the
next replay brings it back.

## 5. How the code is organised

```
backend/src/bin/consumer.rs     entry point: read config, handle signals, call consumer::run
backend/src/consumer/
  mod.rs        Config, run(), run_locked(), prepare_batch(): wires everything together
  stream.rs     reading the stream: batches (collect_batch), the retry loop (run_source), Backoff
  pipeline.rs   the three-stage pipeline: run_pipeline(), run_stage()
  plan.rs       PURE LOGIC, no I/O: validates events and decides what to do (build_plan)
  store.rs      everything that talks to Postgres
  ledger.rs     everything that talks to TigerBeetle (send)
  ids.rs        computes the deterministic transfer ids
  lock.rs       the single-consumer advisory lock
backend/src/service.rs          EngineClient (shared with the backend), plus with_keepalive()
backend/src/domain.rs           TransferCodeType (a `Refund = 6` code was added for the consumer)
backend/tests/consumer_test.rs  integration tests against real Postgres and TigerBeetle
```

The design rule that makes this readable: **`plan.rs` does no I/O.** It receives the events plus
what Postgres knows, and returns a `BatchPlan` (what to send, what to insert, what to update).
`store.rs` and `ledger.rs` only execute plans. This is why the hardest logic (validation, transfer
building) can be tested with plain unit tests, with no database.

## 6. The flow of the code, call by call

### 6.1 The call tree

```
main()                                         bin/consumer.rs
├─ Config::from_env()                          mod.rs        read the environment
├─ spawn: signal handler → cancel.cancel()     bin/consumer.rs
└─ consumer::run(config, cancel)               mod.rs
   ├─ PgPoolOptions...connect()                              Postgres pool
   ├─ lock::try_lock()                         lock.rs       single-consumer lock
   ├─ run_locked()                             mod.rs
   │  ├─ tb::Client::new()                                    TigerBeetle client
   │  ├─ EngineClient::with_keepalive()        service.rs    gRPC client
   │  ├─ creates the LimitMemory                              newest limit price per open order
   │  ├─ builds three closures: prepare, settle, commit
   │  └─ pipeline::run_pipeline(...)           pipeline.rs   runs until everything ends
   │     ├─ source:   stream::run_source()     stream.rs     subscribe, batch, retry
   │     │     └─ collect_batch()                             one batch
   │     ├─ stage 1:  prepare  → prepare_batch()  mod.rs
   │     │     ├─ plan::wanted_ids()            plan.rs       what to look up
   │     │     ├─ store::known_trade_ids()  ┐
   │     │     ├─ store::load_orders()      ├─ run at the same time
   │     │     ├─ store::load_markets()     ┘   store.rs
   │     │     └─ plan::build_plan()            plan.rs       the decisions
   │     │           ├─ plan_trade()                          one trade settlement
   │     │           │     └─ fee_cap(), trade_leg_transfer_id()
   │     │           └─ plan_report()                         one execution report
   │     │                 └─ refund_transfer_id()
   │     ├─ stage 2:  settle   → ledger::send()   ledger.rs
   │     └─ stage 3:  commit   → store::commit()  store.rs
   └─ lock.release()
```

### 6.2 One batch, step by step

This is what happens to a single batch of events, from the stream to the databases.

```
 router ──events──► run_source ──Vec<EventItem>──► [queue] ──► PREPARE
                    (collect_batch)                              │ wanted_ids → 3 Postgres lookups
                                                                 │ build_plan  → BatchPlan
                                                  [queue] ◄──────┘
                                                     │
                                                     ▼
                                                   MONEY:  ledger::send(plan.transfers)  → TigerBeetle
                                                     │
                                                  [queue]
                                                     ▼
                                                   COMMIT: store::commit(trades, order_updates) → Postgres
```

The value passed between the stages is a `BatchPlan`:

```
BatchPlan {
    transfers:     Vec<tb::Transfer>,   // for TigerBeetle: trade legs and refunds
    trades:        Vec<TradeInsert>,    // for Postgres: new rows in `trades`
    order_updates: Vec<OrderUpdate>,    // for Postgres: new state of orders
}
```

### 6.3 Function by function

**`main` (`bin/consumer.rs`)**
Reads the configuration, creates a `CancellationToken` (a shared "please stop" flag), starts a small
task that cancels the token when SIGTERM or Ctrl-C arrives, and calls `consumer::run`. If `run`
returns an error, `main` returns it, which prints it and exits with code 1.

**`Config::from_env` / `Config::from_lookup` (`mod.rs`)**
`from_lookup` takes a function that looks up a variable by name; `from_env` passes `std::env::var`.
The split exists so tests can supply variables without touching the real environment. Two helpers
live inside: `required(name)` fails if the variable is missing, `number(name, default)` parses a
number or uses the default.

**`run` (`mod.rs`)**
Connects to Postgres, takes the lock, calls `run_locked`, then releases the lock **whether or not**
`run_locked` failed, and returns its result.

**`run_locked` (`mod.rs`)**
Builds the TigerBeetle client and the router client, creates the `LimitMemory` once (section 7.3;
every batch shares it), then defines the three pieces of work as closures and hands them to
`run_pipeline`:

- `prepare`: calls `prepare_batch(&pool, &limits, items)`.
- `settle`: calls `ledger::send(&tb_client, &plan.transfers)` and passes the plan on.
- `commit`: calls `store::commit(...)` and prints one `CONSUMER: batch committed: ...` line.

The comment in the code explains the order: money first, records second.

**`prepare_batch` (`mod.rs`)**
1. `plan::wanted_ids(&items)` lists the trade ids, order ids and market symbols the batch mentions.
2. It runs the three Postgres lookups **at the same time** (`tokio::try_join!`).
3. It locks the shared `LimitMemory` (a plain mutex, held only while planning and never while
   waiting for a database), calls `plan::build_plan(&items, &lookups, &mut memory)` and returns the
   `BatchPlan`.

**`run_source` (`stream.rs`)**
The subscription loop. Repeatedly: connect (`engine.subscribe_events`), read batches with
`collect_batch`, send each non-empty batch to the pipeline's first queue; when the stream is over,
log why, count a reconnect, wait with `Backoff`, and subscribe again. It returns only when
shutdown is requested or the pipeline is gone.

**`collect_batch` (`stream.rs`)**
Reads events from the stream into one batch (section 4.2) and returns one of three results:
`Items` (a normal batch; the stream continues), `Ended` (the stream ended; here are the events read
so far) or `Failed` (the stream broke; same).

**`run_pipeline` and `run_stage` (`pipeline.rs`)**
Create the queues and run the source and the three stages together (section 7.8).

**`wanted_ids` (`plan.rs`)**
Walks the batch once and collects, without duplicates and in order of appearance: trade ids, order
ids (from settlements and reports) and market symbols.

**`build_plan` (`plan.rs`)**
The center of the logic. Walks the events **in arrival order** and, for each, calls `plan_trade` or
`plan_report`. It also receives the `LimitMemory` and hands it to `plan_trade` (to read limits) and
`plan_report` (to update them). At the end it returns the plan, with at most one update per order.
An event with no body is logged and skipped.

**`plan_trade` (`plan.rs`)**
Turns one trade settlement into transfers and a trade row, after validating it (section 7.1).

**`plan_report` (`plan.rs`)**
Turns one execution report into an order update and, if the order ended, a refund (section 7.2).

**`known_trade_ids`, `load_orders`, `load_markets` (`store.rs`)**
Three read queries, each using `= ANY($1)` to fetch many rows at once. `load_orders` reads the
backend-owned columns fresh for every batch, because the backend may change them while the
consumer runs.

**`ledger::send` (`ledger.rs`)**
Sends all transfers in one TigerBeetle request and judges the result chain by chain (section 7.5).

**`store::commit` (`store.rs`)**
One Postgres transaction: insert the trades, then update the orders (section 7.6). Returns
`CommitStats` (how many trades were inserted, orders updated, orders skipped).

**`try_lock` / `LockGuard::release` (`lock.rs`)**
Opens a dedicated connection and runs `SELECT pg_try_advisory_lock(key)`. If another connection
holds that key, it returns an error. Otherwise it returns a guard that owns the connection;
`release` closes it, which frees the lock.

## 7. Deep dives

### 7.1 Planning a trade settlement (`plan_trade`)

Input: the event, the trade settlement inside it, the lookups from Postgres, the memory of
the newest limit prices (section 7.3) and the set of trades already planned in this batch.

**Step 1, skip what is already done.** If the `trade_id` is in `known_trades` (already in Postgres)
or was already planned earlier in this batch, return without doing anything and without
validating. The orders of an old trade may have ended long ago, so validating it could fail for no
good reason.

**Step 2, read and check the message.** The market and both orders must exist in the lookups
(`UnknownMarket` / `UnknownOrder` otherwise). Every required field must be present; an absent field
is an error, never zero (`Malformed`). `price`, `base` and `quote` must be above zero, because
Postgres refuses a trade where any of them is zero (a `CHECK` on the `trades` table) and that would
only be discovered after the money moved.

**Step 3, validate against what the backend wrote itself.** The consumer trusts its own `orders`
rows, not the event, for anything that affects money:

| Check | Fails with |
|---|---|
| Both orders belong to the event's market | `Validation` |
| The account holder in the event is the order's user, for both sides | `Validation` |
| The four accounts in the event are the order's own base and quote accounts | `Validation` |
| The fee account is the market's fee account | `Validation` |
| The price respects each order's limit price (buyer pays at most their limit, seller gets at least theirs). Orders without a limit price skip this. | `Validation` |
| Each fee is within the cap of the order's fee rate (see below) | `Validation` |

**The fee cap.** The taker pays the taker rate and the maker pays the maker rate (`taker_side` in
the event says which side took). The cap is `ceil(quote_quantity × bps / 10000)`, computed without
overflow (`fee_cap`): `(quote / 10000) × bps` plus `ceil((quote % 10000) × bps / 10000)`. A fee
above the cap is rejected. A fee equal to or below it is accepted.

**Step 4, build the transfers.** A trade is **one linked chain** of up to four transfers, in this
order:

| Leg | Moves | From (debit) | To (credit) | Ledger | Code |
|---|---|---|---|---|---|
| 1 | `base_quantity` | seller's hold account | buyer's base account | base | `Trade` |
| 2 | `quote_quantity` | buyer's hold account | seller's quote account | quote | `Trade` |
| 3 | `buyer_fee` | buyer's hold account | fee account | quote | `FeeCollection` |
| 4 | `seller_fee` | seller's quote account | fee account | quote | `FeeCollection` |

- A leg with a fee of 0 is left out, but the others keep their numbers.
- All legs except the last carry the `Linked` flag, so TigerBeetle treats them as a unit.
- The order matters: leg 2 credits the seller's quote account, which leg 4 then debits for the fee.
- Each transfer's id is `trade_leg_transfer_id(trade_id, leg)` (section 7.4).

**Step 5, the Postgres row.** A `TradeInsert` with the trade's data and the event's `engine_time`.

### 7.2 Planning an execution report (`plan_report`)

An execution report says "the order is now in this state". It always contains the complete state,
so the consumer **overwrites** the order's values; it never adds to them.

1. **Look up the order.** If the order is not in Postgres, that is an error (`UnknownOrder`), never
   a skip. The backend inserts the order row before the order is sent to the engine, so a report
   for a missing row means something is wrong.
2. **Convert the fields.** An unknown or unspecified status is `Malformed`. A `cancel_reason` of
   0 means "none".
3. **Keep one update per order per batch.** A batch can hold several reports for the same order
   (for example `NEW`, then `PARTIALLY_FILLED`, then `FILLED`), and the SQL update cannot apply
   several rows to one target row reliably. So the plan keeps **one**: the report with the most
   progress, ranked by *(is it terminal?, filled base, filled quote)*. On an exact tie, the later
   report wins. This also protects against a replay that arrives after live events in the same
   batch: the older report ranks lower and loses.
4. **Refund when the order ended.** If the status is terminal (and no refund for this order was
   already planned in this batch), add one refund transfer:
   - It debits the order's hold account (its id is the order id).
   - It credits the user's account of the **reserved asset**: the quote account for a buy, the base
     account for a sell, on that asset's ledger.
   - It uses the `BalancingDebit` flag with the amount `u128::MAX` ("a very large amount").
     TigerBeetle then moves **exactly what is left** in the hold account, which also returns any
     fee over-reservation. If nothing is left, it moves 0 and succeeds.
   - Its id is `refund_transfer_id(order_id)`, so a repeat is recognised.
   - It is not linked to anything.

A modify that **lowers** an order's reservation does not return any money at that moment. The extra
stays in the hold account and comes back with this refund when the order ends. That is a decision,
not a gap (section 12).

### 7.3 Limit prices that change (`LimitMemory`)

**The problem.** For every trade the consumer checks that the buyer does not pay more than their
limit price (and the seller does not get less than theirs). An order can be modified so that its
limit changes: say a buy limit goes from 1000 to 1300 and the order matches at 1200. The engine
sends a `REPLACED` report ("the limit is now 1300") and then the trade at 1200. The consumer reads
limits from Postgres, but Postgres only learns 1300 when the batch holding the `REPLACED` report is
**saved**, and the next batch is **prepared** before that happens (the stages overlap, section
7.8). Without help, the consumer would compare 1200 with the old 1000, call the trade invalid, and
stop.

**The solution.** The consumer keeps a small notebook in memory, the `LimitMemory`, that lives as
long as the program: *the newest limit price seen for each open order*.

- It is created once in `run_locked` and shared by all batches (behind a mutex that is only held
  while planning, never while waiting for a database).
- `plan_report` writes to it when it **plans** a report, not when the batch is saved. The next
  batch therefore already knows the new limit. Batches are prepared one at a time and in order, so
  the notebook is always up to date for the batch being prepared.
- `plan_trade` asks the notebook first (`limit_of`) and falls back to the limit stored in Postgres
  when the order is not in the notebook.
- **A replay cannot bring an old limit back.** Each entry remembers the *progress* of the report it
  came from, the sort key *(finished?, filled base, filled quote)*. A report only replaces the
  entry if its progress is **equal or bigger**, so an older report arriving again is ignored. This
  is the same rule used to choose between two reports of one order inside a batch (section 7.2).
- **It does not grow forever.** When a report says the order is over (terminal), the order is
  removed from the notebook. Only open orders that were modified stay in it.
- **A restart empties it.** That is correct: Postgres has everything that was saved, and the
  replay delivers the `REPLACED` report again before the trade that depends on it, so the notebook
  fills itself again in the right order.

Tested by unit tests (the repricing in an earlier batch, an older report in a later batch, a
finished order being forgotten) and by an end-to-end test that makes every event its own batch.

### 7.4 Transfer ids (`ids.rs`)

TigerBeetle recognises a repeat by the transfer id, so the ids must be the same every time the same
event is processed. They are computed, not random:

```
id = ( high 64 bits of the source id ) | ( first 8 bytes of SHA-256(tag, source id, leg) )
```

- `trade_leg_transfer_id(trade_id, leg)` uses the tag `"trade-leg"`.
- `refund_transfer_id(order_id)` uses the tag `"refund"` and leg 0.
- Keeping the **high 64 bits** of the source id keeps the time-ordered prefix that `tb::id()` and
  UUIDv7 ids have, which TigerBeetle likes.
- The **low 64 bits** are a hash. Simply adding the leg number to the trade id would be unsafe:
  ids created within the same millisecond are consecutive, so "leg 2 of trade A" could equal
  "leg 1 of the next trade".
- Different tags guarantee a refund id never equals a trade-leg id.
- `0` and `u128::MAX` are not valid TigerBeetle ids, so those results are replaced by `1`.

**This formula can never change once deployed**, because old transfers were created with the old
ids.

### 7.5 Sending money (`ledger::send`)

All transfers of a batch go to TigerBeetle in **one request**, with a 5-second timeout. TigerBeetle
answers with a list of **only the transfers that did not succeed**. An empty list means everything
was applied.

The subtle part is a replay. When the transfers of a trade already exist, TigerBeetle reports the
chain like this: the first transfer gets `Exists`, and the other transfers of the chain get
`LinkedEventFailed` (because, inside a chain, an `Exists` counts as a failure that cancels the rest
of the chain). So looking at transfers one by one would wrongly treat every replayed trade as an
error. Instead `send`:

1. Splits the transfers into chains: a chain continues while the current transfer has the `Linked`
   flag, and ends at the first one without it.
2. For each chain, finds its **root cause**: the first reported result that is not
   `LinkedEventFailed`.
3. No reported result, or a root cause of `Exists`: the chain is done (now or before). Success.
4. Any other root cause: return `LedgerError::Rejected` naming the transfer that caused it (not its
   chain mates). Examples: not enough funds, account not found, a repeated id with different
   contents.

A new chain and a replayed chain in the same request are judged independently. A failed network
call or a timeout is `LedgerError::Transport`.

### 7.6 Saving to Postgres (`store::commit`)

One transaction. If any statement fails, nothing is saved, including the trades.

**Trades.** One `INSERT ... SELECT ... FROM unnest(...) ... ON CONFLICT (trade_id) DO NOTHING`. A
trade that already exists is silently ignored (and not counted in `trades_inserted`), so a known
trade next to new ones does not abort the batch.

**Order updates.** One `UPDATE orders o SET ... FROM unnest(...) AS r(...) WHERE ...`:

```sql
WHERE o.order_id = r.order_id
  AND (o.status IS NULL OR o.status NOT IN (3, 4, 5))   -- not already finished
  AND o.filled_base_quantity  <= r.fb::numeric          -- filled amounts never go down
  AND o.filled_quote_quantity <= r.fq::numeric
```

- A row that does not match is an **old report** and is skipped (counted in `orders_skipped`).
- `status IS NULL` is written explicitly. A brand-new order has no status yet, and in SQL
  `NULL NOT IN (3, 4, 5)` is not true, so without it the first report of every order would be
  rejected.
- Equal filled amounts are allowed, because several reports (`NEW`, a trigger firing, `REPLACED`)
  do not change the filled amount.
- `limit_price` is overwritten only when the report carries one (`COALESCE`).

**Why quantities travel as text.** Amounts are `u128` and the columns are `NUMERIC(39, 0)`, which
the `rust_decimal` type cannot hold (it stops at 96 bits). So every quantity is sent as a string
and converted with `::numeric` in SQL.

### 7.7 The stream, batches and retries (`stream.rs`)

- **`collect_batch`** waits for the first event with no time limit, then starts a deadline
  (`max_wait`) and keeps reading until the batch is full or the deadline passes. It uses
  `tokio::select!` to race "next event" against "deadline". Reading the next event is safe to
  abandon and resume, so no event is lost when the deadline wins.
- **`EventSource`** is a tiny trait with one method, `next()`. The real stream
  (`tonic::Streaming`) implements it, and so does a fake in the tests.
- **`Backoff`** is the waiting schedule: `next_delay(jitter)` returns the current wait and doubles
  it up to the maximum; `reset()` goes back to the minimum. The caller passes the jitter (a number
  from 0 to 1) so tests can be exact; production uses the clock's nanoseconds.
- **`run_source`** ties it together (section 6.3). It also counts reconnects in `Stats.reconnects`
  (currently only counted, never read).

### 7.8 The pipeline (`pipeline.rs`)

```
source ──queue──► prepare ──queue──► ledger (money) ──queue──► commit
```

- Each arrow is a **bounded** queue (`tokio::sync::mpsc`) holding at most `QUEUE_DEPTH` batches.
- Each stage handles **one batch at a time**, so batches stay in order.
- When a queue is full, the stage before it waits. That wait travels back to the source, which
  stops reading the gRPC stream, and HTTP/2 flow control then slows the router. A slow stage slows
  the stream; it does not make memory grow.

**Normal shutdown.** When the source ends (shutdown requested), it drops its sender. The prepare
stage sees an empty, closed queue after finishing what is in it, and ends, which closes the next
queue, and so on. Everything already queued is processed.

**Failure.** If a stage's work returns an error, that stage logs `the <name> stage failed: ...`,
**cancels the shared token** (which stops the source) and returns the error. Then:

- the stages **before** it find their output closed and stop quietly;
- the stages **after** it still finish what was already handed to them.

The last rule matters for money: if a batch passed the money stage and the commit stage had not
saved it yet, it still gets saved, so no moved money is left without its record. `run_pipeline`
returns the first error in the order prepare, ledger, commit.

Nothing in the stage code needs special "stop" messages: closing a queue is the signal.

### 7.9 The single-consumer lock (`lock.rs`)

`try_lock(database_url, key)` opens a **dedicated** Postgres connection and asks
`pg_try_advisory_lock(key)`. The key is a fixed number (`DEFAULT_LOCK_KEY`, the bytes of the text
`"tradecon"`). If it is already held, the second consumer exits with
"another consumer holds the advisory lock". The lock belongs to the connection, so it disappears
automatically if the consumer dies. Kubernetes should also run exactly one replica with the
`Recreate` strategy; the lock is the second guard.

## 8. Data: who writes what

### Table `orders`

| Columns | Written by | Notes |
|---|---|---|
| `order_id`, `idempotency_key`, `command_id`, `user_id`, `market_id`, `base_account_id`, `quote_account_id`, `side`, `order_type`, `time_in_force`, quantities, `trigger_*`, `protection_price`, `expire_time`, `maker_fee_bps`, `taker_fee_bps`, `reserved_amount`, `submission`, `reject_*` | **Backend** | The consumer only reads some of them, fresh for every batch. |
| `limit_price` | Backend at placement, **consumer** afterwards | Overwritten only by reports that carry one. |
| `status`, `cancel_reason`, `filled_base_quantity`, `filled_quote_quantity` | **Consumer only** | `status` is `NULL` until the first report. Status values: 1 `NEW`, 2 `PARTIALLY_FILLED`, 3 `FILLED`, 4 `CANCELLED`, 5 `EXPIRED`, 6 `PENDING_TRIGGER`. |
| `created_at`, `updated_at` | Database | `updated_at` is set by a trigger when a row really changes. |

### Table `trades`

Written **only by the consumer**. Primary key `trade_id`. Columns: `market_id`, `buyer_order_id`,
`seller_order_id`, `price`, `base_quantity`, `quote_quantity`, `buyer_fee`, `seller_fee`,
`taker_side` (1 buy, 2 sell), `engine_time`, `created_at`. The database refuses a zero price or
quantity.

### Table `markets` (read only)

`symbol`, `market_id`, `base_ledger_id`, `quote_ledger_id`, `fee_account_id`.

### TigerBeetle

- **Hold accounts** (id = order id) are created and funded by the backend, not the consumer.
- The consumer creates **transfers** with these `code`s (`TransferCodeType`): `Trade` = 1,
  `FeeCollection` = 2, `Refund` = 6 (added for the consumer; `transfer_kind` in `v1.rs` shows it as
  `"refund"`).

## 9. What happens when something goes wrong

| Situation | Where it is detected | Result |
|---|---|---|
| Router unreachable, or subscribe refused | `run_source` | Logs, waits (200 ms doubling to 30 s), subscribes again. Never exits. |
| Stream ends or breaks | `collect_batch` | The events read so far are processed; then the same retry as above. |
| Router sends an event with no body | `build_plan` | Logged, skipped. |
| A required field is absent, or a status/side is unknown | `plan_*` | `Malformed`: the consumer stops (exit 1). |
| Trade fails validation (wrong account, price outside limit, fee too high, wrong fee account, zero amount) | `plan_trade` | `Validation`: the consumer stops. Nothing of that batch reaches TigerBeetle or Postgres. |
| Event refers to an order or market missing from Postgres | `plan_*` | `UnknownOrder` / `UnknownMarket`: the consumer stops. |
| TigerBeetle unreachable or no answer within 5 s | `ledger::send` | `Transport`: the consumer stops. |
| TigerBeetle rejects a transfer (no funds, no such account, a repeated id with different data) | `ledger::send` | `Rejected`: the consumer stops. |
| Postgres error, or the batch transaction fails | `store::commit` | The error stops the consumer; the transaction is rolled back, so no partial batch is saved. |
| Another consumer is already running | `try_lock` | Exits with an error. |
| Missing environment variable | `Config` | Exits with a clear message. |
| Postgres address wrong | `run` | Exits with `PoolTimedOut` after sqlx's 30-second wait. |
| SIGTERM / Ctrl-C | `main` | Clean stop, exit 0 (section 4.6). |

**Why stop instead of skip.** An event that cannot be settled must not be skipped, or its money
would never move. After a stop, Kubernetes restarts the process, the replay brings it back to the
same event, and it fails again until the cause is fixed. That repetition is deliberate: it is the
alert.

**Crash safety.** The money step comes before the commit step. If the process dies after TigerBeetle
succeeded and before Postgres committed, the next replay sends the same transfers again
(TigerBeetle answers "exists", nothing moves twice) and then commits. If the order were reversed,
a crash would leave a recorded trade whose money never moved. Keep this order.

## 10. Reading the short variable names

The code uses short names in small scopes. This section decodes them. When a name is not listed, it
is a plain word.

### Used across several files

| Name | Meaning |
|---|---|
| `tb` | The `cn_tigerbeetle` crate (TigerBeetle client). `tb::Transfer`, `tb::Client`. |
| `pg` | Postgres. |
| `cfg` | The `Config`. |
| `e` | An error (in `.map_err(|e| ...)` and `Err(e) => ...`). |
| `v`, `k` | A value and its key (in `Config::from_lookup`: `k` is the variable name, `v` its text). |
| `m` | A message string (an error text). |
| `f` | The formatter in `fmt(&self, f)`; it is how errors print themselves. |
| `i` | A loop index. |
| `tx` / `rx` | In `pipeline.rs`: the **s**ender and **r**eceiver ends of a queue (`events_tx`, `events_rx`). In `store.rs`, `tx` is a database **t**ransaction. |
| `bps` | Basis points of a fee rate (1 bps = 0.01%). |
| `fb`, `fq` | **F**illed **b**ase, **f**illed **q**uote (SQL, `store.rs`). |
| `lp` | **L**imit **p**rice (SQL). |

### `plan.rs`

| Name | Where | Meaning |
|---|---|---|
| `l` | `plan_trade`, `plan_report` | The `Lookups`: orders, markets and known trades read from Postgres. |
| `ts` | `plan_trade`, `wanted_ids` | The **t**rade **s**ettlement message inside the event. |
| `r` | `plan_report`, `wanted_ids` | The execution **r**eport message inside the event. |
| `item` | most functions | One whole `EventItem` from the stream. |
| `state` | `build_plan`, `plan_report` | The `ReportState` that the reports of this batch fill in. |
| `plan` | most functions | The `BatchPlan` being built. |
| `seen`, `seen_trades` | `plan_trade`, `build_plan` | Trade ids already planned in this batch. |
| `seen_t`, `seen_o`, `seen_s` | `wanted_ids` | Sets that prevent duplicates in the lists of **t**rades, **o**rders and **s**ymbols. |
| `what` | `id_of`, `uuid_of`, `amount_of` | The name of the field, used only to build the error message. |
| `v` | `id_of`, `push_unique` | The value (an optional id or amount from the message; the value to add). |
| `buyer`, `seller` | `plan_trade` | The `OrderRow` of each side, from Postgres. |
| `buyer_id`, `seller_id` | `plan_trade` | The order ids, as UUIDs. |
| `buyer_hold`, `seller_hold` | `plan_trade` | The hold accounts, as numbers. Their id equals the order id. |
| `buyer_base`, `buyer_quote`, `seller_base`, `seller_quote` | `plan_trade` | The four user accounts, as numbers (taken from the event, checked against Postgres). |
| `base`, `quote` | `plan_trade` | The traded quantity of the base asset and its value in the quote asset. |
| `taker` | `plan_trade` | Which side was the taker (`Side::Buy` or `Side::Sell`). |
| `t` | in `engine_time` | The protobuf `Timestamp` being converted. |
| `fail` | `plan_trade` | A small closure that builds a `Validation` error with the trade id in the text. |
| `limit_of` | `plan_trade` | A closure: "the limit price to use for this order", preferring the newest one in the `LimitMemory`. |
| `bps` (closure) | `plan_trade` | A closure: "the fee rate for this order", taker rate if it was the taker, otherwise maker rate. |
| `legs` | `plan_trade` | A list of tuples `(leg number, debit account, credit account, amount, ledger, code)`. |
| `last` | `plan_trade` | Index of the last leg; all earlier legs get the `Linked` flag. |
| `whole`, `rest` | `fee_cap` | `quote / 10000 × bps`, and the remainder part `(quote % 10000) × bps`. |
| `u` | `rank` closure | An `OrderUpdate`. |
| `rank` | `plan_report` | A closure: the sort key `(terminal?, filled base, filled quote)` used to choose between two reports of one order. |
| `old`, `old_terminal` | `plan_report` | The update already kept for this order, and whether it is terminal. |
| `s` | `TERMINAL.iter().any(|s| ...)` | One status from the list of terminal statuses. |
| `v` (in `cancel_reason`) | `plan_report` | The raw cancel-reason number from the message. |
| `credit`, `ledger` | `plan_report` | The user account that receives the refund, and its ledger. |
| `o`, `s` (in `Display`) | `PlanError::fmt` | The order id and the symbol inside the error variants. |
| `PlanError::Malformed` | – | The event is broken (missing field, unknown value). |
| `PlanError::Validation` | – | The event is well-formed but contradicts our own data. |

`ReportState` fields: `refunded` (orders whose refund is already planned), `updates` (the kept
update per order), `update_order` (order ids in order of first appearance, to keep a stable output
order).

`LimitMemory` (section 7.3): `newest` maps an order id to *(progress, limit price)*; `get`, `see` and
`forget` read, write and delete an entry; `Progress` is the sort key *(finished?, filled base,
filled quote)*; `memory` is the name of the `LimitMemory` passed to `build_plan`, `plan_trade` and
`plan_report`.

### `store.rs`

| Name | Meaning |
|---|---|
| `ids` | The list of ids to look up (`= ANY($1)`). |
| `rows` | What Postgres returned. |
| `OrderTuple` | The columns of the `orders` query in order: order id, user id, market id, base account, quote account, side, limit price (as text), maker bps, taker bps. |
| `side == 1` | In the database `side` 1 is a buy, 2 a sell. |
| `stats` | The `CommitStats` counters. |
| `micros` | A closure turning an `engine_time` into microseconds since 1970, which SQL turns back into a timestamp. |
| `t`, `u`, `p` | A trade (`TradeInsert`), an update (`OrderUpdate`), a price. |
| `$1`, `$2`, ... | Query parameters. Each is a whole **array** (one entry per trade or order). |
| `unnest(...) AS r(...)` | Turns the arrays into a temporary table. `r` is **one row of that table**. |
| `o` | The existing row of the `orders` table being updated. |
| `r.buyer`, `r.seller`, `r.price`, `r.base`, `r.quote` | Trade columns: buyer order id, seller order id, price, base and quote quantity. |
| `r.bfee`, `r.sfee`, `r.taker`, `r.us` | Buyer fee, seller fee, taker side, `engine_time` in microseconds. |
| `r.fb`, `r.fq`, `r.lp` | New filled base, filled quote and limit price for an order. |

### `ledger.rs`

| Name | Meaning |
|---|---|
| `results` | What TigerBeetle reported (only the transfers that did not come out `Ok`). |
| `r` | One reported item: `index` (position in the request) and `result`. |
| `reported` | The same, as a map from position to result. |
| `start`, `end` | The positions where the current chain begins and ends. |
| `chain` | The reported results inside the current chain. |
| `root` | The result that explains the chain's outcome (section 7.5). |

### `stream.rs`

| Name | Meaning |
|---|---|
| `min`, `max`, `current` | `Backoff`'s shortest wait, longest wait, and the next wait. |
| `jitter` | A number from 0 to 1 that shortens a wait by up to half. |
| `source` | The thing events are read from (the real stream, or a fake in tests). |
| `S` | The type of the source. |
| `read` / `r` | The result of `collect_batch` (`Items`, `Ended`, `Failed`). |
| `c` | The result of connecting (`connected`). |
| `b` | A batch. |
| `stop` / `why` | Set when the stream is over; the text says why. |
| `deadline` | The moment the current batch must close. |
| `out` | The sender for the pipeline's first queue. |
| `settings` | `SourceSettings`: batch limits and backoff limits. |

### `pipeline.rs` (generic type letters)

| Name | Meaning |
|---|---|
| `I`, `O` | The **i**nput and **o**utput type of a stage (`Vec<EventItem>` in, `BatchPlan` out). |
| `F`, `Fut` | The stage's work function and the future it returns. |
| `Src`/`SrcFut`, `P`/`PFut`, `L`/`LFut`, `C`/`CFut` | The source, **p**repare, **l**edger and **c**ommit functions and their futures. |
| `depth` | Queue size (at least 1). |
| `prepared`, `settled`, `committed` | The final result of each stage. |

### `ids.rs`

`tag` (a byte string that names the kind of id: `trade-leg` or `refund`), `source` (the id it
derives from: a trade id or an order id), `leg` (the leg number), `digest` (the SHA-256 output),
`low` (the 8 bytes of it that are used).

## 11. Tests

All tests run against the same code as production.

- **Unit tests** (98, no database needed), in each module:
  - `plan.rs`: validation of every rule, the four-leg chain and zero-fee chains, the refund (buyer
    vs seller), collapsing several reports, limit changes inside a batch and across batches (`LimitMemory`), replay order.
  - `ids.rs`: distinct, deterministic ids; no collision between consecutive trades; refund vs leg.
  - `stream.rs` and `pipeline.rs`: batch size and wait, resubscribing, backoff, stage failure and
    draining. They use a paused clock, so they take no real time.
  - `mod.rs`: configuration.
- **Integration tests** (26, `backend/tests/consumer_test.rs`), against a real Postgres and a real
  TigerBeetle:
  - `store_*`: lookups, one-transaction commit, the `NULL` status case, the order-update rule, very
    large numbers.
  - `ledger_*`: replayed chains, failed legs named correctly, balancing refunds (repeatable, and of
    an empty hold account).
  - `lock_*`: the single-consumer lock.
  - `e2e_*`: the real consumer connected to a **fake router** (a small gRPC server that plays the
    router's part and sends scripted events): a trade settles and the balances are correct; the
    same events delivered twice change nothing; a stream that drops halfway and then replays ends
    consistent; a missing order and an invalid trade stop the consumer without moving money; a
    second consumer cannot start; a limit raised by an edit is still honored when the edit and its
    trade are in different batches (one event per batch).
- `backend/scripts/integration-test.sh` starts a whole stack in Docker (Postgres, TigerBeetle, the
  engine, the backend) and runs the existing integration tests plus these. It is for running
  everything on a laptop; the GitHub workflows no longer use it (`ci.yaml` has the job commented
  out, and the Kubernetes tests replaced it).

**What runs in the GitHub workflows (`ci.yaml`, `backend_ci.yaml`):** the build, clippy with
`-D warnings`, and `cargo test --lib --bins`, which runs all the **unit tests** of the consumer
(they need no database). The integration tests in `backend/tests/consumer_test.rs` are **not** run
by the workflows: they need Postgres and TigerBeetle, and the Kubernetes job only runs the existing
`integration_test`, because there is no engine or router to deploy yet.

**What is not tested:** a real router and a real engine stream (neither exists yet; section 12).

## 12. Limits, open decisions, and what is not built

**Not built yet**
- The router, and the engine's `PlaceOrder` and `SubscribeEvents` (still `unimplemented`). The
  consumer has only run against a fake router that follows `proto/events.proto`.
- Deployment: no Dockerfile target, Kubernetes manifest or CI job for the consumer. It needs
  `ENGINE_URL` as a plain environment value, the existing `backend-secrets` for `DATABASE_URL` and
  `TB_CLUSTER_ID`, and the same startup step the backend uses to resolve `TB_HOSTS` into
  `TB_IP_ADRESSES`.
- Health probes, metrics, and alerts. The "alert" is the log line and the non-zero exit.

**Decisions taken**
- **Money from a smaller order comes back when the order ends.** When an order is edited to a
  smaller size, the extra money stays in the order's hold account until the order is filled,
  cancelled or expired. Then the refund (section 7.2) returns everything that is left. The contract
  asks for the extra to be returned right after the edit; the team accepted the delay. The
  consequence is that the user cannot use that money for other orders in the meantime. `REPLACED`
  reports still update the order row. (If this is ever reversed: the amount to return must be the
  same on every replay, so it cannot be calculated from `orders.reserved_amount`, which changes
  after the first time.)

**Known limits**
- **Equal filled amounts.** Two reports with the same filled amount (for example `NEW` and
  `REPLACED`) cannot be ordered by the update rule. A replay may briefly show the older price or
  status until it reaches the latest report. The final state is correct. A crash in the middle
  leaves the older value until the next replay.
- **Throughput.** Batches are processed one stage at a time, and the whole history is re-read on
  every restart. How long that takes depends on the engines' retention window, which is not decided.
- **Downtime.** If the consumer is down longer than the engines' retention window, events in the gap
  are never replayed and their trades never settle. Nothing detects it. Alert on consumer downtime.

**Smaller, deferred items**
- `BATCH_MAX_ITEMS` has no upper bound. TigerBeetle accepts a limited number of events per request
  (about 8,189 in a production configuration, fewer in the `--development` mode used by the tests)
  and a trade can produce 4 transfers, so a very large value can make a big batch fail on every
  restart.
- The ledger stage may settle one more batch after the commit stage has failed (money stays
  correct; the replay answers "exists").
- If the lock's connection dies, nobody notices.
- Subscribing has no timeout of its own. Keepalive detects dead connections, but a router that
  accepts the call and never answers would block it.
- The sides are not checked (that the "buyer" order really is a buy), and a refund takes its ledger
  from the event's market rather than from the order's.
- An unknown `cancel_reason` number is stored as is.
- `Stats.reconnects` is counted but never read.

**Decisions that are fixed once deployed**
- The transfer id formula (section 7.4).
- The `Refund = 6` transfer code.
- The fee cap rule (`ceil(quote × bps / 10000)`), which is one unit more lenient than flooring.

## 13. Running it

### As a program

```
ENGINE_URL=http://router:50051 \
DATABASE_URL=postgres://user:password@host:5432/db \
TB_CLUSTER_ID=0 \
TB_IP_ADRESSES=10.0.0.1:3000,10.0.0.2:3000,10.0.0.3:3000 \
cargo run --bin consumer
```

`cargo run` without `--bin` starts the backend, because `Cargo.toml` has `default-run = "backend"`.

### Log lines

| Line | Meaning |
|---|---|
| `CONSUMER: could not subscribe to the event stream: ...` | The subscription failed; it will retry. |
| `CONSUMER: the event stream ended` / `broke: ...` | The stream is over; it will resubscribe. |
| `CONSUMER: batch committed: CommitStats { trades_inserted, orders_updated, orders_skipped }` | One batch saved. `orders_skipped` counts old reports; large numbers right after a restart are normal. |
| `CONSUMER: the <prepare/ledger/commit> stage failed: ...` | Fatal. The next line of the process is its exit. |
| `CONSUMER: stopping` | A shutdown signal arrived. |

### Tests

```
# a throwaway Postgres and TigerBeetle (same images as the integration script)
docker run -d --name pg -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=backend -p 5433:5432 postgres:16
docker run -d --name tb --security-opt seccomp=unconfined --cap-add IPC_LOCK -p 3000:3000 \
  --entrypoint /bin/sh ghcr.io/tigerbeetle/tigerbeetle:0.16.67 -c \
  'FILE=/tmp/0.tigerbeetle; /tigerbeetle format --cluster=0 --replica=0 --replica-count=1 --development "$FILE"; exec /tigerbeetle start --addresses=0.0.0.0:3000 --development "$FILE"'

cd backend
TB_CLUSTER_ID=0 TB_IP_ADRESSES=127.0.0.1:3000 \
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5433/backend \
  cargo test --lib --test consumer_test
```

Or run the whole stack locally with Docker: `bash backend/scripts/integration-test.sh` (not part of
the workflows anymore).

## 14. A complete example with numbers

Setup. Market `BTC-USD`. Base asset BTC, quote asset USD. Prices and amounts are whole smallest
units, to keep the numbers small.

- Seller order **S**: sells 1,000 base at limit 1,000, fee rates maker 10 bps, taker 30 bps. Its
  hold account holds 1,000 base.
- Buyer order **B**: buys 1,000 base at limit 1,000, same fee rates. Its hold account holds
  1,010,000 quote (the 1,000,000 it may spend plus room for the fee).
- S was resting in the book (maker); B arrived and matched (taker).

**The events** (in this order, in one batch):

1. `TradeSettlement`: price 1,000, base 1,000, quote 1,000,000, buyer fee 3,000, seller fee 1,000,
   taker side = buy.
2. `ExecutionReport` for B: status `FILLED`, filled base 1,000, filled quote 1,000,000.
3. `ExecutionReport` for S: status `FILLED`, filled base 1,000, filled quote 1,000,000.

**Prepare.**
- The trade is not in Postgres yet, so it is processed.
- Checks: price 1,000 is not above B's limit (1,000) and not below S's limit (1,000). The buyer is the
  taker, so its cap is `ceil(1,000,000 × 30 / 10,000)` = 3,000; its fee 3,000 is allowed. The seller is
  the maker: cap `ceil(1,000,000 × 10 / 10,000)` = 1,000; its fee 1,000 is allowed.
- Transfers: leg 1 moves 1,000 base from S's hold to B's base account; leg 2 moves 1,000,000 quote
  from B's hold to S's quote account; leg 3 moves 3,000 quote from B's hold to the fee account; leg
  4 moves 1,000 quote from S's quote account to the fee account.
- Both reports are terminal, so two refunds are added: B's hold → B's quote account, and S's hold →
  S's base account, each "everything that is left".
- Updates: B and S both to `FILLED` with the filled amounts above.

**Money.** All seven transfers go to TigerBeetle in one request. Afterwards:

| Account | Result |
|---|---|
| S's hold (base) | 1,000 in, 1,000 out → empty; the refund moves 0 |
| B's base account | +1,000 |
| B's hold (quote) | 1,010,000 in; out 1,000,000 + 3,000 = 1,003,000; the refund returns the remaining **7,000** to B's quote account |
| S's quote account | +1,000,000 − 1,000 fee = **999,000** net |
| Fee account | +4,000 (3,000 + 1,000) |

**Commit.** One transaction inserts the trade and sets B and S to status 3 with the filled amounts.

**Replay.** If the same three events arrive again (after a restart, say):
- Prepare finds the trade in Postgres and skips it. The reports are still processed.
- The two refunds are sent again with the same ids; TigerBeetle answers "exists".
- The order updates match no row (the orders are already `FILLED`), so `orders_skipped` is 2.
- Balances and rows are exactly as before.
