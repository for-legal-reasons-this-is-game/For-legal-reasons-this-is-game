# `src/ids.rs`

Six identifier types. Five wrap integers; one wraps a `String` and is the only one with
anything to validate.

## Why wrap integers at all

Every id here is a `u64`, `u32` or `u128` underneath. Left bare they are all
interchangeable, and

```rust
fill_order(user_id, order_id)
```

compiles exactly as happily as the correct argument order. Wrapped, the swapped version
is a build error.

The wrappers are free — `OrderId(u64)` compiles to the same machine code as `u64`.

## Where each one comes from

| type | wraps | assigned by |
|---|---|---|
| `OrderId` | `u64` | the engine, from a monotonic counter |
| `TradeId` | `u64` | the engine |
| `SeqNo` | `u64` | the engine, **per coin**, at dequeue |
| `CoinId` | `u32` | the instrument catalogue |
| `UserId` | `u128` | the backend |
| `IdempotencyKey` | `String` | **the client** |

**Engine-assigned ids are counters, not random UUIDs, and that is a determinism
decision rather than a performance one.** Replaying a recorded command sequence has to
produce the same ids, which rules out anything drawing on randomness or the clock.

**`SeqNo` is per-coin and assigned when the engine dequeues a command** — not at the
router, not in a transport handler. A single counter ahead of the fan-out would
serialize every command in the platform through one point, discarding the independence
per-coin sharding exists to provide. Assigning in a handler would let thread scheduling
decide the numbers, which breaks replay. See `DATA_MODEL.md` and issue #6.

## The two width choices

**`UserId` is `u128`** because the backend uses UUIDs (`backend/src/v1.rs` depends on
the `uuid` crate) and a UUID is 128 bits. A `u64` would truncate half of one, and
truncated UUIDs collide. Holding the raw 128 bits also means the backend can pass one
through with `Uuid::as_u128()` without the engine taking a dependency on `uuid` for a
value it only carries.

**`CoinId` is `u32`** because it indexes a small catalogue, not an unbounded stream.

## Derives — only what each use needs

| type | beyond `Debug, Clone, Copy, PartialEq, Eq` | why |
|---|---|---|
| `OrderId` | `Hash` | keys the cancel index, `HashMap<OrderId, (Side, Price)>` |
| `TradeId` | — | nothing keys or sorts by it |
| `SeqNo` | `PartialOrd`, `Ord` | sequence numbers are compared to establish what happened first |
| `CoinId` | `Hash` | routes a command to its shard |
| `UserId` | `Hash` | per-user lookups |
| `IdempotencyKey` | `Hash`, and **no `Copy`** | keys the dedupe set; a `String` owns a heap allocation |

This is deliberately not one uniform derive list. An unused `Ord` on `OrderId` would
make `order_a < order_b` compile — but order ids come from a counter, so that expression
silently sorts by creation time. It reads like a legitimate operation and is not.
Leaving the derive off turns it into a compile error, and adding it later becomes a
decision rather than an accident.

**`IdempotencyKey` cannot be `Copy`.** `Copy` means "duplicating this is just copying
the bits", which is true of a `u64` and false of a `String` — a `String` is a pointer to
a heap allocation, and copying the bits would give two owners of the same memory and a
double free. It gets `Clone`, which is a real deep copy.

## `new` and `value`

Both are `const fn`, so ids can be built in constant contexts. The fields stay private
so validation can be added later without changing any call site — which matters for
`IdempotencyKey` and is cheap insurance for the rest.

## Not here yet

**`IdempotencyKey` has no constructor**, so nothing can build one.

It is the only type in this file taking untrusted client input, and it needs a
constructor returning `Result`: non-empty, a length cap, and printable ASCII only. It
ends up as a map key and in log lines, and neither wants arbitrary bytes. That needs a
new `EngineError` variant.

There are no tests in this file. The five numeric ids are pure wrappers with no
behaviour to test; `IdempotencyKey`'s validation is the only thing here that will be
worth testing, and it does not exist yet.
