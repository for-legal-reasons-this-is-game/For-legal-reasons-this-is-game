# Backend boundaries and responsibilities

This document describes the components of the trading system, what each one is
responsible for, which component owns which data, and the rules that keep the
boundaries between them clean.

For the detailed rules on implementing the gRPC contract between the backend and
the trading engine, see [`proto/how_to_use.md`](../proto/how_to_use.md).

## Goal

The system keeps a strict separation between three concerns:

| Concern | Owner |
|---|---|
| Transport and access control | Backend |
| Money | Backend, stored in TigerBeetle |
| Market outcomes: the order book, matching, and order state | Trading engine |

This separation has the following benefits:

- **Testability:** the engine is deterministic and can be tested on its own.
- **Scalability:** the engine scales independently of the backend.
- **Safety:** HTTP code can't silently change trading outcomes, and the engine
  can't move money.

## Components

| Component | Location | Description |
|---|---|---|
| Backend | `backend/` | The API service, written in Rust with axum. It exposes the REST and WebSocket interface, owns money movements, and talks to the engine over gRPC. |
| Trading engine | `tradingengine/` | A separate service that holds the order book, matches orders, and reports results over gRPC streams. |
| gRPC contract | `proto/` | The protobuf definitions shared by the backend and the engine, compiled into the `proto` crate. |
| Postgres | `db` service | Stores users, account metadata, ledgers, markets, orders, trades, and the event stream cursor. The schema is in `backend/migrations/`. |
| TigerBeetle | `tigerbeetle_*` services | Stores balances and transfers. It's the system of record for money. |
| Frontend | Not in this document | Web clients that call the backend. |

## Backend responsibilities

The backend is the system's public interface and its owner of money.

### The backend owns

- Authentication, sessions, and permissions.
- Request validation: request shape, permissions, and the market rules the
  engine doesn't check (minimum sizes, quantity and price steps, and market
  status).
- REST endpoints and WebSocket connections.
- Rate limiting, pagination, and error formatting.
- Translating HTTP requests into engine commands.
- Broadcasting engine events to clients in real time.
- Money: reserving funds before an order reaches the engine, settling every
  trade, and refunding what's left when an order ends.
- Market configuration: markets, fee rates, minimums, steps, and status. The
  backend pushes market definitions to the engine with `SetMarket`.
- The database schema and migrations.
- Observability: logging, metrics, and tracing.

### The backend doesn't own

- Matching logic or order book rules.
- Price formation.
- Order state transitions.
- Decisions about which trades happen.

The backend can reject a request, but it can't decide which trades happen.

## Trading engine responsibilities

The engine is the authority on market outcomes.

### The engine owns

- Validation of the fields of trading commands.
- The order book.
- Matching and trade execution.
- Order state transitions, reported in `ExecutionReport` messages.
- Fee amounts per fill, computed from the rates that the backend sends with each
  order.
- Order expiry for GTD orders, and stop order triggers.
- Idempotency of commands, using `command_id`.
- The event stream (`SubscribeEvents`) and the market data stream
  (`SubscribeMarketData`).

### The engine doesn't own

- Money, balances, or checks on whether a user can pay.
- Market configuration or fee rates.
- Minimum sizes, steps, or market status.
- HTTP routing, REST semantics, or WebSocket connections.
- User sessions or tokens.

If changing code can change which trades happen, the code belongs in the engine.
If changing code can change where money goes, the code belongs in the backend.

## Shared contract

The `proto` crate contains the types shared by the backend and the engine:

- Commands and their responses, such as `PlaceOrderRequest` and
  `PlaceOrderResponse`.
- Stream messages, such as `EventItem`, `ExecutionReport`, `TradeSettlement`,
  and `MarketDataItem`.
- Shared types, such as `Market`, `Id`, and `Uint128`, and conversions between
  them and Rust types.

The shared contract has the following constraints:

- No network I/O.
- No database I/O.
- Deterministic and portable.

## Data ownership

Each piece of data has exactly one system of record.

| Data | System of record | Written by |
|---|---|---|
| Balances and transfers | TigerBeetle | Backend |
| Users, account metadata, ledgers, and markets | Postgres | Backend |
| Order book and live order state | Engine (in memory) | Engine |
| Order history and trade history | Postgres | Backend, from engine events |
| Event stream position | Postgres (`engine_stream_state`) | Backend |
| Current market depth | Backend (in memory) | Backend, from the market data stream |

Postgres keeps a record of orders and trades for display. The engine's events are
the source of truth for market outcomes, and TigerBeetle is the source of truth
for money.

## Frontend responsibilities

### The frontend owns

- User experience: UI, visualizations, charts, and dashboards.
- Calling REST endpoints.
- Maintaining WebSocket subscriptions.
- Client-side caching and state management.
- Generating an `idempotency_key` for each order request, so that retries don't
  create duplicate orders.

### The frontend doesn't own

- Any trading outcome logic, such as matching or pricing rules.
- Authoritative balances or positions. Calculations for display only are
  acceptable.

## Order flow

The following steps describe the life of an order:

1. The client sends an order request to the backend over HTTP.
2. The backend authenticates the user, checks permissions, and validates the
   request against the market's rules.
3. The backend records the order in Postgres and reserves the funds in
   TigerBeetle.
4. The backend sends `PlaceOrder` to the engine over gRPC.
5. The engine validates the command, matches the order, and emits events on the
   event stream.
6. The backend reads the event stream, settles each trade in TigerBeetle, and
   records the results in Postgres.
7. The backend pushes the updates to subscribed clients over WebSockets.

## Boundary rules

- Only the engine executes trades.
- Only the engine changes order state.
- Only the backend moves money.
- The backend makes decisions only from data it writes itself and from the
  message it's processing. It never decides anything from the order book or
  trades it reads from the engine's streams.
- The backend is responsible for authentication and transport.
- The shared contract contains no I/O.
