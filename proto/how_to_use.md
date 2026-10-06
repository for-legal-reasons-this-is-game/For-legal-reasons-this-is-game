# Coding against the trading contract

This guide describes what the backend and the trading engine must each do to
implement the gRPC contract in `proto/`. It covers order placement, the event
stream, settlement, reservations, engine restarts, the market data stream, and
the tests that each component needs.

The backend design that this guide assumes is described in `TB_DESIGN.md`:
TigerBeetle stores and enforces all money movements, each order has its own
TigerBeetle *hold account*, and Postgres stores history and configuration.

## Core rule

The backend makes decisions only from data it writes itself and from the
message it's currently processing:

- **Allowed:** the backend's own configuration, its `orders` table, the
  open-orders map, the clock, and the current message.
- **Not allowed:** anything the backend learned by reading the engine's streams,
  such as the order book or past trades.

The order book and the trades that the backend receives from the engine are for
display only.

## Responsibilities

| Component | Owns |
|---|---|
| Backend | Money, reservations, settlement, market configuration, minimums, steps, and market status. |
| Engine | The order book, matching, order state, fee amounts per fill, and order expiry. |

The engine never touches money. The backend never decides matching outcomes.

## Rules for both components

### Arithmetic

- Use integers only. Don't use floating-point numbers anywhere, including for
  display values.
- Take the scale of every amount from `Market`: `base_decimals`,
  `quote_decimals`, and `price_decimals`.
- Convert a base quantity to a quote amount with this formula:

  `quote = price × base_qty / 10^(price_decimals + base_decimals − quote_decimals)`

- Multiply in a 256-bit intermediate value, then divide.
- Always round **down**. Never round up or to the nearest value. Rounding down
  guarantees that the sum of all fills fits inside the reservation.

### Decoding messages

- prost decodes enum fields as `i32`. An unknown value fails at conversion, not
  at decode. Give every conversion a fallback arm. Don't use `unwrap` or
  `panic`.
- An unknown `oneof` arm arrives as `None`. Log it at a high severity and skip
  the message.
- An absent `Uint128` or `Id` field isn't zero. Check whether the field is
  present; don't compare it to zero.

### Logging

- Don't log account IDs, user IDs, or full orders at the `info` level.

## Backend

### Checks the engine doesn't perform

The engine doesn't validate the following rules. If the backend skips one,
nothing else catches it. Enforce all of them before you reserve funds.

| Check | Notes |
|---|---|
| Minimum order size | Enforce both a base minimum and a quote minimum. The minimum also prevents a fill from rounding down to a quote amount of zero. |
| Quantity step and price step | Only if the market uses them. A step of 1 means no constraint. |
| Step of 0 | Causes a division by zero in the validator. Reject or replace it deliberately. |
| Market status | If the market is halted or cancel-only, refuse the request at the HTTP layer. Don't send it to the engine. |
| Minimum notional | Optional. It's approximate at the edges, because it depends on a price learned from the stream. |

> **Note:** GTD expiry isn't a backend check. Set `expire_time` on the request,
> and the engine expires the order itself.

### Placing an order

Each order has three identifiers:

| Identifier | Created by | Purpose |
|---|---|---|
| `idempotency_key` | Frontend | Detects a retried HTTP request. Stored as a unique column in `orders`. |
| `order_id` | Backend, with `tb::id()` | Identifies the order. Also the ID of the order's hold account in TigerBeetle. |
| `command_id` | Backend | Identifies the `PlaceOrder` gRPC command. The engine uses it to detect a retried command. |

`order_id` and `command_id` are both 128-bit and must have different values.

To place an order, follow these steps:

1. Validate the HTTP request.
2. Load the market from the `markets` table. If the market doesn't exist or
   isn't trading, stop. Don't reserve any funds.
3. Run the [checks the engine doesn't perform](#checks-the-engine-doesnt-perform).
   Still don't reserve any funds.
4. Read `maker_fee_bps` and `taker_fee_bps` into local variables. Use these
   values for this order for its whole life; don't read them from the market
   again.
5. Compute the reservation amount:

   | Order | Reserved asset | Reservation amount |
   |---|---|---|
   | Buy | Quote | `amount + ceil(amount × max(maker_fee_bps, taker_fee_bps) / 10000)` |
   | Sell | Base | `base_quantity`. No fee part. |

   For a market or stop-market buy, `amount` is `quote_quantity`. For a limit or
   stop-limit buy, `amount` is the quote value of `base_quantity` at
   `limit_price`, computed with the [conversion formula](#arithmetic).

6. Generate `order_id` and `command_id`.
7. Insert the order row with `submission` set to `reserving`. If the
   `idempotency_key` already exists, return the existing order instead.
8. Create the hold account in TigerBeetle:
   - ID: `order_id`.
   - Ledger: the ledger of the reserved asset.
   - Flags: `debits_must_not_exceed_credits`.

   If TigerBeetle returns `exists`, continue.
9. Transfer the reservation amount from the user's account to the hold account.
   Derive the transfer ID from `order_id`. If TigerBeetle rejects the transfer
   for insufficient funds, set `submission` to `rejected` and stop.
10. Add the order to the open-orders map.
11. Send `PlaceOrder` with the fee rates from step 4:
    - For a market or stop-market buy, set the `quote_quantity` arm. For every
      other order, set the `base_quantity` arm.
    - The buyer's fee is charged on top of `quote_quantity`, not inside it.
    - For a GTD order, set `expire_time`. Don't run a timer in the backend; the
      engine sends an `EXPIRED` report.

Never reuse a `command_id` for a different command.

### Handling command responses

`PlaceOrder`, `CancelOrder`, and `ModifyOrder` return the same fields:
`reject_reason`, `status`, `cancel_reason`, and both filled amounts.

A command response never changes the order's `status` or filled amounts. Only
the event stream writes those columns. The response only updates `submission`
and the HTTP reply.

| Outcome | What to do |
|---|---|
| `reject_reason` is `UNSPECIFIED` (accepted) | Set `submission` to `accepted`. Return `status` and the filled amounts to the HTTP caller. |
| Accepted with `status` `CANCELLED` and a `cancel_reason` | Normal outcome, not an error. For example, an IOC order that crossed nothing. |
| Accepted with `status` `FILLED` after a modify or cancel | Normal outcome. A modify can reprice into the book, and a cancel can lose a race with a fill. |
| `reject_reason` is set | Refund the whole reservation and set `submission` to `rejected`. Nothing about this order arrives on the stream. |
| Transport error, and the engine certainly didn't receive the command | Refund the whole reservation. |
| Transport error, and you aren't certain | Treat it as a timeout. |
| Timeout | Retry with the **same** `command_id`. Don't refund yet. |
| Retries exhausted | Still don't refund. Wait for the stream or for an engine restart to resolve the order. |
| Engine epoch changed | Never retry. Refund and move on. |

### Processing the event stream

The event stream carries `ExecutionReport`, `TradeSettlement`, and `Control`
items. Open one subscription at startup and keep it alive.

#### Subscribing

1. Read `engine_epoch` and `last_sequence` from the `engine_stream_state` table.
2. Subscribe with both values as `engine_epoch` and `after_sequence`. Always send
   both. On the very first run, both are 0.

#### Processing in batches

Process items strictly in sequence order. The engine guarantees that a trade's
settlement arrives before the reports that reflect it, and that an order's final
report arrives after every settlement that touched it. No additional ordering
logic is needed.

For each item, check the following first:

| Condition | Action |
|---|---|
| `engine_epoch` differs from the stored epoch | Stop normal processing and run the [engine restart procedure](#handling-an-engine-restart). |
| `sequence` is less than or equal to the cursor | Already processed. Skip it. This is normal after a reconnect. |
| `sequence` is greater than the cursor + 1 | A gap. Log it as a serious error and resubscribe to force a snapshot. |

Then process items in batches:

1. Collect items until the batch has N items or T milliseconds have passed.
2. Build the TigerBeetle transfers for every item in the batch, as described in
   [Settling a trade](#settling-a-trade) and
   [Refunding reservations](#refunding-reservations).
3. Send all transfers in one TigerBeetle request. Treat `exists` as success.
4. In one Postgres transaction, write the batch's trades and order updates, and
   advance the cursor in `engine_stream_state`.

Advance the cursor only after the TigerBeetle request succeeds. If the backend
crashes between steps 3 and 4, it reprocesses the batch. Deterministic transfer
IDs make TigerBeetle return `exists` instead of moving money twice.

If any transfer chain fails, stop processing at that item and raise an alert.
Don't skip the item, and don't advance the cursor past it.

#### `ExecutionReport`

- Overwrite the order's totals; never add to them. Every report contains the
  complete current state of the order, including `limit_price`.
- Read the remaining quantity from the arm that's set: the quote arm for a market
  buy, the base arm for every other order.
- If `status` is `CANCELLED`, `EXPIRED`, or `FILLED`, refund the rest of the
  reservation and remove the order from the open-orders map.
- If `exec_type` is `TRADE`, `trade_id` is set. Join it to the `TradeSettlement`
  with the same ID to get the price, amounts, fees, and counterparty.
- If `exec_type` is `RESTATED`, the report is part of a snapshot.

#### `Control`

| Kind | Action |
|---|---|
| `SNAPSHOT_BEGIN` | Discard everything you believe about the engine's state. |
| `RESTATED` reports that follow | Rebuild the set of working orders. These reports have `sequence` 0; don't store it as the cursor. |
| `SNAPSHOT_END` | Set the cursor to `as_of_sequence`. Every order you considered working that didn't appear in the snapshot: cancel it locally and refund it. |

### Settling a trade

Every `TradeSettlement` contains the IDs of both orders, which are also the IDs
of their hold accounts, plus every user account and the fee account. You can
build the transfers from the message alone. Use the open-orders map only to
validate the message.

#### Validating

Before you build the transfers, check the message against the open-orders map:

- The users and accounts match both orders.
- The market matches both orders.
- The price respects each order's limit price.
- Each fee doesn't exceed the amount implied by the order's stored fee rate.

If a check fails, stop processing and raise an alert.

#### Building the transfer chain

Send the legs of one trade as one linked chain, in this order:

| Leg | Amount | Debit account | Credit account |
|---|---|---|---|
| 1 | `base_quantity` | Seller's hold account | Buyer's base account |
| 2 | `quote_quantity` | Buyer's hold account | Seller's quote account |
| 3 | `buyer_fee` | Buyer's hold account | Fee account |
| 4 | `seller_fee` | Seller's quote account | Fee account |

- Skip leg 3 or leg 4 if its amount is 0. Keep the leg numbers unchanged.
- Don't reorder the legs. Leg 2 funds the seller's fee in leg 4.
- Derive each transfer ID from `trade_id` and the leg number.
- The chain is linked: all legs succeed, or none do.

The hold accounts replace manual reservation tracking. Each leg that debits a
hold account lowers that order's reservation automatically, and TigerBeetle
rejects any leg that would exceed it.

#### Recording

Insert the trade into `trades` in the batch's Postgres transaction. The primary
key on `trade_id` skips a trade that's already recorded.

### Refunding reservations

A refund moves the remaining balance of a hold account back to the user's
account on the same ledger. Use a transfer with the `balancing_debit` flag and a
very large amount: TigerBeetle moves exactly what's left. Derive the transfer ID
from `order_id`, so that a repeated refund returns `exists`.

The refund also returns any fee over-reservation, for example when an order
filled at the cheaper maker rate.

Refund in every one of these cases:

| Case | Trigger |
|---|---|
| Order filled | `ExecutionReport` with status `FILLED` |
| User cancel | `ExecutionReport` with status `CANCELLED` |
| IOC or FOK remainder | `ExecutionReport` with status `CANCELLED` |
| GTD expiry | `ExecutionReport` with status `EXPIRED`. Not a cancel. |
| Mass cancel | `ExecutionReport` with `cancel_reason` `ADMIN`, one per order |
| Synchronous rejection | Command response with `reject_reason` set |
| Engine restart | The order is missing from the snapshot |

Except for synchronous rejections, refund on the **stream event**, never on a
command response.

#### Modifying an order

A modify changes the reservation without ending the order:

- **Raising quantity or price:** before you send `ModifyOrder`, transfer the
  additional amount from the user's account to the hold account.
- **Lowering quantity or price:** after the `REPLACED` report arrives, transfer
  the excess from the hold account back to the user's account.
- Compute the new reservation with the fee rates sent on the original
  `PlaceOrder`, not the current rates in the `markets` table.
- Derive these transfer IDs from the modify's `command_id`.
- Update `orders.reserved_amount` to the new reservation.

### Handling an engine restart

A changed `engine_epoch`, or a snapshot you didn't request, means the engine
restarted.

1. Stop accepting new orders.
2. Drop every in-flight command and refund its reservation. Don't retry any of
   them.
3. Send `SetMarket` for every market.
4. Resubscribe with the new epoch and `after_sequence` 0.
5. Process the snapshot. It's empty, because the engine keeps orders in memory.
6. For every order that's still working locally: cancel it locally, refund it,
   and notify the user.
7. Save the new epoch in `engine_stream_state`, and start accepting orders again.

Check only whether the epoch changed. Never compare which epoch is larger.

### Processing the market data stream

The market data stream is for display only. Nothing decides from it.

- Use a separate subscription from the event stream.
- Keep the order book in memory. Never write it to Postgres.
- A market's `BookSnapshot` always arrives before any delta for that market.
- If `md_sequence` has a gap, discard the book, reconnect, and take a new
  snapshot. Don't try to repair the book.
- Never store `md_sequence` as a cursor.
- A delta with quantity 0 removes the price level. Any other quantity is the
  level's new total.
- Serve the frontend's depth and top-of-book from this stream.
- Build the public trade feed from `TradeSettlement`, and remove the account
  fields at the WebSocket layer.

### Administration

- Put `SetMarket` and `CancelOrders` behind an admin-only path, like
  `POST /ledgers`.
- In the `markets` table, reference `ledgers` with `base_ledger_id` and
  `quote_ledger_id`, and read decimals through them. Don't store a second copy.
- Store minimums, steps, and market status in the `markets` table. They're never
  sent to the engine.
- Fee rate changes need no coordination. Working orders keep the rates they were
  placed with.
- A decimals change can return `HAS_OPEN_ORDERS`. Halt the market in your own
  table, clear the book, then send `SetMarket` again.

## Engine

### Startup

- Set `engine_epoch` to the process start time, in Unix nanoseconds. Send the
  same value on every item of both streams.
- Start with no markets loaded. Reject every command with `UNKNOWN_MARKET` until
  the backend sends `SetMarket`.
- Start the event sequence at 1.
- Start with an empty command cache.

### Handling a command

Before you do anything else with a command, check the command cache:

| Cache state | Action |
|---|---|
| `command_id` known, same request hash | Return the stored response. Don't run the command again. |
| `command_id` known, different request hash | Reject with `DUPLICATE_COMMAND`. |
| `command_id` new | Run the command and store its response. |

- Keep cache entries longer than the backend's whole retry period.
- The cache lives in memory and is lost when the process stops. That's
  acceptable, because the orders are lost too.

### Validating a place command

If validation fails, don't create an order and don't emit anything on the
stream. Don't check minimums, steps, or market status; the backend already did.

| Condition | Reject reason |
|---|---|
| The market doesn't exist | `UNKNOWN_MARKET` |
| Wrong amount arm: market and stop-market buys need `quote_quantity`, every other order needs `base_quantity` | `BAD_ORDER_PARAMS` |
| Quantity isn't positive | `BAD_QUANTITY` |
| A price isn't positive | `BAD_PRICE` |
| `limit_price` isn't set exactly for LIMIT and STOP_LIMIT | `BAD_ORDER_PARAMS` |
| `trigger_price` and `trigger_direction` aren't set exactly for STOP_LIMIT and STOP_MARKET | `BAD_ORDER_PARAMS` |
| `expire_time` isn't set exactly for GTD, or isn't in the future | `BAD_ORDER_PARAMS` |
| `protection_price` is set on an order that isn't a market order | `BAD_ORDER_PARAMS` |
| The result doesn't fit in 128 bits after division | `BAD_QUANTITY` |
| The feature isn't implemented yet | `UNSUPPORTED` |

Store `maker_fee_bps` and `taker_fee_bps` from the request on the order for its
whole life.

### Validating a cancel or modify command

| Condition | Reject reason |
|---|---|
| The order doesn't exist | `UNKNOWN_ORDER` |
| The order has already ended | `ORDER_NOT_WORKING` |
| `account_holder` doesn't own the order | `NOT_ORDER_OWNER` |
| Modify: the new quantity isn't above the filled quantity | `BAD_QUANTITY` |
| Modify: the order isn't base-denominated | `ORDER_NOT_WORKING` |

### Matching

- Match by price, then by time.
- **Self-trade:** skip the colliding quantity on the incoming order, and leave
  the resting order unchanged. If nothing is left to match, accept the order and
  then cancel it with `SELF_TRADE`.
- **IOC:** fill what crosses and cancel the rest with `IOC_REMAINDER`. If nothing
  fills, cancel with `IOC_NO_FILL`.
- **FOK:** check that a full fill is possible **before** filling anything. If
  it isn't, cancel with `FOK_NO_FULL_FILL` and leave the book unchanged.
- **Protection price:** when reached, stop and cancel the remainder with
  `PROTECTION_PRICE`. This can happen after partial fills.
- **Market order on an empty book:** fill nothing and cancel the order.
- Round the quote amount and the fee of each fill down.
- A quote amount that rounds to 0 shouldn't be possible. Log it: the backend's
  minimum is too low.
- Charge each fill at the rate of its own role. One order can be the taker for
  part of its quantity and the maker for the rest.
- Compute the fee as `fee = quote × rate_bps / 10000`, rounded down.

### Emitting events for a match

1. Emit one `TradeSettlement`.
2. Emit the buyer's `ExecutionReport`.
3. Emit the seller's `ExecutionReport`.

- A settlement always comes before the reports that reflect it.
- An order's final report always comes after every settlement that touched it.
  Otherwise the backend refunds too early and returns money that the next event
  spends.
- `TradeSettlement` carries both sides' accounts, both fees, and the fee account.
  The backend must be able to settle it without any lookup.
- The fill's `ExecutionReport` carries only `trade_id`, not a copy of the trade.

### Emitting events for an order change

Emit one `ExecutionReport` for every change. Never change an order without a
report.

| Change | Report |
|---|---|
| Accepted or resting | `exec_type` `NEW` |
| Stop order waiting for its trigger | status `PENDING_TRIGGER` |
| Trigger fired | `exec_type` `TRIGGERED` |
| Fill | `exec_type` `TRADE`, with `trade_id` set |
| Modify applied | `exec_type` `REPLACED` |
| Cancelled | `exec_type` `CANCELLED`, with `cancel_reason` set |
| GTD time reached | `exec_type` `EXPIRED`. A final state of its own, not a cancel. |

Every report carries the running totals for the whole order, including the
current `limit_price`.

### Other engine duties

- Run a timer for GTD orders and expire them. Nobody else asks for it.
- Watch the last traded price to fire stop triggers.
- A modify that lowers quantity keeps the order's queue position.
- A modify that raises quantity or changes price moves the order to the back of
  the queue at the new price.
- A modify that reprices into the book matches immediately. The response can
  return `FILLED` or `CANCELLED` with amounts.
- For `CancelOrders`, emit one normal `ExecutionReport` per order, with
  `cancel_reason` `ADMIN`.
- `cancelled_count` in the response is informational. The reports are the
  record.

### Event stream

- Increase `sequence` by 1 per item, with no gaps or repeats, for the life of
  the process.
- Snapshot items carry `sequence` 0.
- `Control` items carry no market symbol.

Send a snapshot when any of these is true:

- `after_sequence` is 0.
- The request's `engine_epoch` doesn't match the current epoch.
- `after_sequence` is older than the retained history.

Otherwise, replay from `after_sequence + 1`.

To send a snapshot, follow these steps:

1. Set `N` to the current sequence.
2. Send `SNAPSHOT_BEGIN`.
3. Send one `RESTATED` `ExecutionReport` per working order, including `type` and
   `time_in_force`.
4. Buffer everything that happens during the snapshot. Don't interleave it with
   the snapshot.
5. Send `SNAPSHOT_END` with `as_of_sequence` set to `N`.
6. Send the buffered items, then continue live.

- If the buffer limit is reached, drop the subscriber and let it reconnect.
  Never block matching.
- Allow multiple subscribers, each with its own position.
- Document how much history the engine retains.

### Market data stream

- Use a separate connection with its own `md_sequence`.
- On connect, send one `BookSnapshot` per market, then deltas.
- A market's snapshot must come before any delta for that market.
- Send one delta per affected price level. Quantity 0 removes the level.
- Drop slow subscribers freely.
- Don't send trades on this stream.

### What the engine never does

- Never touch money, hold a balance, or check whether a user can pay.
- Never change market configuration on its own.
- Never decide a fee rate.
- Never create an `order_id`.
- Never enforce a minimum, a step, or a market status.

Expiry belongs to the engine; minimums don't. The rule: the engine owns what
depends on the book or on its own clock.

## Common mistakes

- Deciding anything from the order book or from trades read off the stream.
- Writing order status or fills from a command response.
- Forgetting to refund the reservation on a synchronous rejection.
- Retrying a command after the epoch changed.
- Advancing the cursor before the TigerBeetle request succeeds.
- Advancing the cursor in a different Postgres transaction from the batch's
  trades and order updates.
- Changing the transfer ID formula after deployment.
- Reading `remaining_base_quantity` on a market buy.
- Storing `md_sequence`.
- Rounding to the nearest value.
- Reading fee rates again for an order that's already placed.
- Assuming the engine checks minimums or steps.
- Calling `unwrap()` on an enum conversion.
- Using a float, even only for display.

## Changing the contract

- Never change or reuse a field number. Mark a removed field `reserved`, with
  its name.
- Only add fields and enum values. Never repurpose an existing one.
- Every enum keeps its `_UNSPECIFIED = 0` value.
- Create a `trading.v2` package only for a change that can't be made by adding
  fields.
