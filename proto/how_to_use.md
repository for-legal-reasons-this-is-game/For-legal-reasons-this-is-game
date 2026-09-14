# Coding against the trading contract

Checklist for the backend and the engine. Contract is `proto/trading/v1/`.

One rule underneath most of what follows: **the backend may decide from things
it writes — its own config, its own order table, the clock — and from the
message in front of it. It may not decide from anything it learned by reading
the engine's stream.** The book and the trades it keeps from the engine are for
display, not for decisions.

---

## Both sides

- Integers only. No floats anywhere, including display values.
- Scale comes from `Market`: `base_decimals`, `quote_decimals`, `price_decimals`. That's all `Market` has, plus symbol and fee account.
- `quote = price * base_qty / 10^(price_decimals + base_decimals - quote_decimals)`.
- Multiply in a 256-bit intermediate, then divide.
- Round **down**. Never up, never to nearest. (Sum of fills must fit inside the reservation.)
- Enum fields arrive as `i32`. Every conversion needs a fallback arm. No `unwrap`, no `panic`.
- Unknown `oneof` arm arrives as `None`. Log loudly, skip the message.
- Absent `Uint128`/`Id` is not zero. Check absence, not zero.
- Don't log account ids, user ids, or full orders at info level.

---

## Backend

### Enforce before you send — the engine no longer checks these

If you skip one of these, nobody catches it.

- [ ] Minimum order size, base and quote denominated. (Also what stops a fill flooring to a quote amount of zero.)
- [ ] Quantity step and price step, if the market uses them. Step of 1 = no constraint.
- [ ] Step of 0 is a division by zero in your validator. Reject or default it, deliberately.
- [ ] Market status. Halted or cancel-only → refuse at the HTTP layer, don't send.
- [ ] Minimum notional, if you want one. Approximate at the edges — it reads a price you learned from the stream.
- [ ] GTD is not on this list. Set `expire_time` on the request and the engine expires the order itself.

### Placing an order

- [ ] Validate the HTTP request first.
- [ ] Load the market from your own table. Not found or not TRADING → stop, don't touch money.
- [ ] Run the checks above. Still don't touch money.
- [ ] Read `maker_fee_bps` / `taker_fee_bps` into locals **now**. Don't re-read for this order ever again.
- [ ] Reserve, buy: `amount + ceil(amount * max(maker_bps, taker_bps) / 10000)`.
- [ ] Reserve, sell: `base_quantity` only, no fee part.
- [ ] Reservation fails → insufficient funds, stop.
- [ ] Generate `order_id` and `command_id`. Different values. Both 128-bit.
- [ ] Write the local order row as "sent, unconfirmed".
- [ ] Send `PlaceOrder` with the two fee rates you read.
- [ ] Market buy → `quote_quantity` arm. Everything else → `base_quantity` arm.
- [ ] GTD → set `expire_time`. No timer of your own; you'll get an `EXPIRED` report.
- [ ] Fee goes on top of `quote_quantity`, not inside it.
- [ ] Never reuse a `command_id` across different commands.

### Command response — four paths, all needed

All three trading commands return the same shape: `reject_reason`, `status`,
`cancel_reason`, and both filled amounts.

- [ ] `reject_reason` UNSPECIFIED → return `status` + fills to the HTTP caller. **Do not write to the order table.**
- [ ] Accepted with `status` CANCELLED and a `cancel_reason` → normal, not an error.
- [ ] A modify that repriced into the book can come back FILLED with amounts set. Same for a cancel that lost a race.
- [ ] `reject_reason` set → release the whole reservation, mark rejected. Nothing will arrive on the stream.
- [ ] Transport error, certain the engine never saw it → release everything.
- [ ] Transport error, not certain → treat as timeout.
- [ ] Timeout → retry with the **same `command_id`**. Do not release yet.
- [ ] Retries exhausted → still don't release. Wait for the stream or for a restart.
- [ ] Epoch changed → never retry. Release, move on.

### Event stream

- [ ] One subscription at startup, kept alive. Carries `ExecutionReport`, `TradeSettlement`, `Control`.
- [ ] Subscribe with `after_sequence` **and** `engine_epoch` from the saved cursor. Both, always.
- [ ] First run ever → send zeros.
- [ ] Every item: epoch changed → run the restart procedure, stop normal processing.
- [ ] `sequence` <= cursor → already seen, skip. Normal after reconnects.
- [ ] `sequence` > cursor + 1 → gap. Log as serious, resubscribe to force a snapshot.
- [ ] Apply the effect and advance the cursor in **one** Postgres transaction.
- [ ] Process strictly in sequence order. The engine guarantees a trade's settlement comes before the reports that reflect it, and an order's terminal report comes after every settlement that touched it — so no special ordering logic is needed.

`ExecutionReport`:

- [ ] Overwrite totals, never accumulate. Every report is a full picture, `limit_price` included.
- [ ] Read the remaining quantity from whichever arm is set — quote arm on a market buy, base arm otherwise.
- [ ] Status CANCELLED, EXPIRED or FILLED → release what's left of the reservation.
- [ ] `exec_type` TRADE → `trade_id` is set. Join to the `TradeSettlement` with that id for price, amounts, fee, counterparty.
- [ ] `exec_type` RESTATED → you're inside a snapshot.

`TradeSettlement` → settle (below).

`Control`:

- [ ] `SNAPSHOT_BEGIN` → drop everything you believe about engine state.
- [ ] `RESTATED` run → rebuild working orders. These carry `sequence` 0; don't store it.
- [ ] `SNAPSHOT_END` → set cursor to `as_of_sequence`.
- [ ] Anything you thought was working and didn't appear → cancel locally, release.

### Settling

- [ ] Key on `trade_id`. Unique constraint in Postgres, not check-then-insert.
- [ ] One linked chain, this order:
      1. `base_quantity` seller_base → buyer_base
      2. `quote_quantity` buyer_quote → seller_quote
      3. `buyer_fee` buyer_quote → fee_account (skip if 0)
      4. `seller_fee` seller_quote → fee_account (skip if 0)
- [ ] Do not reorder. The seller's fee is funded by step 2.
- [ ] All four commit or none.
- [ ] Everything you need is in the message. No lookups — that's what makes replay safe.
- [ ] Lower each side's reservation by what its legs used.
- [ ] Release the fee over-lock if the order filled on the cheaper side.

### Releasing reservations — every path

- [ ] Fill (partial consume + fee over-lock)
- [ ] User cancel
- [ ] IOC / FOK remainder
- [ ] GTD expiry (arrives as an `EXPIRED` report, not a cancel)
- [ ] Synchronous rejection
- [ ] Mass cancel (arrives as reports with `cancel_reason` ADMIN)
- [ ] Engine restart, for every order it no longer has
- [ ] Modify that lowered quantity or price
- [ ] Release on the **stream event**, never on an RPC response.
- [ ] Modify raising qty/price → lock the extra **before** the call.
- [ ] Modify lowering → release **after** it succeeds.
- [ ] Modify reservations use the rates sent on the original `PlaceOrder`, not your market table.

### Engine restart

Triggered by a changed `engine_epoch`, or an unrequested snapshot.

- [ ] Stop serving new orders.
- [ ] Drop every in-flight command, release their reservations, retry none.
- [ ] Push every market with `SetMarket`.
- [ ] Resubscribe with the new epoch and `after_sequence` 0.
- [ ] Take the snapshot (it will be empty).
- [ ] Every order still local-working → cancel locally, release, notify the user.
- [ ] Save the new epoch, resume serving.
- [ ] Check *that* the epoch changed. Never compare which is bigger.

### Market data stream

Display only. Nothing decides from this.

- [ ] Separate subscription from the event stream.
- [ ] Keep the book in memory. Never write it to Postgres.
- [ ] A market's `BookSnapshot` always arrives before any delta for that market.
- [ ] `md_sequence` gap → drop the book, reconnect, re-snapshot. Don't repair.
- [ ] Never save `md_sequence` as a cursor.
- [ ] Delta quantity 0 → remove the level. Otherwise it's the level's new total.
- [ ] Serve frontend depth and top-of-book from this.
- [ ] Build the public tape from `TradeSettlement`, stripping accounts at the WebSocket layer.

### Admin

- [ ] `SetMarket` and `CancelOrders` behind an admin-only path. Same as `POST /ledgers`.
- [ ] `markets` table: `base_ledger_id` / `quote_ledger_id` FK into `ledgers`. Read decimals through them. Don't store a second copy.
- [ ] Minimums, steps and status live in that table too. They never go on the wire.
- [ ] Fee rate edits need no coordination. Working orders keep their rates.
- [ ] Decimals edits can come back `HAS_OPEN_ORDERS`. Halt in your own table, clear the book, then push.

---

## Engine

### Startup

- [ ] `engine_epoch` = process start time, Unix nanos. Same value on every item on both streams.
- [ ] No markets loaded. Everything gets `UNKNOWN_MARKET` until pushed.
- [ ] Event sequence starts at 1.
- [ ] Command cache empty.

### Every command, before anything else

- [ ] Known `command_id` + same request hash → return the stored response, run nothing.
- [ ] Known `command_id` + different hash → `DUPLICATE_COMMAND`.
- [ ] New `command_id` → run it, store the response.
- [ ] Cache retention window must exceed the backend's whole retry budget.
- [ ] Cache is in memory, dies with the process. Fine — so do the orders.

### Validating a place

No order created, nothing on the stream, for any failure here. No minimums, no
steps, no market status — the backend already did those.

- [ ] Market exists → `UNKNOWN_MARKET`
- [ ] Correct amount arm (`quote_quantity` for market/stop-market buy, `base_quantity` otherwise) → `BAD_ORDER_PARAMS`
- [ ] Quantity positive → `BAD_QUANTITY`
- [ ] Price positive → `BAD_PRICE`
- [ ] `limit_price` present iff LIMIT / STOP_LIMIT
- [ ] `trigger_price` + `trigger_direction` present iff STOP_LIMIT / STOP_MARKET
- [ ] `expire_time` present iff GTD, and in the future
- [ ] `protection_price` only on market orders
- [ ] Result still fits in 128 bits after the division → `BAD_QUANTITY`
- [ ] Feature implemented at this stage → `UNSUPPORTED`
- [ ] Pin `maker_fee_bps` / `taker_fee_bps` from the request onto the order, for life.

### Validating a cancel or modify

- [ ] Order exists → `UNKNOWN_ORDER`
- [ ] Not already finished → `ORDER_NOT_WORKING`
- [ ] `account_holder` owns it → `NOT_ORDER_OWNER`
- [ ] Modify: new quantity above what's already filled → `BAD_QUANTITY`
- [ ] Modify: order is base-denominated → else `ORDER_NOT_WORKING`

### Matching

- [ ] Price, then time.
- [ ] Self-trade → skip the colliding quantity on the incoming order, leave the resting one alone.
- [ ] Nothing left to match after that → admit, then cancel with `SELF_TRADE`.
- [ ] IOC → fill what crosses, cancel the rest `IOC_REMAINDER`; nothing filled → `IOC_NO_FILL`.
- [ ] FOK → check the full fill is possible **before** filling anything; otherwise cancel `FOK_NO_FULL_FILL`, book untouched.
- [ ] Protection price → stop, cancel remainder `PROTECTION_PRICE`. Can happen after partial fills.
- [ ] Market order, empty book → fills nothing, cancelled.
- [ ] Quote per fill: round down. Fee per fill: round down.
- [ ] Quote amount rounding to 0 → shouldn't be reachable; log it, the backend's minimum is too low.
- [ ] Charge each fill at its own role's rate. One order can be taker on part and maker on the rest.
- [ ] `fee = quote * rate_bps / 10000`, rounded down.

### Emitting, per match

- [ ] One `TradeSettlement`, then the buyer's `ExecutionReport`, then the seller's.
- [ ] Settlement always before the reports that reflect it.
- [ ] An order's terminal report always after every settlement that touched it. (Otherwise the backend's release runs early and hands back money the next event spends.)
- [ ] `TradeSettlement` carries both sides' accounts, both fees, the fee account. It must be settleable with no lookups.
- [ ] The fill's `ExecutionReport` carries only `trade_id`. No per-side copy of the trade.

### Emitting, per order change

One `ExecutionReport` for every change. No silent transitions.

- [ ] Accepted / resting → `NEW`
- [ ] Stop order waiting → status `PENDING_TRIGGER`
- [ ] Trigger fires → `TRIGGERED`
- [ ] Fill → `TRADE`, `trade_id` set
- [ ] Modify applied → `REPLACED`
- [ ] Cancelled → `CANCELLED` + `cancel_reason`
- [ ] GTD time reached → `EXPIRED`. Its own terminal state, not a cancel.
- [ ] Every report carries running totals for the whole order, including current `limit_price`.

### Also the engine's job

- [ ] A timer for GTD orders. Expire them yourself; nobody will ask.
- [ ] Watching last traded price to fire stop triggers.
- [ ] Modify lowering quantity → keeps queue position.
- [ ] Modify raising quantity or changing price → back of the queue at the new price.
- [ ] Modify that reprices into the book matches immediately; response may come back FILLED or CANCELLED with amounts.
- [ ] `CancelOrders` → one ordinary `ExecutionReport` per order, `cancel_reason` ADMIN.
- [ ] `cancelled_count` in the response is advisory. The reports are the record.

### Event stream

- [ ] `sequence` +1 per item, no gaps, no repeats, for the life of the process.
- [ ] Snapshot lines carry `sequence` 0.
- [ ] `Control` items carry no market symbol.

Snapshot procedure:

- [ ] Pick `N` = current sequence.
- [ ] Send `SNAPSHOT_BEGIN`.
- [ ] One RESTATED `ExecutionReport` per working order, including `type` and `time_in_force`.
- [ ] **Buffer everything that happens during that.** Do not interleave it.
- [ ] Send `SNAPSHOT_END` with `as_of_sequence = N`.
- [ ] Flush the buffer, go live.

Snapshot when: `after_sequence` is 0, OR the request's `engine_epoch` doesn't match, OR `after_sequence` is older than retained history. Otherwise replay from `after_sequence + 1`.

- [ ] Buffer limit hit → drop the subscriber, let it reconnect. Never block matching.
- [ ] Multiple subscribers allowed, each with its own position.
- [ ] Write down how much history is retained.

### Market data stream

- [ ] Separate connection, separate `md_sequence`.
- [ ] On connect: one `BookSnapshot` per market, then deltas.
- [ ] A market's snapshot must precede any delta for that market.
- [ ] One delta per affected price level; quantity 0 removes it.
- [ ] Drop slow subscribers freely.
- [ ] No trades on this stream.

### Never

- [ ] Never touch money, hold a balance, or check whether someone can afford something.
- [ ] Never change market config on its own.
- [ ] Never decide a fee rate.
- [ ] Never invent an `order_id`.
- [ ] Never enforce a minimum, a step, or a market status. Those aren't yours.
      (Expiry is yours. Minimums aren't. The line is whether it needs the book or the clock you own.)

---

## Easy to get wrong

- Deciding anything from the book or the trades you read off the stream. Display only.
- Writing order state from a command response.
- Forgetting to release the reservation on a synchronous rejection.
- Retrying a command after the epoch changed.
- Cursor advanced in a different transaction from the effect.
- Reading `remaining_base_quantity` on a market buy.
- Persisting `md_sequence`.
- Rounding to nearest.
- Re-reading fee rates for an order already placed.
- Assuming the engine checks minimums or steps. It doesn't any more.
- `unwrap()` on an enum conversion.
- A float "just for display".

---

## Tests first

Backend:

- Rejection releases the full reservation.
- Partial fill releases the right amount, no more.
- Duplicate stream item changes nothing.
- Duplicate `TradeSettlement` posts one chain.
- Epoch change cancels every local order and releases everything.
- Market buy's remaining read from the quote arm.
- Fee rate edit doesn't change a resting order's charge.
- An undersized order is refused before any money is reserved.
- An `EXPIRED` report releases the reservation the same as a cancel.

Engine:

- Same command twice → same answer, one order.
- Same `command_id`, different contents → rejected.
- IOC crossing nothing → cancelled order, not a rejection.
- FOK that can't fill fully → book untouched.
- Order matching only its own owner → admitted, then cancelled.
- Taker on part, maker on the rest → both rates charged.
- A GTD order past its `expire_time` reports `EXPIRED`, not `CANCELLED`.
- A trade's settlement precedes both its reports, and the terminal report follows every settlement.
- No sequence gaps across a snapshot into live items.
- Order placed during a snapshot arrives after `SNAPSHOT_END`.
- Quote and fees always round down.

Both:

- Place, fill, settle; the four accounts plus the fee account balance to the cent.
- An IOC that fills once and cancels its remainder releases exactly the unspent amount, not more.

Field numbers freeze:

- Never change or reuse a field number. Mark a removed one `reserved`, with its
  name.
- Add fields and enum values only; never repurpose one.
- Every enum keeps its `_UNSPECIFIED = 0`.
- A `trading.v2` package is only for a change that cannot be additive.

Worth knowing on the Rust side: prost gives you enum fields as `i32`, so an
unknown value fails at conversion, not at decode — every conversion needs a
catch-all arm. An unknown `oneof` arm arrives as `None`; handle it loudly rather
than ignoring the message.

