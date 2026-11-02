# Event Consumer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **The user said: do not commit or push.** Ignore any "commit" habit; no task below has a commit step.

**Goal:** A second binary, `consumer`, that subscribes to the router's `SubscribeEvents` stream and settles trades in TigerBeetle and Postgres, safely and quickly, however many times events are replayed.

**Architecture:** One crate, two binaries. All logic is in the `backend` library under `src/consumer/`; `src/bin/consumer.rs` only reads env vars and calls it. There is no cache: every replayed event is re-attempted and rejected by Postgres or TigerBeetle. Work runs as a three-stage pipeline (prepare → ledger → commit) connected by bounded `tokio::sync::mpsc` channels.

**Tech Stack:** Rust 2024, tokio, tonic 0.14 (`proto` crate), sqlx 0.9 (Postgres), `cn-tigerbeetle` 0.1.2, `tokio-util` (`CancellationToken`), `sha2`.

**Spec:** `proto/how_to_use.md` ("Processing the event stream", "Settling a trade", "Refunding reservations"), `proto/devops.md` ("Event consumer", "Router"), `proto/events.proto`. The design decisions made in the design discussion are written into this plan; where they add to the docs they are listed under *Decisions to confirm*.

## Global Constraints

- Integers only; never floats (`how_to_use.md` Arithmetic). Quantities are `u128`; bind them to Postgres as **text cast to `numeric`**, never `rust_decimal::Decimal` (it caps at 96 bits; the columns are `NUMERIC(39,0)`).
- Never `unwrap()`/`panic!` on message contents or enum conversions; every conversion has a fallback arm (`how_to_use.md` Decoding messages). An absent `Uint128`/`Id` is not zero. A `body` of `None` is logged at error level and skipped.
- Do not log account IDs, user IDs or full orders at `info` level.
- Transfer legs for one trade form one linked chain, in this order: (1) `base_quantity` seller hold → buyer base account; (2) `quote_quantity` buyer hold → seller quote account; (3) `buyer_fee` buyer hold → fee account; (4) `seller_fee` seller quote account → fee account. Skip leg 3 or 4 if its amount is 0; keep leg numbers.
- Treat TigerBeetle `Exists` as success. Any other non-`Ok` result stops the consumer.
- `ON CONFLICT (trade_id) DO NOTHING` for trades; trades and order updates of one batch go in **one** Postgres transaction.
- Order update rule, enforced in SQL: apply only if `(status IS NULL OR status NOT IN (3,4,5)) AND filled_base_quantity <= new AND filled_quote_quantity <= new`. `status IS NULL` must be explicit.
- TigerBeetle is called before the Postgres commit in each batch.
- Exactly one consumer: Kubernetes `Recreate` plus a Postgres advisory lock taken at startup.
- No commit, no push.

## Out of scope (propose separately)

- Dockerfile target, k8s manifests, CI changes for the consumer.
- The transfer after a `REPLACED` report that lowers a reservation (the amount must be reproducible on replay; see *Decisions to confirm* 4). Reports with `exec_type` `REPLACED` still update the order row.
- Any change to `backend/src/main.rs` or the HTTP API.

## Decisions to confirm (the architect decides; defaults are used if no answer)

1. **Transfer ID formulas.** Docs say "derive from `trade_id` and the leg number" / "from `order_id`" without a formula, and say changing it after deployment is a mistake. Default: high 64 bits = high 64 bits of the source id (keeps the `tb::id()`/UUIDv7 time prefix), low 64 bits = first 8 bytes of `SHA-256(tag || id_be_bytes || leg)`, tags `b"trade-leg"` and `b"refund"`. Plain `trade_id + leg` is unsafe: ids within one millisecond are consecutive, so leg 2 of trade A can equal leg 1 of trade B. Distinct tags keep a refund from colliding with the backend's reservation transfer, which is also derived from `order_id`.
2. **Refund transfer code.** There is no `TransferCodeType` for refunds. Default: add `Refund = 6` and an arm `"refund"` in `transfer_kind` (`src/v1.rs`). That is the only edit outside `src/consumer/`.
3. **Fee cap in validation.** Docs say "doesn't exceed the amount implied by the stored fee rate". Default: cap = `ceil(quote_quantity × bps / 10000)`, with `bps` = taker rate if the order is on the taker side, else maker rate.
4. **Lowering-reservation transfer** after `REPLACED`: not built here. Its amount must come from the message or from data that doesn't change.
5. **Defaults** (env-overridable): `BATCH_MAX_ITEMS=500`, `BATCH_MAX_WAIT_MS=50`, `QUEUE_DEPTH=4`. Tune by measuring a replay.

## Review Focus

- First report of a brand-new order has `status IS NULL` (guard must still apply it). → Task 4.
- The same batch delivered twice produces identical final state and no error. → Task 8.
- Two reports for one order in one batch (`UPDATE … FROM` would apply an arbitrary one). → Task 3.
- A replay arriving mid-batch after live items (filled amounts go backwards within a batch). → Task 3.
- A report for an order missing from Postgres stops the consumer instead of being skipped. → Task 3 / Task 8.
- Stream ends or errors: partial batch is flushed, consumer resubscribes with capped backoff, never exits. → Task 6.
- A failing stage cancels the others and `run` returns `Err` (non-zero exit). → Task 7.

## File Structure

| File | Responsibility |
|---|---|
| `backend/src/lib.rs` | add `pub mod consumer;` |
| `backend/src/consumer/mod.rs` | `Config`, `run`, error type, re-exports |
| `backend/src/consumer/ids.rs` | deterministic transfer IDs (pure) |
| `backend/src/consumer/plan.rs` | pure: classify events, collapse reports, validate, build transfers (no I/O) |
| `backend/src/consumer/store.rs` | Postgres: known trades, orders/markets lookup, batch commit |
| `backend/src/consumer/ledger.rs` | TigerBeetle: send a batch of transfers |
| `backend/src/consumer/stream.rs` | subscription loop, backoff, batching |
| `backend/src/consumer/pipeline.rs` | the three stages, channels, cancellation |
| `backend/src/consumer/lock.rs` | Postgres advisory lock |
| `backend/src/bin/consumer.rs` | env → `Config`, signals, `run` |
| `backend/tests/consumer_test.rs` | integration tests against real Postgres + TigerBeetle and a fake router |
| `backend/src/domain.rs`, `backend/src/v1.rs` | `TransferCodeType::Refund` + `transfer_kind` arm (Decision 2) |
| `backend/Cargo.toml` | `tokio-util`, `sha2` already present; `tokio` `test-util` as dev-dependency |

---

### Task 1: Skeleton, dependencies, transfer IDs

**Files:**
- Create: `backend/src/consumer/mod.rs`, `backend/src/consumer/ids.rs`
- Modify: `backend/src/lib.rs`, `backend/Cargo.toml`, `backend/src/domain.rs`, `backend/src/v1.rs` (transfer_kind arm)

**Interfaces:**
- Produces:
  - `pub fn trade_leg_transfer_id(trade_id: u128, leg: u8) -> u128`
  - `pub fn refund_transfer_id(order_id: u128) -> u128`
  - `TransferCodeType::Refund = 6`
  - `pub type ConsumerError = Box<dyn std::error::Error + Send + Sync>` in `mod.rs`

- [ ] **Step 1: Write failing tests in `ids.rs`**
  - `legs_of_one_trade_have_distinct_ids`: legs 1..=4 of one `trade_id` give 4 different IDs.
  - `ids_are_deterministic`: same inputs twice give the same ID.
  - `refund_id_differs_from_trade_leg_id`: `refund_transfer_id(x) != trade_leg_transfer_id(x, 1)`.
  - `ids_are_never_zero_or_max`: TigerBeetle rejects `0` and `u128::MAX`; assert neither is returned for inputs `0`, `1`, `u128::MAX`.
- [ ] **Step 2: Run** `cd backend && cargo test consumer::ids` — expected FAIL (module missing).
- [ ] **Step 3: Implement** the two functions with SHA-256 as in Decision 1; map a result of `0` or `u128::MAX` to `1` (document why). Add `pub mod consumer;` to `lib.rs`, add `tokio-util = "0.7"` to `[dependencies]`, and `tokio = { version = "1.53.0", features = ["test-util"] }` under `[dev-dependencies]`. Add `Refund = 6` to `TransferCodeType` and the `"refund"` arm to `transfer_kind`.
- [ ] **Step 4: Run** `cargo test consumer::ids && cargo build` — expected PASS, builds clean.

---

### Task 2: Backoff and configuration

**Files:**
- Create: `backend/src/consumer/stream.rs` (only `Backoff` here), extend `mod.rs` with `Config`

**Interfaces:**
- Produces:
  - `pub struct Backoff` with `pub fn new(min: Duration, max: Duration) -> Self`, `pub fn next_delay(&mut self, jitter: f64) -> Duration` (`jitter` in `0.0..=1.0`, injected so tests are deterministic), `pub fn reset(&mut self)`.
  - `pub struct Config { pub router_url: String, pub database_url: String, pub tb_cluster_id: u128, pub tb_addresses: String, pub batch_max_items: usize, pub batch_max_wait: Duration, pub queue_depth: usize }` with `pub fn from_env() -> Result<Config, ConsumerError>` (env names: `ROUTER_URL`, `DATABASE_URL`, `TB_CLUSTER_ID`, `TB_IP_ADRESSES`, `BATCH_MAX_ITEMS`, `BATCH_MAX_WAIT_MS`, `QUEUE_DEPTH`; defaults from Decision 5).

- [ ] **Step 1: Write failing tests**
  - `delay_doubles_until_cap`: min 100 ms, max 1 s, jitter 0 → 100, 200, 400, 800, 1000, 1000 ms.
  - `jitter_only_shortens`: with jitter 1.0 each delay is ≥ half the un-jittered value and ≤ the value.
  - `reset_returns_to_min`.
  - `config_defaults_when_optional_vars_missing` and `config_missing_router_url_is_error` (use a helper `Config::from_lookup(impl Fn(&str) -> Option<String>)` so tests don't touch process env; `from_env` calls it).
- [ ] **Step 2: Run** `cargo test consumer::` — FAIL.
- [ ] **Step 3: Implement** as specified. No random crate: the caller supplies `jitter` (the loop in Task 6 derives it from the system clock's nanoseconds).
- [ ] **Step 4: Run** `cargo test consumer::` — PASS.

---

### Task 3: Pure batch planning (the core logic)

**Files:**
- Create: `backend/src/consumer/plan.rs`

**Interfaces:**
- Consumes: `proto::{EventItem, ExecutionReport, TradeSettlement, OrderStatus}`, `ids::*`, `tb::Transfer`.
- Produces (all `pub`, no I/O):
  ```text
  pub struct OrderRow   { order_id: Uuid, market_id: i32, base_account_id: Uuid, quote_account_id: Uuid,
                          buyer: bool, limit_price: Option<u128>, maker_fee_bps: i32, taker_fee_bps: i32,
                          user_id: Uuid }
  pub struct MarketRow  { market_id: i32, base_ledger_id: i32, quote_ledger_id: i32 }
  pub struct Lookups    { orders: HashMap<Uuid, OrderRow>, markets: HashMap<String /*symbol*/, MarketRow>,
                          known_trades: HashSet<Uuid> }
  pub struct TradeInsert { trade_id, market_id, buyer_order_id, seller_order_id, price, base_quantity,
                           quote_quantity, buyer_fee, seller_fee, taker_side: i16, engine_time: SystemTime }
  pub struct OrderUpdate { order_id, status: i16, cancel_reason: Option<i16>, filled_base: u128,
                           filled_quote: u128, limit_price: Option<u128> }
  pub struct BatchPlan   { transfers: Vec<tb::Transfer>, trades: Vec<TradeInsert>, order_updates: Vec<OrderUpdate> }
  pub enum PlanError { Validation(String), UnknownOrder(Uuid), UnknownMarket(String), Malformed(String) }
  pub fn wanted_ids(items: &[EventItem]) -> (Vec<Uuid> /*trade ids*/, Vec<Uuid> /*order ids*/, Vec<String> /*symbols*/)
  pub fn build_plan(items: &[EventItem], lookups: &Lookups) -> Result<BatchPlan, PlanError>
  ```

- [ ] **Step 1: Write failing tests** (build `EventItem`s with small helpers in the test module):
  - `known_trade_is_skipped_without_validation`: a `TradeSettlement` whose id is in `known_trades` and whose orders are **absent** from `orders` yields an empty plan, no error.
  - `new_trade_builds_four_linked_legs_in_order`: asserts debit/credit accounts, amounts, ledgers (base ledger for legs 1, quote ledger for 2–4), codes (`Trade` for 1–2, `FeeCollection` for 3–4), `Linked` set on legs 1–3 and not on leg 4, ids equal `trade_leg_transfer_id(trade_id, n)`.
  - `zero_fees_skip_legs_3_and_4_and_clear_linked_on_last_leg`: only 2 transfers; leg 1 linked, leg 2 not linked.
  - `two_trades_in_one_batch_form_separate_chains`.
  - `price_above_buyer_limit_is_validation_error` and `price_below_seller_limit_is_validation_error`; a market order (`limit_price: None`) passes.
  - `fee_above_cap_is_validation_error`; `fee_at_cap_passes` (Decision 3).
  - `account_not_belonging_to_order_is_validation_error` (e.g. `buyer_base_account != orders[buyer].base_account_id`).
  - `unknown_order_is_error` (`PlanError::UnknownOrder`), `unknown_market_is_error`.
  - `terminal_report_builds_refund`: a `FILLED`/`CANCELLED`/`EXPIRED` report emits one transfer with `BalancingDebit`, `amount == u128::MAX`, debit = order_id (hold account), credit = quote account for a buy / base account for a sell, id = `refund_transfer_id(order_id)`, code `Refund`, and the right ledger.
  - `non_terminal_report_builds_no_refund`.
  - `two_reports_same_order_collapse_to_highest_filled`: reports with filled 2 then 5 → one `OrderUpdate` with 5; reports 5 then 2 (replay after live) → still 5.
  - `terminal_beats_non_terminal_when_collapsing`; `equal_filled_later_arrival_wins`.
  - `terminal_refund_emitted_once_even_with_duplicate_terminal_reports`.
  - `report_for_order_not_in_lookups_is_error`.
  - `item_with_no_body_is_skipped` (plan unaffected, no error).
- [ ] **Step 2: Run** `cargo test consumer::plan` — FAIL.
- [ ] **Step 3: Implement** `wanted_ids` and `build_plan`. Collapsing rule: per `order_id`, keep the report with the greatest `(is_terminal, filled_base, filled_quote)`, ties to the later item. Quantities come from `Uint128` via `u128::from`; an absent required field is `PlanError::Malformed`. Process items in arrival order so transfer chains keep the engine's order. Remaining quantity arms are not needed.
- [ ] **Step 4: Run** `cargo test consumer::plan` — PASS.

---

### Task 4: Postgres store

**Files:**
- Create: `backend/src/consumer/store.rs`
- Test: `backend/tests/consumer_test.rs` (store section)

**Interfaces:**
- Consumes: `plan::{Lookups, OrderRow, MarketRow, BatchPlan, TradeInsert, OrderUpdate}`.
- Produces:
  - `pub async fn known_trade_ids(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<HashSet<Uuid>>`
  - `pub async fn load_orders(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<HashMap<Uuid, OrderRow>>`
  - `pub async fn load_markets(pool: &PgPool, symbols: &[String]) -> sqlx::Result<HashMap<String, MarketRow>>`
  - `pub async fn commit(pool: &PgPool, trades: &[TradeInsert], updates: &[OrderUpdate]) -> sqlx::Result<CommitStats>` with `pub struct CommitStats { pub trades_inserted: u64, pub orders_updated: u64, pub orders_skipped: u64 }`

Integration tests need Postgres with migrations applied; read `DATABASE_URL`, call `sqlx::migrate!()` on it. Each test seeds its own users/ledgers/accounts/markets/orders with fresh UUIDs, as `tests/integration_test.rs` does.

- [ ] **Step 1: Write failing tests**
  - `known_trade_ids_returns_only_existing`.
  - `commit_inserts_trade_once`: committing the same `TradeInsert` twice reports `trades_inserted` 1 then 0, and does not error.
  - `first_report_applies_when_status_is_null` (**the NULL case**): fresh order row with `status IS NULL` receives status 1.
  - `update_ignored_when_order_already_terminal`; `update_ignored_when_filled_would_go_down`; `update_applies_when_filled_equal` (e.g. `NEW` → `REPLACED`).
  - `commit_is_one_transaction`: force a failing statement (trade referencing a missing order) and assert that an update in the same call was rolled back.
  - `big_values_round_trip`: filled quantity `10^38` survives write and read (text/numeric binding).
- [ ] **Step 2: Run** `cargo test --test consumer_test store_ -- --nocapture` — FAIL.
- [ ] **Step 3: Implement.** Lookups use `= ANY($1)`. Trades insert with `INSERT … SELECT … FROM unnest(…) ON CONFLICT (trade_id) DO NOTHING`, quantities as `text[]` cast to `numeric`. Order updates run per `OrderUpdate` inside the same transaction (after Task 3's collapse there is at most one per order) with the guard from *Global Constraints*; count zero-row updates as `orders_skipped`. `OrderRow.buyer` comes from `side = 1`.
- [ ] **Step 4: Run** the same command — PASS. (Requires the stack: run via the integration script, see Task 8.)

---

### Task 5: TigerBeetle ledger

**Files:**
- Create: `backend/src/consumer/ledger.rs`
- Test: `backend/tests/consumer_test.rs` (ledger section)

**Interfaces:**
- Produces: `pub async fn send(client: &tb::Client, transfers: &[tb::Transfer]) -> Result<(), LedgerError>`; `pub enum LedgerError { Transport(String), Rejected { index: usize, result: tb::CreateTransferResult } }`. `Exists` is success; every other non-`Ok` result is `Rejected`. An empty slice is a no-op. Wrap the call in `tokio::time::timeout(5s)` → `Transport`.

- [ ] **Step 1: Write failing tests** against real TigerBeetle with fresh accounts and ledger numbers:
  - `sending_same_chain_twice_succeeds` (second call hits `Exists`).
  - `insufficient_funds_chain_is_rejected` (debit account with no balance → `Rejected`).
  - `balancing_refund_moves_remaining_and_is_repeatable`: fund a hold account with 100, spend 30, send the refund transfer (`BalancingDebit`, `amount u128::MAX`) → 70 returned; send it again → success, balances unchanged. **This test decides whether Decision 1/2 and the `u128::MAX` amount work with `Exists`; if TigerBeetle answers `ExistsWithDifferentAmount`, stop and report, do not paper over it.**
  - `refund_of_empty_hold_account_is_ok`.
- [ ] **Step 2: Run** `cargo test --test consumer_test ledger_` — FAIL.
- [ ] **Step 3: Implement** `send`.
- [ ] **Step 4: Run** — PASS.

---

### Task 6: Subscription loop and batching

**Files:**
- Modify: `backend/src/consumer/stream.rs`

**Interfaces:**
- Consumes: `service::EngineClient` (`subscribe_events`), `Backoff`, `Config`.
- Produces:
  - `pub async fn collect_batch<S: EventSource>(source: &mut S, max_items: usize, max_wait: Duration) -> BatchRead` where `pub enum BatchRead { Items(Vec<EventItem>), Ended(Vec<EventItem>) /* flush then reconnect */, Failed(Vec<EventItem>, tonic::Status) }`.
  - `pub trait EventSource { async fn next(&mut self) -> Result<Option<EventItem>, tonic::Status>; }` implemented for `tonic::Streaming<EventItem>` and for a test fake.
  - `pub async fn run_source(client: EngineClient, cfg: Arc<Config>, out: mpsc::Sender<Vec<EventItem>>, cancel: CancellationToken)`: connect → loop `collect_batch` → send; on `Ended`/`Failed` or a failed subscribe, flush any partial batch, wait `Backoff::next_delay`, resubscribe; `Backoff::reset` after a message arrives; return only when `cancel` fires or `out` is closed.

- [ ] **Step 1: Write failing tests** (`#[tokio::test(start_paused = true)]`, fake source):
  - `batch_closes_at_max_items`.
  - `batch_closes_at_max_wait_with_partial_items`: 2 items, then silence → `Items` of 2 after `max_wait`, without waiting for more.
  - `empty_wait_does_not_emit_empty_batch`.
  - `stream_end_returns_ended_with_partial_items`; `stream_error_returns_failed_with_partial_items`.
  - `run_source_resubscribes_after_end_and_error` using a fake client factory (closure returning sources) and asserting the call count, the backoff sleeps (paused clock), and that cancelling stops the loop.
- [ ] **Step 2: Run** `cargo test consumer::stream` — FAIL.
- [ ] **Step 3: Implement.** Race `source.next()` against the deadline with `tokio::select!`; start the deadline when the first item of a batch arrives. Log every reconnect at `warn` and keep a `reconnects` counter in a `Stats` struct (`AtomicU64`) shared via `Arc`. Enable keepalive in `EngineClient::new` is **not** touched; instead add `EngineClient::with_keepalive(url, interval: Duration, timeout: Duration)` next to `new` in `service.rs` (`http2_keep_alive_interval`, `keep_alive_timeout`, `keep_alive_while_idle(true)`) and use it from `run`.
- [ ] **Step 4: Run** `cargo test consumer::stream` — PASS.

---

### Task 7: Pipeline, lock, `run`, binary

**Files:**
- Create: `backend/src/consumer/pipeline.rs`, `backend/src/consumer/lock.rs`, `backend/src/bin/consumer.rs`
- Modify: `backend/src/consumer/mod.rs`

**Interfaces:**
- Consumes: Tasks 2–6.
- Produces:
  - `pub async fn try_lock(pool_url: &str) -> Result<LockGuard, ConsumerError>`; `LockGuard` owns a dedicated `PgConnection` holding `pg_try_advisory_lock(<fixed i64 key>)`; `try_lock` errors if the lock is held. Dropping the guard closes the connection.
  - `pub async fn run(cfg: Config, cancel: CancellationToken) -> Result<(), ConsumerError>`: connect Postgres and TigerBeetle, take the lock, spawn the three stages joined by `mpsc::channel(cfg.queue_depth)`, wait for the first stage to end or fail, cancel the rest, return the first error.
  - Stage A (prepare): receives `Vec<EventItem>`; `wanted_ids` → `known_trade_ids`, `load_orders`, `load_markets` → `build_plan` → sends `Prepared { transfers, trades, order_updates }`. `PlanError` is fatal.
  - Stage B (ledger): `ledger::send`; `Rejected`/`Transport` fatal (log with `error!`, which is the alert hook).
  - Stage C (commit): `store::commit`; logs `CommitStats` at `info`.
  - Each stage, on a fatal error, cancels the shared token and returns `Err`.
  - Binary: read `Config::from_env`, install SIGTERM/SIGINT handlers (as `main.rs` does) that cancel the token, call `run`, exit non-zero on `Err`.
  - On cancel: stage A stops reading and drops its sender, so B and C drain their queues and end.

- [ ] **Step 1: Write failing tests**
  - `lock_second_holder_is_refused`; `lock_is_released_when_guard_dropped` (integration, in `consumer_test.rs`).
  - `stage_error_cancels_other_stages`: pipeline unit test with stub stages (generic over closures) where B fails → `run_pipeline` returns the error and A's loop observed the cancel.
  - `cancel_drains_queued_batches`: batches already queued when cancel fires still reach stage C.
- [ ] **Step 2: Run** `cargo test consumer::pipeline && cargo test --test consumer_test lock_` — FAIL.
- [ ] **Step 3: Implement.** Keep stage bodies thin: they call the functions from Tasks 3–5 and forward. Factor the three stages as functions taking their channel ends and the token so `run_pipeline` can be tested with stubs.
- [ ] **Step 4: Run** the same commands, then `cargo build --bins` — PASS; `cargo run --bin consumer` with no env prints a clear "ROUTER_URL not set" style error and exits non-zero.

---

### Task 8: End-to-end tests with a fake router

**Files:**
- Modify: `backend/tests/consumer_test.rs`; `backend/scripts/integration-test.sh` (add `cargo test --test consumer_test` next to the existing `integration_test` invocation)

**Interfaces:**
- Consumes: everything above. A fake router implements `proto::trading_engine_server::TradingEngine`; only `subscribe_events` is real (the rest return `Status::unimplemented`). It serves a scripted list of `EventItem`s, optionally replaying the whole list on a second subscription, and can drop the stream after N items.

- [ ] **Step 1: Write failing tests** (each seeds users, ledgers, base/quote/fee accounts funded in TigerBeetle, hold accounts for both orders funded with their reservations, a market and two `orders` rows):
  - `trade_settles_and_orders_update`: one `TradeSettlement` + two `ExecutionReport`s (`FILLED`) → trade row exists, balances moved per the four legs, holds refunded, orders `status = 3`.
  - `replaying_the_same_events_changes_nothing`: serve the list twice (second subscription) → balances, trade count and order rows identical to after the first pass; consumer still running.
  - `stream_drop_mid_batch_then_replay_ends_consistent`: drop after 2 of 3 items, reconnect, full replay → final state equals a clean run.
  - `report_for_missing_order_stops_the_consumer`: `run` returns `Err`, nothing committed for that batch.
  - `invalid_trade_price_stops_the_consumer_with_alert_error`.
  - `second_consumer_cannot_start_while_first_runs` (lock).
  - `first_report_of_new_order_applies` (**NULL case** end to end).
- [ ] **Step 2: Run** `cd backend && bash scripts/integration-test.sh` — new tests FAIL.
- [ ] **Step 3: Fix** whatever the end-to-end run exposes in Tasks 3–7 (the only expected adjustments are wiring; if a test shows a design problem, stop and report instead of changing the design).
- [ ] **Step 4: Run** `bash scripts/integration-test.sh` — all tests, old and new, PASS. Then run `cargo test` (unit tests) and `cargo clippy --all-targets` — clean.
- [ ] **Step 5:** Do **not** commit. Report the list of changed files and the test output.

---

## Self-review

- **Spec coverage:** replay safety (known-trade query, SQL guard, `Exists`, `ON CONFLICT`) → Tasks 3–5; batching N/T → Task 6; reconnect/backoff/keepalive → Tasks 2, 6; single consumer → Task 7; pipeline + backpressure + cancellation → Task 7; one Postgres transaction per batch → Task 4; refunds → Tasks 3, 5; alerts are `error!` logs (no metrics stack exists in the repo; not added).
- **Type consistency:** `OrderRow`, `MarketRow`, `Lookups`, `BatchPlan`, `TradeInsert`, `OrderUpdate` are defined in Task 3 and used unchanged in Tasks 4 and 7. `LedgerError` and `CommitStats` are defined where produced.
- **Proportion:** the plan states interfaces, tests and rules; function bodies are left to the implementer.
- **Known risk:** Task 5's repeatable-refund test is the first place Decisions 1–2 and the `u128::MAX` balancing amount meet real TigerBeetle; if it fails, the design needs a decision, not a workaround.
