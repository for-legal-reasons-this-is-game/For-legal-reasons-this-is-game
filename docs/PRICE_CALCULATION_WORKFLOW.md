# Trading Engine: Price Calculation Implementation Workflow

A complete guide to understanding the price calculation epic, learning necessary skills, and executing the implementation in an optimized way.

---

## Overview: What We're Building

The **price calculation engine** is the financial brain of your trading platform. It:
- Takes market state (order book, recent trades) as input
- Applies mathematical models to determine fair prices
- Emits price updates so the API can broadcast to clients
- Feeds prices back into the matching engine for fair trade execution

**Key principle:** Price calculation is pure domain logic—no HTTP, no database, no side effects. This makes it testable, deterministic, and scalable.

---

## Part 1: The Workflow (Phases)

### Phase 1: Foundation & Design (Week 1)

**Goal:** Understand finance concepts deeply enough to design clean Rust code.

#### What you need to learn:

1. **Order Book Mechanics**
   - What is a bid vs. ask?
   - How does the spread reflect price?
   - Why does order book depth matter for pricing?
   - **Key concept:** Mid-price = (best_bid + best_ask) / 2

2. **Market Microstructure Basics**
   - How does supply/demand affect price?
   - What is "slippage" and why does it matter?
   - How do large trades move prices more than small ones?

3. **Domain Modeling in Finance**
   - What data structures represent an Order, a Trade, a Market?
   - What invariants must never be violated? (e.g., price ≥ 0)
   - How do you represent Money without floating-point errors?

#### Deliverable:
- A **design document** (shared Google Doc or markdown in `/docs`)
  - Core types: Order, Trade, Market, Price, Quantity
  - Price calculation rules (in plain English, no code)
  - Event schema (what the engine emits)

---

### Phase 2: Rust Fundamentals & Best Practices (Week 1–2)

**Goal:** Learn Rust deeply enough to write optimized, maintainable trading code.

#### What you need to learn:

1. **Rust Ownership & Borrowing** (critical for trading systems)
   - Ownership rules: each value has one owner
   - Borrowing: immutable (`&`) vs. mutable (`&mut`)
   - Why this matters: prevents data races, memory leaks, threading bugs
   - **Trading context:** Multiple clients reading the same price simultaneously = multiple `&` borrows; engine updates = `&mut` borrow

2. **Error Handling (Result types)**
   - `Result<T, E>` is the Rust way to handle errors
   - Why `unwrap()` is dangerous in production code
   - Propagating errors vs. handling them locally
   - **Trading context:** Invalid order? Return `Err`, don't panic

3. **Traits & Generics**
   - Traits are contracts: "anything that implements `PriceCalculator` must have this function"
   - Generics let you write code once for multiple types
   - **Trading context:** Different instruments (stocks, options, crypto) need different price formulas, but all are `Instrument`

4. **Performance-Critical Patterns**
   - Avoiding allocations in hot loops
   - Using references instead of cloning
   - Stack vs. heap allocation
   - **Trading context:** Price updates happen millions of times → every nanosecond counts

5. **Testing Strategies**
   - Unit tests for pure logic (price formulas)
   - Property-based tests (randomized inputs, check invariants)
   - **Trading context:** Prices must be correct under all market conditions

---

### Phase 3: API Design & Integration (Week 2)

**Goal:** Understand how the engine connects to the HTTP API and WebSocket streams.

#### What you need to learn:

1. **REST API Basics**
   - HTTP methods: GET (read), POST (create), PUT (update), DELETE
   - Status codes: 200 (OK), 400 (bad request), 500 (server error)
   - Statelessness: each request is independent
   - **Trading context:** `POST /orders` creates an order, engine processes it, API returns the price

2. **WebSocket Fundamentals**
   - Persistent connection between client and server
   - Server can push data to client (unlike HTTP request-response)
   - Why it's needed: real-time price updates
   - **Trading context:** Client connects once, receives live price ticks without polling

3. **Event-Driven Architecture**
   - Engine emits events (e.g., `PriceUpdated`)
   - API subscribes to events and broadcasts them
   - Decouples domain logic from transport
   - **Trading context:** Engine doesn't know about HTTP; it just emits events. API decides to broadcast them.

4. **API Boundaries & Contracts**
   - Engine inputs: structured commands with validation
   - Engine outputs: immutable events
   - Idempotency: same command twice = same result (important for retries)
   - **Trading context:** If a client reconnects and replays an order, price doesn't change twice

---

### Phase 4: Math for Trading (Week 2–3)

**Goal:** Learn the mathematics of price calculation well enough to implement confidently.

#### What you need to learn:

1. **Basic Pricing Models**
   - **Mid-price formula:** `P = (bid + ask) / 2`
   - **Slippage model:** `P_effective = mid_price + (size / liquidity) * impact_factor`
   - **Volume-weighted average price (VWAP):** `VWAP = Σ(price × volume) / Σ(volume)`

2. **Statistics for Finance**
   - Standard deviation (volatility)
   - Returns (log returns vs. simple returns)
   - Normal distribution (foundation for options pricing)
   - **Why:** Volatility is input to options pricing

3. **Options Pricing (if in scope)**
   - **Black-Scholes formula:** Price = S₀·N(d₁) - K·e^(-rT)·N(d₂)
   - Where N() is the standard normal CDF
   - Inputs: spot price, strike price, time to expiry, volatility, risk-free rate
   - **Why:** Options are complex; formula is deterministic
   - **Note:** Don't memorize; understand what each term means

4. **Greeks (if in scope)**
   - **Delta:** price sensitivity to underlying movement
   - **Theta:** time decay
   - **Gamma:** acceleration (delta sensitivity)
   - **Why:** Traders use Greeks to hedge; you may need to display them

---

### Phase 5: Implementation & Integration (Week 3–4)

**Goal:** Write production-ready Rust code that's tested and integrated with the API.

#### What you need to do:

1. **Build Core Types**
   - Define `Order`, `Trade`, `Market`, `Money`, `Quantity` in shared library
   - Implement validation (prices ≥ 0, quantities > 0)
   - No business logic yet—just data structures

2. **Implement Pure Functions**
   - `mid_price(bid: Money, ask: Money) -> Money`
   - `slippage(trade_size: Qty, book_liquidity: Qty) -> f64`
   - These take inputs, return outputs, no side effects

3. **Add Market State Management**
   - Track order book depth
   - Store price history (OHLC candles)
   - Update on each trade

4. **Emit Events**
   - Engine creates `PriceUpdated` events
   - API subscribes and broadcasts via WebSocket
   - Clients receive live updates

5. **Integrate with Matching Engine**
   - Matching engine calls price calculation
   - Price affects trade execution
   - Circular dependency check: does matching affect price? Yes → iterate

6. **Write Tests**
   - Unit tests: given order book state, price is correct
   - Property tests: prices never go negative, always increase with supply
   - Integration tests: end-to-end order → price update

---

## Part 2: Where to Learn Rust (Optimized Path)

### 0. Prerequisite: Understand Why Rust Matters Here

**Why Rust (not Python, Go, or others)?**
- **Memory safety:** No null pointer exceptions, no data races
- **Performance:** Compiled to machine code; no GC pauses
- **Correctness:** Type system catches bugs at compile time
- **Concurrency:** Easy to write multi-threaded code safely

**For a trading engine:** Speed + correctness are non-negotiable. Rust excels.

---

### 1. Absolute Beginner (0–10 hours)

**Goal:** Understand Rust syntax and core concepts.

#### Resources:

1. **The Rust Book (official):** https://doc.rust-lang.org/book/
   - **Read:** Chapters 1–6 (introduction through enums & pattern matching)
   - **Time:** ~8 hours
   - **Why:** Official, comprehensive, free, written by Rust team
   - **Key chapters:**
     - Ch. 1: Installation & setup
     - Ch. 2: Guessing game (first program)
     - Ch. 3: Variables, functions, control flow
     - Ch. 4: **Ownership** (critical—read twice)
     - Ch. 5: Structs & methods
     - Ch. 6: Enums & pattern matching

2. **Rustlings (interactive exercises):** https://github.com/rust-lang/rustlings
   - **Do:** Exercises 1–20 (variables, types, strings, functions)
   - **Time:** ~4 hours
   - **Why:** Hands-on; immediate feedback; no setup needed
   - **How:** Clone repo, run `cargo run --bin watch`, solve exercises

#### Deliverable:
- Run `rustlings` and complete up to `functions/functions3.rs`

---

### 2. Intermediate (10–30 hours)

**Goal:** Master Rust's type system, error handling, and traits (foundation for trading code).

#### Resources:

1. **The Rust Book (chapters 7–11):** https://doc.rust-lang.org/book/
   - **Read:** Chapters 7–11
   - **Time:** ~10 hours
   - **Key chapters:**
     - Ch. 7: **Modules** (organize your code)
     - Ch. 8: Collections (Vec, HashMap, String)
     - Ch. 9: **Error Handling** (Result, custom errors)
     - Ch. 10: **Generics & Traits** (write reusable code)
     - Ch. 11: **Testing** (critical for trading logic)

2. **Rustlings (continued):** https://github.com/rust-lang/rustlings
   - **Do:** Exercises 21–50 (traits, error handling, generics, modules, lifetimes)
   - **Time:** ~6 hours

3. **"Rust by Example"** (reference): https://doc.rust-lang.org/rust-by-example/
   - **Browse:** Sections on traits, generics, error handling
   - **Time:** ~4 hours (reference, not sequential)
   - **Why:** More concise than the book; good for quick lookups

4. **Lifetime Crash Course** (tricky topic):
   - **Read:** https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html
   - **Watch:** No Boilerplate's "Lifetimes Explained" (5 min): https://www.youtube.com/watch?v=l3r9qmL3Z_A
   - **Key idea:** Lifetimes tell Rust how long references are valid
   - **Trading context:** Price updates reference the order book; lifetimes ensure the book isn't dropped mid-calculation

#### Deliverable:
- Write a custom `Result<T, E>` type for trading errors
- Implement a `Trait` (e.g., `Drawable` that can be printed)

---

### 3. Advanced: Performance & Production Code (30–60 hours)

**Goal:** Write fast, idiomatic, production-grade Rust.

#### Resources:

1. **The Rust Book (chapters 13–20):** https://doc.rust-lang.org/book/
   - **Read:** Chapters 13–20 (closures, iterators, smart pointers, concurrency)
   - **Time:** ~12 hours
   - **Key chapters:**
     - Ch. 13: **Closures & Iterators** (functional programming; avoids allocations)
     - Ch. 15: Smart pointers (`Box`, `Rc`, `RefCell`) — understand when to use
     - Ch. 16: **Concurrency** (threads, channels, Arc<Mutex<T>>)
     - Ch. 19: Advanced traits & patterns

2. **"Rust for Rustaceans"** by Jon Gjengset (book): https://nostarch.com/rust-rustaceans
   - **Read:** Chapters 1–5 (particularly chapters 2–3 on traits and types)
   - **Alternative:** Free videos by same author: https://www.youtube.com/c/JonGjengset
   - **Watch:** "Crust of Rust" series (especially "Lifetime Annotations")
   - **Time:** ~15 hours
   - **Why:** Optimizations, patterns for production code, performance tuning

3. **"The Rustonomicon"** (advanced reference): https://doc.rust-lang.org/nomicon/
   - **Read:** Chapters 1–4 (unsafe Rust, memory layout, concurrency)
   - **Time:** ~10 hours (optional; only if optimizations demand it)
   - **Note:** Unsafe Rust is powerful but dangerous; probably not needed for trading engine MVP

4. **Zero-Cost Abstractions & Performance:**
   - **Read:** https://doc.rust-lang.org/book/ch13-04-performance.html
   - **Watch:** "Optimizing Rust" talk by Alexis Beingessner: https://www.youtube.com/watch?v=xHDmPDWKKBo
   - **Concept:** Rust abstractions compile to machine code; no runtime overhead
   - **Trading context:** Price calculations can be inlined; order book lookups can use `BTreeMap` for O(log n) performance

5. **Concurrency & Channels:**
   - **Read:** https://doc.rust-lang.org/book/ch16-00-concurrency.html
   - **Concept:** Engine receives orders on one channel, emits events on another
   - **Key:** `Arc<Mutex<T>>` for shared mutable state (multiple clients, one price)
   - **Trading context:** Multiple API threads read `Arc<Market>` simultaneously

#### Deliverable:
- Implement a simple concurrent calculator using channels
- Benchmark iteration vs. allocation in a hot loop

---

### 4. Specific to Trading (ongoing)

#### Resources:

1. **Tokio (async runtime):** https://tokio.rs/
   - **Read:** Tokio tutorial https://tokio.rs/tokio/tutorial
   - **Time:** ~4 hours
   - **Why:** Your API will be async; engine may be too
   - **Concept:** `async fn`, `await`, futures

2. **Serde (serialization):** https://serde.rs/
   - **Read:** https://serde.rs/
   - **Time:** ~2 hours
   - **Why:** Convert Rust structs to/from JSON for API + events

3. **Example Project (complex):**
   - Build a small exchange: read `orders.json`, calculate prices, output `trades.json`
   - Combine ownership, traits, error handling, iterators
   - Time: 8 hours

---

### Rust Learning Path: Timeline

```
Week 1 (Phase 1 overlap):
  Mon–Wed: Rust Book Ch. 1–6 + Rustlings 1–20        (8h)
  Thu–Fri: Error handling deep dive + practice       (4h)

Week 2 (Phase 2–3):
  Mon–Wed: Rust Book Ch. 7–11 + Rustlings 21–50     (10h)
  Thu–Fri: Traits, generics, lifetimes deep dive     (6h)

Week 3 (Phase 4):
  Mon–Wed: Iterators, closures, smart pointers       (8h)
  Thu–Fri: Concurrency (channels, Arc<Mutex<T>>)     (6h)

Week 4 (Phase 5):
  Ongoing: Serde, Tokio, apply learning to code
```

---

## Part 3: Where to Learn APIs (Beginner → Advanced)

### 0. Why APIs Matter for Your Trading Engine

Your trading engine must:
- **Receive commands** from the API (orders, queries)
- **Emit events** that the API broadcasts to clients
- **Provide data** (current prices, order status) via HTTP endpoints
- **Support real-time** via WebSockets

APIs are the "translator" between clients and domain logic.

---

### 1. Fundamentals: HTTP & REST (Beginner)

**Goal:** Understand how the web works; what HTTP is.

#### Resources:

1. **MDN Web Docs: HTTP Overview**
   - **Read:** https://developer.mozilla.org/en-US/docs/Web/HTTP
   - **Time:** ~4 hours
   - **Sections to prioritize:**
     - HTTP Messages (request/response structure)
     - HTTP Methods (GET, POST, PUT, DELETE, PATCH)
     - HTTP Status Codes (200, 400, 404, 500, etc.)
     - Headers & Body

2. **Representational State Transfer (REST) concept**
   - **Read:** https://en.wikipedia.org/wiki/Representational_state_transfer (just intro)
   - **Concept:** Resources (identified by URLs) + operations (HTTP verbs)
   - **Example:** 
     ```
     GET /api/markets/AAPL         → Read Apple stock market state
     POST /api/orders              → Create a new order
     GET /api/orders/12345         → Read order #12345
     DELETE /api/orders/12345      → Cancel order #12345
     ```

3. **RESTful API Design Basics**
   - **Read:** https://github.com/microsoft/api-guidelines
   - **Time:** ~3 hours
   - **Key ideas:**
     - Use nouns for resources, verbs for HTTP methods
     - Consistent naming (plural: `/orders`, not `/order`)
     - Version your API (`/api/v1/`, `/api/v2/`)
     - Status codes convey success/failure

#### Deliverable:
- Sketch 5 API endpoints for your trading platform:
  - GET /api/markets (list all markets)
  - POST /api/orders (place order)
  - GET /api/orders/{id} (get order details)
  - etc.

---

### 2. Intermediate: JSON, Status Codes & Error Handling

**Goal:** Understand data serialization and how to communicate errors over HTTP.

#### Resources:

1. **JSON Format**
   - **Read:** https://developer.mozilla.org/en-US/docs/Learn/JavaScript/Objects/JSON
   - **Time:** ~2 hours
   - **Why:** Standard format for REST APIs
   - **Trading example:**
     ```json
     {
       "order_id": "12345",
       "instrument": "AAPL",
       "side": "BUY",
       "quantity": 100,
       "price": 150.50,
       "status": "FILLED",
       "timestamp": "2026-06-12T10:30:00Z"
     }
     ```

2. **HTTP Status Codes**
   - **Read:** https://developer.mozilla.org/en-US/docs/Web/HTTP/Status
   - **Memorize:**
     - 2xx: Success (200 OK, 201 Created)
     - 4xx: Client error (400 Bad Request, 401 Unauthorized, 404 Not Found)
     - 5xx: Server error (500 Internal Server Error)
   - **Time:** ~1 hour
   - **Trading context:**
     - 200: Order placed successfully
     - 400: Invalid quantity (negative)
     - 401: Not authenticated
     - 500: Engine crashed

3. **Error Response Design**
   - **Read:** https://www.rfc-editor.org/rfc/rfc7807 (Problem Details standard)
   - **Concept:** Standardize error responses so clients can parse them
   - **Example:**
     ```json
     {
       "type": "https://api.example.com/errors/invalid-quantity",
       "title": "Invalid Quantity",
       "status": 400,
       "detail": "Quantity must be greater than 0, got -50"
     }
     ```

#### Deliverable:
- Design error responses for 3 trading scenarios:
  - Insufficient balance
  - Invalid order parameters
  - Market closed

---

### 3. Advanced: WebSockets & Real-Time Communication

**Goal:** Understand how to push data from server to clients without polling.

#### Resources:

1. **WebSockets Fundamentals**
   - **Read:** https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API
   - **Time:** ~3 hours
   - **Key concepts:**
     - Persistent connection (TCP upgrade from HTTP)
     - Full-duplex (client ↔ server at same time)
     - Server can push (unlike HTTP request-response)
     - Reconnection logic (client must handle disconnects)

2. **WebSocket Protocol**
   - **Read:** https://tools.ietf.org/html/rfc6455 (skim intro + basics)
   - **Time:** ~2 hours
   - **Why:** Understand handshake, frames, close codes

3. **Message Broadcasting Patterns**
   - **Concept:** Engine emits event → API broadcasts to all connected clients
   - **Pattern 1:** Pub-Sub (publish-subscribe)
     - Engine publishes `PriceUpdated` to topic
     - API subscribers receive and forward to clients
   - **Pattern 2:** Fan-out
     - Event triggers function that sends to all client sockets
   - **Read:** https://tokio.rs/tokio/tutorial/select (for async waiting)

4. **Handling Disconnects & Retries**
   - **Concept:** Network is unreliable; clients may disconnect mid-stream
   - **Strategy:**
     - Server should track active connections
     - Client should reconnect on network error
     - Optional: event replay (if client was offline, catch up)
   - **Read:** https://www.ably.io/topic/websockets (good overview)

#### Deliverable:
- Design a WebSocket message protocol for price updates:
  ```json
  {
    "type": "PRICE_UPDATE",
    "data": {
      "instrument": "AAPL",
      "price": 150.50,
      "timestamp": "2026-06-12T10:30:00Z"
    }
  }
  ```

---

### 4. Advanced: API Frameworks & Integration with Rust

**Goal:** Learn how to build REST + WebSocket APIs in Rust.

#### Resources:

1. **Axum Web Framework** (recommended for your team)
   - **Docs:** https://docs.rs/axum/latest/axum/
   - **Tutorial:** https://tokio.rs/tokio/tutorial (foundation)
   - **Why:** Async, composable, integrates with Tokio
   - **Time:** ~10 hours
   - **Key concepts:**
     - Routing (define endpoints)
     - Handlers (functions that process requests)
     - Extractors (parse request body, headers, etc.)
     - Layers (middleware for auth, logging)
     - WebSocket support

2. **Alternative: Actix-web**
   - **Docs:** https://actix.rs/
   - **Time:** ~10 hours
   - **Why:** Faster than Axum; more mature
   - **Trade-off:** Steeper learning curve

3. **Error Handling in Web APIs**
   - **Read:** https://docs.rs/axum/latest/axum/response/index.html
   - **Concept:** Convert Rust `Result` into HTTP responses
   - **Pattern:** Implement `IntoResponse` trait for your domain errors
   - **Trading context:**
     ```rust
     impl IntoResponse for PriceError {
         fn into_response(self) -> Response {
             match self {
                 PriceError::InvalidPrice => (
                     StatusCode::BAD_REQUEST,
                     Json(json!({"error": "Price must be ≥ 0"}))
                 ).into_response(),
                 // ...
             }
         }
     }
     ```

4. **Authentication & Authorization**
   - **Read:** https://docs.rs/jsonwebtoken/ (JWT tokens)
   - **Concept:** Each request includes token proving identity
   - **Pattern:** Middleware layer checks token before reaching handler
   - **Time:** ~3 hours

5. **OpenAPI/Swagger Spec**
   - **Why:** Document your API so clients know what endpoints exist
   - **Tools:** `utoipa` crate (generates OpenAPI from Rust code)
   - **Docs:** https://docs.rs/utoipa/
   - **Time:** ~2 hours

#### Deliverable:
- Implement a simple REST endpoint in Axum:
  ```
  GET /api/health → returns {"status": "ok"}
  POST /api/orders → accepts JSON, returns order confirmation
  ```

---

### 5. Full Architecture: Event-Driven Design

**Goal:** Understand how to connect everything: API receives request → engine calculates → events broadcast.

#### Resources:

1. **Event-Driven Architecture**
   - **Read:** https://www.eventdriven.io/en/ (overview)
   - **Time:** ~3 hours
   - **Key pattern:**
     - Commands (inputs): `PlaceOrder { ... }`
     - Handlers: Engine processes command
     - Events (outputs): `OrderAccepted { ... }`, `PriceUpdated { ... }`
     - Subscribers: API receives events, broadcasts to clients

2. **Message Queues & Channels**
   - **Concept:** Decouple API from engine using queues
   - **Tools:** Tokio channels (simple), RabbitMQ / Kafka (production)
   - **Read:** https://tokio.rs/tokio/tutorial/channels
   - **Pattern:**
     - API thread sends command via channel
     - Engine thread receives, processes, sends event back
     - API thread broadcasts event to WebSocket clients
   - **Time:** ~4 hours

3. **Idempotency & Retries**
   - **Why:** Network requests fail; clients may retry
   - **Pattern:** Give each command a unique ID; ignore duplicates
   - **Read:** https://stripe.com/blog/idempotency
   - **Trading context:** Client places order twice (connection drops, auto-retry) → should execute once
   - **Time:** ~2 hours

#### Deliverable:
- Design the full request-response flow:
  1. Client sends `POST /api/orders` with JSON
  2. API validates, forwards to engine via channel
  3. Engine calculates price, creates `Trade` event
  4. API receives event, sends response to client
  5. API broadcasts via WebSocket to all price subscribers

---

### 6. Specific Tools & Libraries

#### For REST APIs:
| Tool | Purpose | Learn Time |
|------|---------|-----------|
| **Axum** | Web framework | 10h |
| **serde** | JSON serialization | 2h |
| **tower** | Middleware | 3h |
| **utoipa** | OpenAPI generation | 2h |
| **jsonwebtoken** | Auth (JWT) | 3h |

#### For WebSockets:
| Tool | Purpose | Learn Time |
|------|---------|-----------|
| **tokio-tungstenite** | WebSocket library | 3h |
| **axum::extract::ws** | Axum WebSocket support | 2h |
| **tokio::sync::broadcast** | Fan-out messaging | 2h |

#### For Testing:
| Tool | Purpose | Learn Time |
|------|---------|-----------|
| **reqwest** | HTTP client (test API) | 2h |
| **tokio-test** | Async testing | 2h |
| **mockito** | Mock HTTP requests | 2h |

---

## API Learning Timeline

```
Week 1 (Phase 2 overlap):
  Mon–Tue: HTTP basics + REST concepts                (5h)
  Wed–Fri: JSON, status codes, error handling        (5h)

Week 2 (Phase 3):
  Mon–Tue: WebSockets fundamentals                   (5h)
  Wed: Message protocols & disconnection logic       (3h)
  Thu–Fri: Axum basics + hello world                 (5h)

Week 3 (Phase 4–5):
  Mon: REST endpoints (GET, POST, PUT, DELETE)       (4h)
  Tue–Wed: WebSocket handler in Axum                 (4h)
  Thu: Error handling + IntoResponse                 (3h)
  Fri: Authentication + middleware                   (3h)

Week 4 (Phase 5):
  Mon–Tue: Event-driven integration                  (5h)
  Wed: OpenAPI/Swagger doc generation               (2h)
  Thu–Fri: Full integration test                     (5h)
```

---

## Part 4: Recommended Learning Order (Combined)

### Weeks 1–2: Foundations (Parallel)

**Rust (Chapters 1–11):**
- Ownership, borrowing, error handling, traits, generics
- **Why first:** These are prerequisites for everything else

**APIs (HTTP, REST, JSON):**
- What is HTTP? What are REST principles?
- How do JSON payloads look?
- **Why first:** Understand the interface you're building

**Finance (Order books, mid-price):**
- What does an order book look like?
- How is mid-price calculated?
- **Why first:** Frame the problem before coding

### Weeks 2–3: Intermediate (Sequential)

**Rust (Chapters 12–16: Concurrency, Smart Pointers):**
- How do I safely share data across threads?
- What is `Arc<Mutex<T>>`?
- **Why now:** Engine needs to handle concurrent orders

**WebSockets & Event-Driven Design:**
- How does real-time work?
- How do I broadcast price updates?
- **Why now:** Design your event system

**Trading Math (Slippage, VWAP):**
- How do large trades affect price?
- How is volatility measured?
- **Why now:** Implement realistic models

### Weeks 3–4: Advanced (Applied)

**Rust (Async, Tokio, Serde):**
- Write async code with `async/await`
- Serialize to JSON
- **Why now:** Build the API

**Axum Framework:**
- Route HTTP requests
- Handle WebSocket connections
- Implement middleware
- **Why now:** Write the API service

**Integration:**
- Connect engine to API via channels
- Emit events to clients
- Test end-to-end
- **Why now:** Ship MVP

---

## Part 5: Key Resources Summary (Bookmarked)

### Rust Learning
- **The Rust Book (free):** https://doc.rust-lang.org/book/
- **Rustlings (interactive):** https://github.com/rust-lang/rustlings
- **Crust of Rust (videos):** https://www.youtube.com/c/JonGjengset
- **Tokio Tutorial:** https://tokio.rs/tokio/tutorial

### API & Web
- **MDN HTTP Docs:** https://developer.mozilla.org/en-US/docs/Web/HTTP
- **Microsoft REST API Guidelines:** https://github.com/microsoft/api-guidelines
- **WebSockets (MDN):** https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API
- **Axum Docs:** https://docs.rs/axum/latest/axum/

### Finance & Trading
- **Order Book (Wikipedia):** https://en.wikipedia.org/wiki/Order_book
- **Market Microstructure (Madhavan):** Academic paper (deep dive, optional)
- **Black-Scholes (Wikipedia):** https://en.wikipedia.org/wiki/Black%E2%80%93Scholes_model
- **12factor.app:** https://12factor.net/ (system design principles)

### Testing & Quality
- **Rust Book: Testing:** https://doc.rust-lang.org/book/ch11-00-testing.html
- **proptest (property-based testing):** https://docs.rs/proptest/
- **Tokio Test:** https://tokio.rs/tokio/topics/testing

---

## Part 6: Mindset & Expectations

### On Learning Rust
- **Week 1:** "Why is ownership so complicated?"
- **Week 2:** "Oh, ownership prevents data races. That's clever."
- **Week 3:** "I can now design safe concurrent systems without thinking about it."

**Expect to be slow the first 2 weeks.** You're learning a new way of thinking. By week 4, you'll write idiomatic code fast.

### On Learning APIs
- **Week 1:** "REST is just HTTP + JSON?"
- **Week 2:** "How do I make this async?"
- **Week 3:** "I can design and implement a full system."

**APIs are simpler than Rust.** You'll likely pick them up faster.

### On Trading Math
- **Week 1:** "Mid-price is just average of bid/ask?"
- **Week 2:** "How does volatility actually affect options?"
- **Week 3:** "I can implement realistic pricing models."

**Start simple, iterate.** MVP only needs mid-price + slippage. Greeks and Black-Scholes are week 4+.

---

## Part 7: Tracking Progress

### Use This Checklist

```markdown
## Rust Learning
- [X] Rust Book Ch. 1–6 (ownership fundamentals)
- [X] Rustlings exercises 1–20
- [X] Rust Book Ch. 7–11 (traits, error handling, testing)
- [X] Rustlings exercises 21–50
- [X] Rust Book Ch. 13–16 (concurrency, iterators)
- [ ] Write a simple async program with Tokio

## API Learning
- [X] HTTP methods & status codes
- [ ] REST principles (resources, URLs)
- [ ] JSON format & serialization
- [ ] WebSocket handshake & message format
- [ ] Axum hello-world endpoint
- [ ] Axum WebSocket handler
- [ ] Error handling in REST responses

## Finance Learning
- [ ] Order book mechanics (bid, ask, spread)
- [ ] Mid-price calculation (bid + ask) / 2
- [ ] Slippage formula: price += (size / liquidity) * impact
- [ ] Optional: VWAP formula
- [ ] Optional: Black-Scholes formula (if options in scope)

## Implementation
- [ ] Design domain types (Order, Trade, Market, Price)
- [ ] Implement PriceCalculator trait
- [ ] Write unit tests for price formulas
- [ ] Integrate with matching engine
- [ ] Build REST endpoint for price query
- [ ] Build WebSocket endpoint for price stream
- [ ] End-to-end integration test
```

---

## Final Advice

1. **Don't skip the Rust Book.** It's written by the Rust team, and it's comprehensive. Reading it once saves hours of debugging.

2. **Type-driven development.** In Rust, getting the types right often means the logic works. This is different from Python/JavaScript. Embrace it.

3. **Test your formulas.** Price calculation is pure logic. Write property tests: "for any order book state, mid-price is ≥ 0." These catch subtle bugs.

4. **Sketch APIs on paper first.** Before writing code, define your endpoints, request/response shapes, and event schemas. This saves major refactoring later.

5. **Ship MVP fast.** Aim for:
   - Orders placed ✓
   - Mid-price calculated ✓
   - Prices broadcast via WebSocket ✓
   - Everything tested ✓
   
   Then iterate on: slippage, volatility, Greeks, external feeds.

6. **Collaborate async.** Design decisions in GitHub Issues, not Slack. Future contributors (and future you) can catch up.

---

## Questions to Answer Before Coding

Before phase 5 (implementation), get clarity from your team:

1. **Scope:**
   - Stocks only, or crypto + options too?
   - How many instruments at launch?

2. **Performance:**
   - How many price updates per second?
   - Latency budget (ms from trade to WebSocket broadcast)?
   - Concurrent clients?

3. **Data:**
   - Real market data or simulated/seeded?
   - External API integration (Yahoo Finance, etc.)?

4. **Matching:**
   - Price-time priority, pro-rata, other?
   - Partial fills allowed?
   - Matching happens before or after price calc?

5. **Infrastructure:**
   - Single service or separate API + engine?
   - Database technology finalized?
   - Deployment: Docker only, or Kubernetes?

Once answered, update `/docs/BACKEND.md` with these decisions.

---

## Go Build! 🚀

You have a clear path:
1. Learn Rust (weeks 1–3)
2. Learn APIs (weeks 1–3, parallel)
3. Learn finance math (week 2–3)
4. Implement (week 3–4)

Good luck. The trading engine is the heart of your platform. Make it fast, reliable, and maintainable.
