# Backend boundaries & responsibilities

This document defines what the **backend** is responsible for in this project, and where other parts (database, frontend) take the lead.

## Scope: what “backend” means here

**Backend (this repository) includes:**
- **API service** (REST + WebSockets): client-facing network interface
- **Trading engine**: matching/simulation/pricing and market state transitions
- **Shared domain code**: types, commands/events, pure validation rules

**Not in this repository:**
- **Database implementation** (separate service/repository)
- **Frontend** (web/mobile/CLI clients)

---

## High-level goal

Keep a strict separation between:
- **Transport & access control** (API)
- **Business/domain truth** (engine)

This makes the system:
- easier to test (engine is deterministic)
- easier to scale (engine workers can scale independently)
- harder to break (HTTP code cannot silently change trading outcomes)

---

## API service responsibilities (what API owns)

The API service is the backend’s **public interface**.

It owns:
- Authentication / sessions / permissions
- Request validation (shape + permission checks)
- REST endpoints and WebSocket connections
- Rate limiting / pagination / error formatting
- Translating HTTP requests into **engine commands**
- Broadcasting **engine events** to clients (real-time updates)
- Observability (logging, metrics, tracing)

API must NOT own:
- matching logic (order book rules)
- price formation / simulation logic
- order lifecycle transitions (beyond trivial sanity checks)
- trade execution decisions

**Rule:** API can reject a request, but it cannot decide *what trades happen*.

---

## Trading engine responsibilities (what the engine owns)

The engine is the backend’s **domain authority** (“market brain”).

It owns:
- Domain-level validation of trading commands
- Order lifecycle transitions (NEW → OPEN → PARTIAL → FILLED/CANCELLED)
- Matching and trade execution
- Price calculation / simulation rules
- Creating authoritative events:
  - `OrderAccepted` / `OrderRejected`
  - `TradeExecuted`
  - `PriceUpdated`
  - etc.
- Idempotency handling for commands (important for reliability)

Engine must NOT own:
- HTTP routing, REST semantics
- WebSocket connection management
- user sessions / tokens

**Rule:** if changing code can change trading outcomes, it belongs in the engine.

---

## Shared domain library responsibilities

The shared domain code should include:
- Core types: `Order`, `Trade`, `Instrument`, `Market`, `Money`, etc.
- Command/event schemas (engine inputs/outputs)
- Pure validation utilities and shared errors

Constraints:
- No network IO
- No database IO
- Keep it deterministic and portable

---

## Database responsibilities (external service)

The database is a separate project/service and owns:
- persistence, replication, consistency model
- schema/migrations (depending on DB design)
- long-term storage of users/orders/trades/balances/etc.

Backend responsibilities around the DB boundary:
- define a clear contract: what is stored, how it is queried, how it is updated
- ensure idempotency of write operations (to handle retries)
- keep engine events as the truth for market outcomes

**Rule:** backend defines *what* must be persisted; DB service defines *how* it is stored.

---

## Frontend responsibilities (clients)

Frontend owns:
- user experience (UI/UX)
- visualizations, charts, dashboards
- calling REST endpoints
- maintaining WebSocket subscriptions
- client-side caching and state management

Frontend must NOT own:
- any trading outcomes logic (matching/pricing rules)
- authoritative balance/position calculations (display-only is fine)

---

## Recommended system flow (typical request)

1. Client sends `PlaceOrder` to API (HTTP).
2. API authenticates, checks permissions, validates request shape.
3. API forwards a command to the engine (sync or async).
4. Engine validates domain rules, performs matching/pricing, emits events.
5. API streams events to subscribed clients via WebSockets.
6. Persistence is handled via the DB contract (API-owned or engine-owned writes — decide and document).

---

## Non-negotiable border rules (keep it clean)

- Only the engine can execute trades.
- Only the engine can transition order state.
- API is responsible for auth and transport.
- Domain types/events are shared and IO-free.
- Database is external; its implementation is not part of this repo.
