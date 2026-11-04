# Engine data model

This is the design for issue #21 (TE-2). It brings the engine's types and matching in line with the trading contract merged in #19. It goes layer by layer, from single values up to the events the engine sends out.

**Work plan:** [SLICES.md](SLICES.md) cuts this design into small slices you can build and test one at a time. Each slice links back to the sections here that it depends on.

**Updated for the contract remake (#53, PR #56, `aefaeca`):** `engine_epoch`, snapshots, `Control` and `EXEC_TYPE_RESTATED` are removed; `REJECT_REASON_INTERNAL_ERROR` is added; the engine's books, command cache, event history and `sequence` live in Redis, and nothing is sent before it is saved; one instance owns at most 8 markets. Rows changed by it are marked **#53**. Summary: [SPRINT.md](SPRINT.md#what-the-contract-remake-changed).

## How to read this

The contract has two parts. This document keeps them apart.

- **The proto files.** These are `proto/common.proto`, `proto/service.proto` and `proto/events.proto`. They fix the shape of every value the engine receives or sends. Every engine type below names the message, field or enum it comes from.
- **`proto/how_to_use.md`.** This holds the behaviour rules: validation order, matching, fees, the order events go out in. It is cited where a rule shapes a type.

Every row carries one status marker:

- **proto**: the shape is fixed by a `.proto` file. Cited as `file:Message.field`.
- **rule**: the behaviour is fixed by `how_to_use.md`.
- **decided**: an engine-internal choice that is settled.
- **proposed**: a recommendation for #21, not yet agreed.
- **open**: the contract is silent or unclear. It needs an answer before coding.
- **removed**: it was in the previous revision, and the contract replaces it.

One rule holds everywhere (**rule**, the "Never" list). The engine never touches money, never holds a balance, never decides a fee rate, never invents an `order_id`, and never enforces a minimum, a step or a market status.

**The running example.** Most examples use the BTC-USD market:

- **Base** is BTC, counted in satoshis. `base_decimals = 8`, so 1 BTC is 100,000,000.
- **Quote** is USD, counted in cents. `quote_decimals = 2`, so $1 is 100.
- **Price** is cents per BTC. `price_decimals = 2`, so $29,000 is `2_900_000`.

### Scope of #21

#21 covers types and matching rules, as a library, with no networking.

These are outside #21: the `TradingEngine` gRPC service, `SubscribeEvents`, `SubscribeMarketData`, snapshots, `engine_epoch`, stop orders and the GTD timer. Their messages are still described here, because they shape the types #21 builds. Each one is marked out of scope.

---

## Wire conventions (proto)

These apply to every message below. "Wire" means the bytes that travel between the backend and the engine.

| Convention | Source | What it means for the engine |
|---|---|---|
| A 128-bit value is a message `{ fixed64 high; fixed64 low }`, and `value = (high << 64) \| low`. | `common.proto:Uint128`, `common.proto:Id` | There are two wire types: `Uint128` for amounts and prices, `Id` for identifiers. The engine keeps them as separate Rust newtypes over `u128`, so an amount can never be used as an id. A newtype is a struct that wraps one value to give it its own type. |
| Message fields can be absent. Scalar fields cannot. | proto3 | A `Uint128`, `Id` or `Timestamp` field can be missing, and missing is not zero. A `uint32`, `uint64` or `string` field is never missing. Unset reads as `0` or `""`. |
| Every enum has `_UNSPECIFIED = 0`. | `common.proto` header comment | An unset enum field reads as `UNSPECIFIED`. It must become a rejection, never a default variant. |
| Enums arrive as `i32`, because prost (the Rust protobuf library) does it that way. | `how_to_use.md`, "Worth knowing" | An unknown number fails when converted, not when decoded. Every conversion needs a fallback arm. |
| An unknown `oneof` arm arrives as `None`. A `oneof` is a field that holds exactly one of several choices. | `how_to_use.md` | A request with no amount set is `BAD_ORDER_PARAMS`, not a panic. |
| Timestamps are `google.protobuf.Timestamp`: `int64 seconds` plus `int32 nanos`. | `service.proto`, `events.proto` | **decided** (changed in the PR #31 review): no engine type of its own. The engine uses `std::time::SystemTime` and the conversions `prost-types` already has (`SystemTime::try_from(wire)`, `Timestamp::from(system_time)`). `wire.rs` only maps `TimestampError` to `EngineError::TimestampOutOfRange`. Two consequences: times before 1970 are accepted, and nanos of a whole second or more are carried into the seconds instead of rejected. If GTD needs either rejection, check it where `expire_time` is used. |
| Field numbers freeze. | `how_to_use.md`, "Field numbers freeze" | Not about engine types, but no engine type may assume a field will be reused for something else. |

**An example of absent versus zero.** A `PlaceOrderRequest` with no `limit_price` is a market order, or a mistake. A `limit_price` of `0` is a bad price. The engine must tell them apart, so it checks for absence first and zero second.

---

## Layer 0: values

These are the single numbers the engine works with. Each has its own Rust type, so the compiler stops you from mixing them up.

| Engine type | Wire source | Status | Notes |
|---|---|---|---|
| `Price` | `Uint128`: `PlaceOrderRequest.limit_price`, `.trigger_price`, `.protection_price`; `ModifyOrderRequest.new_limit_price`, `.new_trigger_price`; `TradeSettlement.price`; `ExecutionReport.limit_price`; `BookDelta.price`; `PriceLevel.price` | **proto** | Quote minor units per base unit, at `Market.price_decimals`. Must be above zero (**rule**, else `BAD_PRICE`). Still the key of the book's `BTreeMap`. |
| `BaseQuantity` | `Uint128`: `PlaceOrderRequest.amount.base_quantity`, `ModifyOrderRequest.new_base_quantity`, `TradeSettlement.base_quantity`, `ExecutionReport.*_base_quantity`, `BookDelta.quantity`, `PriceLevel.quantity` | **proto** | At `Market.base_decimals`. Must be above zero on requests (**rule**, else `BAD_QUANTITY`). |
| `QuoteQuantity` | `Uint128`: `PlaceOrderRequest.amount.quote_quantity`, `TradeSettlement.quote_quantity`, `.buyer_fee`, `.seller_fee`, `ExecutionReport.*_quote_quantity` | **proto** | At `Market.quote_decimals`. Fees are in quote: both `buyer_fee` and `seller_fee` come out of the quote account. **decided:** a separate type from `BaseQuantity`. |
| `FeeBps` | `uint32`: `PlaceOrderRequest.maker_fee_bps`, `.taker_fee_bps` | **proto** | A basis point (bps) is one hundredth of a percent: `1 bps = 0.01%`. It is a scalar, so unset reads as `0`, which is also a legal rate. The engine cannot tell "forgot to send" from "free". **open:** reject anything above `10_000` as `BAD_ORDER_PARAMS`, or trust the backend? |
| `SystemTime` (std) | `PlaceOrderRequest.expire_time`, `.sent_at`; `EventItem.engine_time`; `MarketDataItem.engine_time` | **proto** | No engine newtype, see Wire conventions. `expire_time` drives GTD. `sent_at` is only for measuring latency. `engine_time` is stamped when an event goes out, which is out of #21. |
| `Money`, `SCALE`, `ONE` | none | **removed** | There are no signed amounts and no crate-wide scale. Scale comes from each `Market`. |
| `Tick`, `Lot`, `CoinSpec` | none | **removed** | `Market` has no step or minimum fields. Those live in the backend's table and "never go on the wire". |

### Quote math (rule)

```
k     = price_decimals + base_decimals - quote_decimals
quote = floor(price * base_qty / 10^k)        // 256-bit intermediate
fee   = floor(quote * rate_bps / 10_000)
```

Always round down. The sum of all fills must fit inside the money the backend reserved. Rounding up could charge a cent more than was reserved.

**An example.** Alice buys half a bitcoin from Bob at $29,000. Alice is the taker and pays 10 bps. Bob is the maker and pays 5 bps.

```
k          = 2 + 8 - 2                           = 8
quote      = 2_900_000 * 50_000_000 / 10^8       = 1_450_000   ($14,500.00)
buyer_fee  = 1_450_000 * 10 / 10_000             = 1_450       ($14.50, Alice)
seller_fee = 1_450_000 * 5  / 10_000             = 725         ($7.25, Bob)
```

**An example of rounding down.** Alice buys 12,345 satoshis at $29,000:

```
quote = 2_900_000 * 12_345 / 10^8 = 358.005   ->  358 cents
```

The 0.005 of a cent is dropped. It is never rounded up to 359.

**Why 256 bits.** `price * base_qty` can be far bigger than a `u128` can hold, even when the final answer after dividing by `10^k` fits easily. Multiplying in plain `u128` would overflow, which means the number wraps around or panics. So the multiply keeps a 256-bit result, and the divide brings it back down. **decided** (PR #31 review): the 256-bit math uses the `primitive-types` crate (`U256`) instead of a hand-written multiply and long division. Two `u128`s multiplied always fit in 256 bits, so the multiply itself cannot overflow; only the final result can fail to fit back into a `u128`.

The divide has to come last. Dividing first would round small amounts to zero: 123 satoshis divided by 10^8 is 0 in integer math.

### Implemented

Layer 0 is in the code:

| Type | Module |
|---|---|
| `Price` | `src/price.rs` |
| `BaseQuantity`, `QuoteQuantity` | `src/quantity.rs` |
| `FeeBps` and `fee_on` | `src/fee.rs` |
| `QuoteScale` and `quote_for` | `src/quote.rs` |
| 256-bit `mul_div_floor`, via `primitive_types::U256` | `src/wide.rs`, private to the crate |
| Wire conversions: `required()`, `Uint128` to `Price` and quantities, `Market` to `QuoteScale`, `TimestampError` to `EngineError` | `src/wire.rs` |

Timestamps have no module any more (`src/timestamp.rs` was removed). The `TryFrom<Option<_>>` impls in `wire.rs` were removed too: callers write the full `Price::try_from(required(field)?)?`.

`QuoteScale` holds `10^k` for one market. Its constructor returns `DecimalsOutOfRange` if `k` is negative or above 38 (`QuoteScale::MAX_EXPONENT`). 38 because `10^38` is the largest power of 10 that fits in a `u128`; the limit is on the divisor, not on the multiply. That is the proposed `BAD_DEFINITION` check from Layer 3. The `FeeBps > 10_000` question is still open, so `FeeBps` accepts any `u32`.

---

## Layer 1: identity

These are the names and numbers that say which market, order, user or account something belongs to.

| Engine type | Wire source | Status | Notes |
|---|---|---|---|
| `MarketSymbol` | `string`: `Market.symbol`, `PlaceOrderRequest.market_symbol`, `CancelOrdersRequest.market_symbol`, `EventItem.market_symbol`, `MarketDataItem.market_symbol` | **proto** | For example `BTC-USD`. Replaces `CoinId`. It is a scalar, so unset reads as `""`, which must be `UNKNOWN_MARKET`. |
| `OrderId` | `Id`: `PlaceOrderRequest.order_id`, cancel and modify requests, `ExecutionReport.order_id`, `TradeSettlement.buyer_order_id` and `.seller_order_id` | **proto** | 128 bits. **Assigned by the backend** (**rule**). The engine never invents one. |
| `CommandId` | `Id`: `.command_id` on every request and every trading response; `ExecutionReport.command_id` | **proto** | 128 bits. Replaces `IdempotencyKey`. Sent back on every response. |
| `AccountHolder` | `Id`: `PlaceOrderRequest.account_holder`, cancel, modify and cancel-orders requests, `ExecutionReport.account_holder`, `TradeSettlement.*_account_holder` | **proto** | The user. The proto comment on `PlaceOrderRequest` gives its purpose: "to detect self-trading". Also used to check ownership on cancel and modify. Replaces `UserId`. |
| `AccountId` | `Id`: `PlaceOrderRequest.base_account`, `.quote_account`; `Market.fee_account`; `TradeSettlement.*_base_account`, `*_quote_account`, `.fee_account` | **proto** | TigerBeetle accounts. TigerBeetle is the ledger database that moves the money. The proto comment says "you just store them and send them back in the TradeSettlement". The engine never reads them. |
| `TradeId` | `Id`: `TradeSettlement.trade_id`, `ExecutionReport.trade_id` | **proto** | **Assigned by the engine**, one per match. **rule (#53):** a time-ordered unique id (`tb::id()` or UUIDv7), so it never collides across instances, strictly increasing in trade order within one instance (UUIDv7 needs a counter inside one millisecond). The backend orders trades with the same `engine_time` by it and derives its TigerBeetle transfer ids from it. The earlier epoch-in-the-high-bits proposal is dropped. After a restart it continues past the last saved id. |
| `Sequence` | `uint64`: `EventItem.sequence` | **proto**, out of #21 | **#53:** one counter per engine instance (not per market), saved in Redis, continuing across restarts, 1 on the very first start. It numbers the history; nobody resumes from it. See Layer 8. |
| `EngineEpoch` | none | **removed** (#53) | Removed from `EventItem`, `MarketDataItem` and `SubscribeEventsRequest`. |
| `MdSequence` | `uint64`: `MarketDataItem.md_sequence` | **proto**, out of #21 | A separate counter, only for spotting gaps. |
| `SeqNo` (per coin) | none | **removed** | Time priority comes from an order's place in the FIFO queue at its level. |

**Two ids, two owners.** When Alice places an order, the backend picks both the `order_id` and the `command_id`. They are different values. If the order trades, the engine picks the `trade_id`.

**Status of `src/ids.rs`** (after the PR #31 review): `OrderId`, `TradeId` and `CommandId` are `u128` newtypes, like the wire `Id`. `CommandId` replaced the string `IdempotencyKey`; any `u128` is valid, so it has no validation and no error of its own. `CoinId` is removed. Still to do: `UserId` is not yet renamed to `AccountHolder`, there is no `AccountId`, and `SeqNo` is still declared but unused, so it can be deleted. `wire::required_id` still returns a raw `u128` instead of a typed id.

**Two kinds of duplicate.** A duplicate `order_id` is `EngineError::DuplicateOrderId`, from `Book::insert`. **open:** `RejectReason` has no matching reason. Either it cannot happen (the backend always generates a fresh id) and maps to `UNSUPPORTED`, or the contract needs `DUPLICATE_ORDER`. A duplicate `command_id` is not an error by itself: the same request replays the stored response, and only a different request is `DUPLICATE_COMMAND` (Layer 4). That needs the command cache, which is not built yet.

---

## Layer 2: enums (all proto, from `common.proto`)

Each enum keeps its wire meaning. `UNSPECIFIED` is never a valid engine value. So the engine enums below leave it out, and converting from `i32` returns a rejection for it.

| Enum | Variants | Notes from the proto comments |
|---|---|---|
| `Side` | `Buy`, `Sell` | |
| `OrderType` | `Limit`, `Market`, `StopLimit`, `StopMarket` | Market means "market price, no resting". StopLimit and StopMarket "become a limit / market when triggered". |
| `TimeInForce` | `Gtc`, `Gtd`, `Ioc`, `Fok` | The comment calls GTC "the default". But on the wire, unset is `UNSPECIFIED`, not GTC. **proposed:** reject it rather than default silently. FIX's Day is left out on purpose. |
| `OrderStatus` | `New`, `PartiallyFilled`, `Filled`, `Cancelled`, `Expired`, `PendingTrigger` | Expired is a "GTD order reaching expiration". It is its own final state. There is no `Open` and no `Rejected`. |
| `ExecType` | `New`, `Triggered`, `Trade`, `Cancelled`, `Expired`, `Replaced` | **#53:** `Restated` is removed with the snapshots. |
| `RejectReason` | `UnknownMarket`, `BadQuantity`, `BadPrice`, `BadOrderParams`, `UnknownOrder`, `OrderNotWorking`, `NotOrderOwner`, `DuplicateCommand`, `Unsupported`, `InternalError` | "Only returned synchronously", meaning in the direct reply. `UNSPECIFIED` means accepted. So in Rust the response holds `Result<Accepted, RejectReason>`, not an enum with an "ok" variant. **#53:** `InternalError` is only for a command still queued when the engine panicked or got SIGTERM, so it never ran. It is not a mapping target for `EngineError`, and it is never cached. |
| `CancelReason` | `UserRequest`, `IocRemainder`, `IocNoFill`, `FokNoFullFill`, `SelfTrade`, `ProtectionPrice`, `Admin` | "Commands are valid and an order exists, but it's terminated." |
| `TriggerDirection` | `LastAtOrAbove`, `LastAtOrBelow` | Stop orders only. Out of #21. |
| `ConfigRejectReason` | `BadDefinition`, `HasOpenOrders` | `SetMarket` only. `UNSPECIFIED` means accepted. |

The time-in-force codes:

- **GTC** (good till cancelled): the leftover rests in the book until filled or cancelled.
- **GTD** (good till date): the leftover rests until filled or until `expire_time`.
- **IOC** (immediate or cancel): fill what you can now, and cancel the rest.
- **FOK** (fill or kill): fill the whole order now, or fill nothing.

**Mapping from #20.** The old `OrderStatus` loses `Open` (it is now `New`) and `Rejected` (a rejection creates no order). `TimeInForce` gains `Gtd`.

**Changing under #21.** Today `src/order.rs` still holds `OrderKind { Market, Limit { price } }`, `TimeInForce { GoodTilCancelled, ImmediateOrCancel, FillOrKill }` and an `OrderStatus` with `New`, `Open` and `Rejected`. `OrderKind` and `TimeInForce` are only used by tests so far, and `OrderStatus::New` is never set: a new `RestingOrder` starts as `Open`. A limit order fills at its price **or better**, because a fill happens at the maker's price.

---

## Layer 3: the market

A market is one trading pair, such as BTC-USD. It tells the engine how many decimals each number has. `common.proto:Market` is **proto**, and this is all of it:

| Field | Type | Notes |
|---|---|---|
| `symbol` | `string` | The key. |
| `base_decimals` | `uint32` | |
| `quote_decimals` | `uint32` | |
| `price_decimals` | `uint32` | The proto comment: "how many decimals you need to write down the exchange rate". |
| `fee_account` | `Id` | Copied into every `TradeSettlement.fee_account`. |

There is no status, no minimum and no step. The proto comment says the rest is "backend config", kept in Postgres.

Rules:

- **rule:** The engine holds no markets at startup. Every command gets `UNKNOWN_MARKET` until `SetMarket` pushes one.
- **proto:** `SetMarketResponse { reject_reason: ConfigRejectReason, reject_detail }`.
- **proto:** `BAD_DEFINITION` covers things like "decimals out of range". **open:** define the range. If `quote_decimals > price_decimals + base_decimals`, then `k` goes negative and the quote formula turns into a multiply. **proposed:** reject that case, and cap the decimals so `10^k` fits in 128 bits. `QuoteScale::new` already does both checks (Layer 0).
- **proto:** `HAS_OPEN_ORDERS` is for "structural changes when the book is not empty". **proposed:** structural means any change to the decimals. A `fee_account` change applies from the next trade.

**An example of a negative `k`.** A market with `price_decimals = 0`, `base_decimals = 2`, `quote_decimals = 6` gives `k = 0 + 2 - 6 = -4`. The formula would need to multiply by `10^4` instead of dividing. The engine does not support that, so `QuoteScale::new` rejects it.

---

## Layer 4: commands

A command is one request from the backend: set a market, place, cancel, cancel many, or modify.

### Requests (proto, from `service.proto`)

A `?` marks a message field that can be absent. Scalars are always present.

```
SetMarketRequest    { market: Market? }

PlaceOrderRequest   { command_id: Id?, order_id: Id?, market_symbol: string,
                      account_holder: Id?, base_account: Id?, quote_account: Id?,
                      side, type: OrderType, time_in_force,
                      amount: oneof { base_quantity: Uint128 | quote_quantity: Uint128 }?,
                      limit_price: Uint128?, trigger_price: Uint128?,
                      trigger_direction, protection_price: Uint128?,
                      expire_time: Timestamp?, sent_at: Timestamp?,
                      maker_fee_bps: u32, taker_fee_bps: u32 }

CancelOrderRequest  { command_id: Id?, order_id: Id?, account_holder: Id? }

CancelOrdersRequest { command_id: Id?, market_symbol: string,
                      account_holder: Id? }      // absent = every holder in the market

ModifyOrderRequest  { command_id: Id?, order_id: Id?, account_holder: Id?,
                      new_base_quantity: Uint128?,  // absent = unchanged
                      new_limit_price: Uint128?,
                      new_trigger_price: Uint128? }
```

Things only the proto shows:

- **Cancel and modify carry no `market_symbol`.** The engine must find the market from the `order_id` alone. So it needs one engine-wide index from order to market (Layer 6).
- **`CancelOrdersRequest.account_holder` is the one `Id` where absent means something.** Absent means "cancel every holder's orders in this market". It must not be treated as a missing field.
- **A missing required `Id` has no reject reason of its own.** For example, a place with no `order_id`. **proposed:** `BAD_ORDER_PARAMS`. A missing `command_id` cannot be checked for duplicates at all, so reject it before looking in the cache.
- **A modify with all three `new_*` fields absent is legal on the wire.** **open:** is it a no-op that returns the current status, or `BAD_ORDER_PARAMS`?
- **`trigger_direction` is a scalar enum.** "Present" here means "not `UNSPECIFIED`".

### Parsed command (decided)

After validation, a request is turned into engine types. These types make a wrong combination of fields impossible to build. The wire format stays flat.

```
Command   = SetMarket(Market)
          | Place(PlaceOrder)
          | Cancel   { command_id, order_id, account_holder }
          | CancelAll{ command_id, market, account_holder: Option<AccountHolder> }
          | Modify   { command_id, order_id, account_holder,
                       new_base_quantity: Option<BaseQuantity>,
                       new_limit_price:   Option<Price> }   // new_trigger_price -> UNSUPPORTED in #21

OrderKind = Limit  { price: Price }
          | Market { protection_price: Option<Price> }
          // StopLimit / StopMarket -> UNSUPPORTED in #21

Amount    = Base(BaseQuantity)      // limits, and all market sells
          | Quote(QuoteQuantity)    // market buy only

Lifetime  = Gtc | Gtd { expire_time: Timestamp } | Ioc | Fok
```

**Why.** On the wire, a limit order with no `limit_price` is possible. In `OrderKind`, a `Limit` without a `price` does not compile. The check happens once, at parse time, and the rest of the engine never has to repeat it.

### Responses (proto, from `service.proto`)

Three responses share one shape. `CancelOrdersResponse` has its own.

```
PlaceOrderResponse / CancelOrderResponse / ModifyOrderResponse
  { command_id, reject_reason, reject_detail,
    status, cancel_reason,                    // only when accepted
    filled_base_quantity, filled_quote_quantity }

CancelOrdersResponse
  { command_id, reject_reason, reject_detail, cancelled_count: u32 }
```

The proto comment on `PlaceOrderResponse` says `status` and the fills are set "after you try to match the order for the first time". They are "ONLY FOR THE IMMEDIATE REPLY TO USER". Money only moves on `TradeSettlement`.

**decided:** inside the engine, one `CommandResponse` type:

```
CommandResponse = Rejected { reason: RejectReason, detail: String }
                | Order    { status, cancel_reason: Option<CancelReason>,
                             filled_base: BaseQuantity, filled_quote: QuoteQuantity }
                | Bulk     { cancelled_count: u32 }
```

This is the value the command cache stores.

### Duplicate detection: store the request, no hash

**decided:** the engine does not hash requests. It stores the request itself and compares it.

**What the contract requires** (**rule**, and a #21 engine test):

- A known `command_id` with the same contents replays the stored response.
- A known `command_id` with different contents is `DUPLICATE_COMMAND`.

**An example.** The backend sends command `42`: Alice buys 0.5 BTC at $29,000. The engine places it and answers `New`.

- The network drops the answer, so the backend retries command `42` with the same contents. The engine replays `New`. It does not place a second order.
- A bug makes the backend reuse `42` for "Alice sells 0.5 BTC". The engine answers `DUPLICATE_COMMAND`.

The comparison is needed. Without it, the sell would get the stored answer for the buy. The backend would then believe an order was placed, cancelled or rejected when the engine never ran it.

**Why no hash.** `how_to_use.md` calls it a "request hash". But no proto message carries a hash, and the backend never sends one. The engine would compute it only to compare it. Comparing the request directly does the same job better:

- **It is exact.** Two different requests can share a hash. That is called a collision, and it would replay the wrong answer.
- **Nothing to choose.** There is no hash algorithm or byte format to agree on. Protobuf does not promise identical bytes from two encoders anyway.
- **The cost is small.** Each cache entry holds a few hundred bytes instead of a 32-byte digest. The cache is in memory and bounded by its retention window.

**What is stored.** The request as received, with `command_id` and `sent_at` cleared. It sits in a `CommandRequest` enum with one variant per request kind. Comparing is plain equality. Storing the parsed command is not enough. A rejected request never becomes a parsed command, and its rejection is cached too.

- **The request kind is part of the value.** A place and a cancel that reuse one `command_id` never compare equal.
- **Absent compares as absent.** An absent `CancelOrdersRequest.account_holder` differs from any present one.
- **`sent_at` is left out.** A backend retry may restamp it. Including it would turn a legitimate retry into `DUPLICATE_COMMAND`. In the example above, the retry of `42` might carry a `sent_at` one second later, and it must still replay.

**open:** the contract should state the `sent_at` rule. Either it is ignored for duplicate detection, or the backend promises to resend identical requests. Otherwise each side can follow its own checklist and still disagree.

`SetMarket` has no `command_id`, so it is not checked for duplicates. Pushing the same market twice is idempotent anyway, meaning the second push changes nothing.

### Validation (rule, mapped to the proto fields)

Checks for a place, in this order. For any failure here, no order is created and nothing goes on the stream.

| Check | Fields | Reject reason |
|---|---|---|
| Market exists | `market_symbol` | `UNKNOWN_MARKET` |
| Right amount arm: quote for a market or stop-market buy, base otherwise | `amount`, `type`, `side` | `BAD_ORDER_PARAMS` |
| Quantity above zero | `amount.*` | `BAD_QUANTITY` |
| Every present price above zero | `limit_price`, `trigger_price`, `protection_price` | `BAD_PRICE` |
| `limit_price` present only for `LIMIT` and `STOP_LIMIT`, and always for them | `limit_price`, `type` | `BAD_ORDER_PARAMS` |
| `trigger_price` and `trigger_direction` present only for stops, and always for them | | `BAD_ORDER_PARAMS` |
| `expire_time` present only for `GTD`, always for it, and in the future | `expire_time`, `time_in_force` | `BAD_ORDER_PARAMS` |
| `protection_price` only on market orders | | `BAD_ORDER_PARAMS` |
| Quote result fits in 128 bits after the division | `amount`, `limit_price`, market decimals | `BAD_QUANTITY` |
| Feature is built at this stage | `type`, `trigger_*` | `UNSUPPORTED` |
| **proposed:** any enum `UNSPECIFIED` or unknown, any required `Id` absent | | `BAD_ORDER_PARAMS` |

**An example.** Alice sends a market buy with the `base_quantity` arm set to 0.5 BTC. A market buy must say how much money to spend, not how much BTC to get. So the engine rejects it with `BAD_ORDER_PARAMS`.

In #21, `UNSUPPORTED` covers stop orders and `new_trigger_price`. GTD is accepted and rests, but nothing expires it until the timer ticket lands. **open:** is protection price in #21, or `UNSUPPORTED` as well?

Not checked, on purpose (**rule**): minimum size, minimum notional (the order's total value in quote), quantity step, price step, market status, fee rates, funds. The backend checks these before sending.

Checks for cancel and modify, in this order:

| Check | Reject reason |
|---|---|
| Order exists | `UNKNOWN_ORDER` |
| Not already finished | `ORDER_NOT_WORKING` |
| `account_holder` owns it | `NOT_ORDER_OWNER` |
| Modify: new quantity is above what is already filled | `BAD_QUANTITY` (`EngineError::QuantityNotAboveFilled` in the code) |
| Modify: order is base-denominated | `ORDER_NOT_WORKING` |

### Modify and queue position (rule)

- **Lowering the quantity** keeps the order's place in the queue.
- **Raising the quantity or changing the price** sends it to the back of the queue at the new price.
- **A new price that crosses the book** matches at once. The response may say `FILLED` or `CANCELLED`.

**An example.** The bids at $29,000 are, in order: Alice 0.5 BTC, then Bob 0.3 BTC.

- Alice lowers to 0.4 BTC. She stays first. Bob still waits behind her.
- Alice raises to 0.6 BTC. She moves behind Bob. Otherwise she could jump the queue by first placing a tiny order and then growing it.
- Alice changes her price to $29,100 while the best ask is $29,050. Her order crosses and trades straight away.

**Implemented** (PR #31): `Book::modify(id, new_quantity: Option<BaseQuantity>, new_price: Option<Price>) -> Result<Execution>` replaced `amend`. `amend` swapped in a whole replacement order, possibly with a new id, and always sent it to the back.

- **Same price and a lower or equal total:** changed in place, so the order keeps its place. No matching happens.
- **A higher total or a new price:** remove, then insert. The order goes to the back and can cross the book.
- **Sending the same price explicitly** counts as no price change.
- **`new_quantity` is the order's new total, not its remaining quantity.** `RestingOrder::modify` rejects a total that is not above what is already filled, and sets remaining to the new total minus filled. A finished order is rejected with `OrderNotLive`.
- **A rejected modify leaves the book unchanged.** On the remove-and-insert path, the change is checked on a copy first.
- **All `new_*` fields absent:** currently a no-op that keeps the place and returns the order as resting. That answers the Layer 4 open question for now; the service may still choose `BAD_ORDER_PARAMS`.

---

## Layer 5: the order

### `RestingOrder` (proposed)

A resting order is one that sits in the book, waiting for a match. Only a limit order with a base amount and a GTC or GTD leftover ever rests. Market, IOC and FOK orders never become a `RestingOrder`.

Every field exists because a proto message needs it later:

| Field | Needed by |
|---|---|
| `order_id` | `ExecutionReport.order_id`, `TradeSettlement.*_order_id`, finding the order for cancel and modify |
| `command_id` | `ExecutionReport.command_id`. **open:** the placing command's id forever, or the id of whichever command caused each change? FIX does the second. |
| `account_holder` | `ExecutionReport.account_holder`, `TradeSettlement.*_account_holder`, self-trade and ownership checks |
| `base_account`, `quote_account` | `TradeSettlement.*_base_account`, `*_quote_account` |
| `side` | `ExecutionReport.side`, `TradeSettlement.taker_side` |
| `order_type` | `ExecutionReport.type`. Always `Limit` while resting in #21, but every report carries it. |
| `lifetime` | `ExecutionReport.time_in_force`, GTD expiry |
| `limit_price` | `ExecutionReport.limit_price` and the level key. A modify can change it. |
| `order_base_quantity` | `ExecutionReport.order_amount.order_base_quantity`. A modify can change it. |
| `remaining_base_quantity` | `ExecutionReport.remaining_amount.remaining_base_quantity`. Matching walks it down. |
| `filled_base_quantity`, `filled_quote_quantity` | `ExecutionReport.filled_*`. Running totals. The backend overwrites its copy rather than adding to it. |
| `maker_fee_bps`, `taker_fee_bps` | `TradeSettlement.buyer_fee` and `.seller_fee`. Fixed for the order's whole life. |
| `status` | `ExecutionReport.status`: `New` or `PartiallyFilled` while resting |

The market symbol is not a field. The order lives in its market's book, and the event envelope carries `market_symbol`.

Dropped from the #20 version: `user_id`, `idempotency_key`, `seq_no`.

**`command_id` probably does not belong here.** It names one request, not the order. A modify or cancel arrives with a new `command_id` for the same `order_id`, so a stored value goes stale. Duplicate detection belongs to the command cache. The `ExecutionReport.command_id` question above decides whether the order needs to remember any command id at all.

**Why fees are pinned.** Alice places a GTC order while the maker rate is 5 bps. An admin later raises it to 8 bps. Alice's order still pays 5 bps on every fill, because the backend reserved her money at the old rate.

**Status of `src/resting_order.rs`** (after PR #31): `id`, `user_id`, `side`, `price`, `qty_original`, `qty_remaining`, `status` and `command_id` (`u128`). `seq_no` and `idempotency_key` are gone. A new order starts as `OrderStatus::Open`. Methods: `new`, `fill`, `modify`, `cancel`. **Naming:** after a modify, `qty_original` holds the new total, not the original, so it should be renamed (for example `qty_total`). The table above is what the struct becomes.

### Incoming order

While it matches, the incoming order needs the same fields plus its `Amount`. A market buy is quote-denominated, so its `ExecutionReport` uses the `order_quote_quantity` and `remaining_quote_quantity` arms. `ExecutionReport.limit_price` is absent for market orders.

---

## Layer 6: engine state

This is what the engine keeps in memory between commands.

| Structure | Status | Notes |
|---|---|---|
| `BookSide = BTreeMap<Price, VecDeque<RestingOrder>>` | **decided** | Bids best-first from the highest price, asks best-first from the lowest. Within a level, orders are FIFO: first in, first out, so the oldest order fills first. |
| `Book { bids, asks, all_orders: HashMap<OrderId, (Side, Price)> }` | **decided** | One per market. `all_orders` is an index: it lets `remove`, `get_by_id` and `modify` find an order's level without walking a whole side, and lets `insert` reject a duplicate id. |
| `markets: HashMap<MarketSymbol, (Market, Book)>` | **proposed** | Empty at startup. |
| `order_index: HashMap<OrderId, MarketSymbol>` | **proposed, needed** | Because cancel and modify requests carry no `market_symbol`. |
| `CommandCache: HashMap<CommandId, (CommandRequest, CommandResponse)>` | **rule** | Same id and equal request: replay the stored response. Same id and different request: `DUPLICATE_COMMAND`. No hash (see Layer 4). **#53:** saved to Redis with the rest of the state, so a retry after a restart gets the same answer. `INTERNAL_ERROR` answers are not stored. It must keep entries longer than the backend keeps retrying. One cache for the whole instance, not one per market. |
| Finished orders | **open** | Cancel and modify must tell `UNKNOWN_ORDER` apart from `ORDER_NOT_WORKING`. That needs a record of finished `order_id`s after they leave the book, bounded like the command cache. |

**An example of the finished-orders gap.** Alice's order fills and leaves the book. She then tries to cancel it. The right answer is `ORDER_NOT_WORKING`. But if the engine has forgotten the order, it can only say `UNKNOWN_ORDER`.

**Sharding.** Sharding means splitting the work across threads or processes. The previous revision ran one single-threaded actor per coin. #21 is a library, so one engine value holding every market is enough. The service ticket must revisit sharding with three engine-wide things in mind: `EventItem.sequence`, the command cache and the order index.

### One thread per market (proposed)

**proposed** (the team's plan, still to be written down in an issue): each market runs on its own thread, as an actor. An actor is a thread that owns its data alone and only receives work as messages through a channel, so it needs no locks.

```
 gRPC handlers ──► Router ──────mpsc───► BTC-USD thread  (owns its Book)
                   - order_index  ─────► ETH-USD thread  (owns its Book)
                   - command cache          │        │
                                            ▼ events ▼
                                       Sequencer  (stamps sequence, one counter)
                                            │
                                            ▼
                                     SubscribeEvents stream
```

| Part | Owns | Why it is there |
|---|---|---|
| Market thread, one per market | its `Market`, `QuoteScale` and `Book` | Books are independent, so they can match in parallel. One command at a time per market keeps matching deterministic. |
| Router | `order_index`, `CommandCache` | Cancel and modify carry no `market_symbol`, so something outside the markets must know which thread owns an order. `command_id` is unique engine-wide, so the cache cannot be split per market. |
| Sequencer | the `sequence` counter | `EventItem.sequence` is one counter for the whole engine. Only one place may hand out numbers. Each market's events arrive in their own order, and the sequencer fixes the order across markets. |

**Rules this needs:**

- **Per-market code takes no global state.** Place, cancel and modify on one market are methods on `MarketState` that need nothing outside it. Then moving a `MarketState` onto its own thread is plumbing, not a rewrite.
- **The router updates `order_index` from the market's answer.** It adds an order when it rests and removes it when it fills, is cancelled or expires. A market thread never writes the index itself.
- **Back-pressure.** A slow subscriber must not block matching: drop it and let it reconnect. The channel from market tasks to the sequencer must not stall a market.
- **Write before send (#53).** A market task takes a batch of queued commands, runs them, saves the result to Redis once, and only then sends the batch's responses and events. Redis is on the path of every response.
- **Shutdown (#53).** On SIGTERM (or a panic): stop taking commands, answer every queued command with `INTERNAL_ERROR` (not cached), save, exit.

**rule (#53): at most 8 markets per instance.** The engine runs as several instances (pods, `StatefulSet` still TBD in `proto/devops.md`), each owning up to 8 markets, every market owned by exactly one. The pod name is the instance's identity and the prefix of its Redis keys. Each market is one task. `SetMarket` spawns it.

Settled by #53, replacing the earlier open points:

- **Who owns which market.** A separate Router service holds the map of markets to instances and forwards each command. The backend and the event consumer only talk to the Router. Inside the engine, call the symbol-to-task part the *dispatcher*, so it doesn't get mixed up with that Router.
- **Silent markets.** Dropped. Moving a market to another instance isn't covered by the contract, and an instance can be removed only when it owns no markets.
- **Counters.** `sequence` is per instance and continues across restarts. `engine_epoch` is removed. The Router merges the instances' streams; items of one instance keep their order.

Still open: the reject reason for a 9th `SetMarket` (the `ConfigRejectReason` enum has none for "instance full"), and what one Redis save holds (per market or per instance).

The work for this is slice T in [SLICES.md](SLICES.md).

### Errors with no proto meaning (decided)

**decided:** the engine panics and exits on any error the contract does not define. An error that maps to a `RejectReason`, `CancelReason` or `ConfigRejectReason` goes back in the response. Anything else, such as a poisoned lock or a broken internal invariant, crashes the process.

- **Why.** After such an error the book may be half updated. Serving it is worse than restarting: Kubernetes restarts the pod and it loads its state from Redis, so resting orders survive the crash. **#53:** there is no fallback to an empty start any more (no `engine_epoch` to tell the backend); Redis is assumed always available, and the backend does nothing on a restart.
- **Redis must only hold finished work.** A batch is written to Redis only once it has fully applied. Then a panic in the middle of a command leaves nothing half done, and the restart lands on the last saved batch. **Open (#53):** `how_to_use.md` asks the panic handler to write everything in memory to Redis, which would include the half-updated book. See [SPRINT.md, PR B](SPRINT.md#pr-b-multithreading-not-mine).
- **#53, before exiting:** answer every queued command with `INTERNAL_ERROR`. With `panic = "abort"` only a panic hook runs; how it reaches the other market tasks' queues is open.
- **How.** `tradingengine/Cargo.toml` sets `panic = "abort"` for dev and release. Without it, tokio catches a panic inside a request's task, only that request dies, and the pod becomes a zombie that health checks still report as serving. With it, `.expect(...)` and any other panic end the process. Tests ignore the setting and still unwind.
- **Rule for new code.** Never turn an undefined error into a gRPC `Status` to keep the pod alive. When mapping `EngineError` to a reject reason, list the contract cases explicitly and let the fallback arm panic.

First applied in PR #51: `service.rs` uses `.lock().expect("engine lock poisoned")` instead of returning `Status::internal`.

---

## Layer 7: matching

`match(state, command, now) -> (state, CommandResponse, Vec<Event>)` is **decided**. The function is pure: the same inputs always give the same outputs, and it reads nothing from outside. The clock is an argument, because expiry is now the engine's job and GTD validation needs to know what "in the future" means.

| Rule | Source | Notes |
|---|---|---|
| Price, then time | **rule** | The best price fills first. At the same price, the oldest order fills first. |
| Quote per fill, fee per fill, round down | **rule** | One `TradeSettlement` per fill, so rounding happens once per trade. |
| Fee by role in that fill | **rule** | The incoming order pays `taker_fee_bps`. The resting one pays `maker_fee_bps`. `TradeSettlement` names only `taker_side`, so the backend works out which fee was which. |
| Market buy by quote | **proposed** | Base taken at a level is `floor(remaining_quote * 10^k / price)`, capped by what the level holds. The order stops when the remaining quote buys zero base. |
| IOC | **rule** and proto comment | Fill what crosses and cancel the rest as `IOC_REMAINDER`. If nothing filled, `IOC_NO_FILL`. The order is cancelled, not rejected. |
| FOK | **rule** | Check that a full fill is possible **before** touching the book. If not, `FOK_NO_FULL_FILL`, and the book is untouched. The check must skip liquidity from the same owner. |
| Market order on an empty book | **rule** | Admitted, fills nothing, cancelled. **open:** which `cancel_reason`? And is a market order with GTC or GTD `BAD_ORDER_PARAMS`? **proposed:** market orders behave as IOC and use the IOC reasons. |
| Protection price | **rule** | Stop at that price and cancel the leftover as `PROTECTION_PRICE`, possibly after some fills. |
| Self-trade | **rule** | Skip the colliding quantity on the incoming order and leave the resting one alone. If nothing is left to match, admit the order, then cancel it as `SELF_TRADE`. |

Maker and taker: the **maker** is the order that was already resting in the book. The **taker** is the incoming order that hits it.

### Examples

**Fees by role.** Bob's sell of 0.5 BTC at $29,000 rests in the book. Alice's buy arrives and fills it. Alice is the taker and Bob is the maker, so the settlement has `taker_side = Buy`, `buyer_fee = 1_450` and `seller_fee = 725` (see the Layer 0 example).

**One order, two roles.** Alice's GTC buy for 1 BTC at $29,000 fills 0.5 BTC against Bob at once. She pays the taker rate on that half. The other 0.5 BTC rests. Later Carol's sell hits it, and Alice pays the maker rate on that half.

**Market buy by quote.** Bob places a market buy for $1,000 (`100_000` cents). The best ask is $29,000:

```
base = floor(100_000 * 10^8 / 2_900_000) = 3_448_275 satoshis
cost = floor(2_900_000 * 3_448_275 / 10^8) = 99_999 cents  ($999.99)
```

**IOC and FOK.** The asks are 0.1 BTC at $29,000 and 0.1 BTC at $29,100. Alice sends a buy for 0.3 BTC with a limit of $29,100.

- **As IOC:** she fills 0.2 BTC across both levels. The last 0.1 BTC is cancelled as `IOC_REMAINDER`.
- **As FOK:** only 0.2 BTC is available, not 0.3. The engine sees this before it touches anything. The order is cancelled as `FOK_NO_FULL_FILL`, and both asks are still there.

**Self-trade.** The asks at $29,000 are, in order: Alice sells 0.2 BTC, then Bob sells 0.3 BTC. Alice sends a buy for 0.4 BTC at $29,000. She must not trade with herself, so her own 0.2 BTC is skipped and her resting sell is left alone. If only Alice's own sell were in the book, her buy would be admitted and then cancelled as `SELF_TRADE`.

**open: self-trade.** What exactly does "skip" mean? In the example above:

- **Step over** Alice's resting sell and keep matching behind it. Her buy fills 0.3 BTC from Bob, and 0.1 BTC is left.
- **Decrement** the incoming order by the colliding quantity. Her buy shrinks by 0.2 to 0.2 BTC, and fills 0.2 BTC from Bob.

And if a GTC limit steps past its owner's order with some left over, resting that leftover would cross the owner's own order. Should that leftover be cancelled as `SELF_TRADE` instead?

---

## Layer 8: events

Events are what the engine tells the backend after a command. #21 does not stream them, but it must produce them in the right order.

The old `OrderAccepted`, `OrderRejected`, `OrderCancelled`, `TradeExecuted`, `PriceUpdated` and `OrderAmended` are **removed**.

**decided:** matching returns `Event = Settlement(TradeSettlement) | Report(ExecutionReport)`, plus a market-data change. The stream layer wraps each one in its envelope later.

### Envelope (proto, out of #21, from `events.proto:EventItem`)

```
EventItem { sequence: u64, engine_time: Timestamp, market_symbol: string,
            body: oneof { execution_report | trade_settlement } }
```

- **`sequence`** counts every `ExecutionReport` and `TradeSettlement` of one instance, +1 per item, no gaps, continuing across restarts. This replaces the per-coin `SeqNo` decision from issue #6. **#53:** no more 0-sequence snapshot lines.
- **`engine_time` and `market_symbol`** live on the envelope, not in the bodies. So `TradeSettlement` and `ExecutionReport` carry no time or market of their own.
- **`SubscribeEventsRequest {}`** (#53): no resume position and no snapshots. Every subscription replays the whole retained history (kept in Redis, trimmed to a fixed window), then goes live. The consumer is idempotent. Out of #21.

### `TradeSettlement` (proto)

```
trade_id, price, base_quantity, quote_quantity, taker_side,
buyer_order_id,  buyer_account_holder,  buyer_base_account,  buyer_quote_account,  buyer_fee,
seller_order_id, seller_account_holder, seller_base_account, seller_quote_account, seller_fee,
fee_account
```

One per match. The proto comment says to send it before the two execution reports. It carries both order ids and both holders, which settles the previous revision's question of whether to name the taker and maker by order or by user.

### `ExecutionReport` (proto)

```
order_id, command_id, account_holder,
exec_type, status, cancel_reason,
side, type, time_in_force,
order_amount:     oneof { order_base_quantity | order_quote_quantity },
remaining_amount: oneof { remaining_base_quantity | remaining_quote_quantity },
filled_base_quantity, filled_quote_quantity,
limit_price?,                // absent on market orders
trade_id?                    // set only when exec_type == TRADE
```

It carries no fee, no trade price and no counterparty. The backend joins it to the `TradeSettlement` by `trade_id`. The rule says: "No per-side copy of the trade."

| Change | `exec_type` | In #21 |
|---|---|---|
| Accepted or resting | `NEW` | yes |
| Fill | `TRADE`, with `trade_id` set | yes |
| Modify applied | `REPLACED` | yes |
| Cancelled | `CANCELLED` plus `cancel_reason` | yes |
| GTD time reached | `EXPIRED` | no, timer ticket |
| Stop fires | `TRIGGERED` | no |

### Ordering (rule)

- For each match: the settlement first, then the buyer's report, then the seller's.
- An order's final report comes after every settlement that touched it.
- So a self-trade cancel is `NEW`, then `CANCELLED`.

**An example.** Alice's buy for 0.5 BTC fills Bob's resting sell for 0.5 BTC. The engine emits, in order:

```
41  TradeSettlement   trade 9, 0.5 BTC at $29,000, taker_side = Buy
42  ExecutionReport   Alice's buy, TRADE, FILLED, trade_id = 9
43  ExecutionReport   Bob's sell,  TRADE, FILLED, trade_id = 9
```

**Without this order,** the backend could see Bob's `FILLED` report first. It would release what is left of Bob's reservation before the settlement moved his BTC. The next event would then spend money that was already handed back.

### Market data (proto, out of #21, from `events.proto:MarketDataItem`)

```
MarketDataItem { md_sequence, engine_time, market_symbol,
                 body: oneof { book_snapshot | book_delta } }
BookSnapshot   { bids: [PriceLevel] /* highest first */, asks: [PriceLevel] /* lowest first */ }
BookDelta      { side, price, quantity }          // 0 = level removed
PriceLevel     { price, quantity }                // total resting base at this price
```

This replaces `PriceUpdated`. It affects #21 even without the stream:

- **Level totals.** `PriceLevel.quantity` is the level's total resting base. **proposed:** each level keeps a running total. Then a delta costs O(1), one step, instead of adding up the whole queue.
- **Deltas per change.** A match, cancel or modify must say which `(side, price)` levels it touched. Each touched level produces one delta.
- **No trades on this stream.** The public list of trades is built from `TradeSettlement`.

**An example.** The asks at $29,000 hold Bob's 0.3 BTC and Carol's 0.2 BTC. The level total is 0.5 BTC. Alice buys 0.3 BTC and takes all of Bob's order. The engine emits one delta: `{ Sell, 2_900_000, 20_000_000 }`. If Carol then cancels, the delta is `{ Sell, 2_900_000, 0 }`, meaning the level is gone.

---

## Layer 9: outside the engine

Listed so the boundary stays clear.

- **Funds.** The backend reserves funds before sending, including the extra fee reserve on buys. It releases them when stream events arrive.
- **Settlement.** The backend runs the four-transfer chain in TigerBeetle for each `TradeSettlement`, using its account ids. The engine never talks to TigerBeetle.
- **Market configuration.** The backend owns market status, minimums and steps. The proto comment places them in a Postgres `market` table.
- **Restart.** **rule (#53, checked 2026-11-04):** the engine loads the markets, books, command cache, event history and last `sequence` from Redis before it answers any command; on the very first start Redis is empty. The backend does nothing on a restart: no `SetMarket` again, no cancel and refund. `engine_epoch` and the cancel-all fallback are gone. Still for the engine to settle: the exact contents of a save (resting orders with pinned fees and accounts, the last `trade_id`) and the retention window of the event history.
- **Topology (#53).** A Router service in front of the instances, and one event consumer that alone reads the merged stream and writes to TigerBeetle and Postgres.

---

## Resolved from the previous revision

| Previous gap | Resolution |
|---|---|
| Fees not modelled | `maker_fee_bps` and `taker_fee_bps` on the request. Quote fees per fill on `TradeSettlement`. |
| Self-trade prevention | `CancelReason::SelfTrade` plus the rule. Open questions are in Layer 7. |
| Market order against an empty book | Admitted and cancelled, not rejected. |
| Halts | The backend's job. `Market` has no status field. |
| Snapshots, or replay from a log | **#53:** no snapshots. State is in Redis and loaded at startup; subscribers get a replay of the retained history. See Layer 9. |
| `OrderRejected` had no reason | `RejectReason`, only in the direct reply. |
| `AmendOrder` undefined | `ModifyOrderRequest`, with queue-position rules. |
| Taker and maker named by order or by user | `TradeSettlement` carries both, for each side. |
| Where tick and lot live | Not in the engine. |

## What changes in the #20 code

| Before #21 | After #21 |
|---|---|
| `SCALE = 8` and `ONE` in `lib.rs` | Done. Removed. Scale comes from `Market`. |
| `Price(u128)`, `Quantity(u128)` | Done. `Price` kept. `Quantity` split into `BaseQuantity` and `QuoteQuantity`. 256-bit quote and fee math added. |
| `OrderId(u64)`, `TradeId(u64)` | Done. `u128`, from `Id`. |
| `UserId(u128)` | `AccountHolder`. Add `AccountId`. |
| `CoinId(u32)` | Done: removed. `MarketSymbol` plus `Market` still to add. |
| `IdempotencyKey` | Done: `CommandId(u128)`. The command cache that stores each request for comparison is still to do. |
| `SeqNo` on `RestingOrder` | Done: removed from `RestingOrder`. The unused `SeqNo` type is still in `ids.rs`. |
| `OrderKind { Market, Limit }` | `Market` gains `protection_price`. Stops parse to `UNSUPPORTED`. |
| `TimeInForce { GoodTilCancelled, ImmediateOrCancel, FillOrKill }` | Adds `Gtd { expire_time }`. |
| `OrderStatus` with `Open` and `Rejected` | The proto set. Drops both, adds `Expired` and `PendingTrigger`. |
| `Fill { maker, taker, price, quantity }` | Becomes `TradeSettlement`: trade id, both orders, both holders, all four accounts, quote amount, both fees, taker side, fee account. |
| `Execution { fills, resting }` | Becomes `CommandResponse` plus `Vec<Event>`. |
| `EngineError` | Command outcomes move to `RejectReason`, `CancelReason` and `ConfigRejectReason`. `EngineError` stays only for broken internal invariants, and **decided:** an error with no proto meaning panics and the process exits (see Layer 6, errors with no proto meaning). So far `IdempotencyKeyInvalid` is gone and `QuantityNotAboveFilled` (`BAD_QUANTITY`) is added. |
| `Book::amend(id, replacement)` | Done: `Book::modify(id, new_quantity, new_price)` with the queue-position rules (Layer 4). |
| `Timestamp(u64)` in `src/timestamp.rs` | Done: removed, `std::time::SystemTime` used instead. |
| Hand-written 256-bit multiply and long division in `wide.rs` | Done: `primitive_types::U256`. |
| `TryFrom<Option<Uint128>>` for `Price` and the quantities | Done: removed, callers use `required()` explicitly. |

Rows marked "Done" are already in the code (as of the PR #31 review rework). Every other row describes `src/ids.rs`, `src/order.rs`, `src/resting_order.rs`, `src/book.rs` and `src/error.rs` as they are today.

**Not in the engine code, but found during the rework:** the CI `test` job runs `cargo test --locked --bins`, which runs zero engine tests because they all live in `tests/`. The flag is there to skip the backend's integration test, which needs Postgres and TigerBeetle. The engine's job needs it without `--bins`.
