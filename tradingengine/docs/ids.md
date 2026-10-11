# `src/ids.rs`: identifiers

This file holds six identifier types. Each one is a thin wrapper around a plain value, so that different kinds of id cannot be mixed up. Five wrap integers. The sixth, `IdempotencyKey`, holds text sent by the client, and it is the only one with rules to check.

> **Changing under #21.** Most of these types are replaced by the trading contract. See the "Today / After #21" table in [DATA_MODEL.md](DATA_MODEL.md). In short: `OrderId` and `TradeId` become 128-bit, `UserId` becomes `AccountHolder`, `CoinId` becomes `MarketSymbol`, `IdempotencyKey` becomes `CommandId`, and `SeqNo` is removed. This doc describes the code as it is now.

## Why wrap integers at all

Underneath, every numeric id is a `u64`, `u32` or `u128`. If they were left bare, Rust could not tell one from another.

**An example.** Bob's user id is `42`. His order's id is `7`. A function takes them in the order `(order_id, user_id)`. With bare `u64`s, this swapped call compiles fine:

```rust
fill_order(user_id, order_id);   // looks up order 42 for user 7. Wrong, and silent.
```

With wrapper types, `UserId` and `OrderId` are different types. The same line fails to compile.

**The cost is zero at runtime.** `OrderId(u64)` compiles to the same machine code as a bare `u64`.

## The six types

| type | wraps | who assigns it |
|---|---|---|
| `OrderId` | `u64` | the engine, from a counter that only goes up |
| `TradeId` | `u64` | the engine |
| `SeqNo` | `u64` | the engine, **one counter per coin**, when it takes a command off the queue |
| `CoinId` | `u32` | the list of tradable coins |
| `UserId` | `u128` | the backend |
| `IdempotencyKey` | up to 64 bytes of text | **the client** |

### Counters, not random ids

The engine assigns its ids from counters, not random UUIDs. This is about **determinism**, not speed. Determinism means: feed in the same commands, get the same results. Replaying a recorded list of commands must produce the same ids every time. Anything based on randomness or the clock would break that.

### When `SeqNo` is assigned

`SeqNo` is counted per coin. It is assigned when the engine takes a command off its queue. It is not assigned at the router, and not in a network handler.

- **Why not at the router?** One counter in front of every coin would push every command on the platform through one point. That throws away the independence that splitting work per coin was meant to give.
- **Why not in a handler?** Then thread scheduling would decide the numbers. A replay could come out in a different order.

See [DATA_MODEL.md](DATA_MODEL.md) and issue #6. Under #21 this per-coin `SeqNo` is dropped. Time priority will come from an order's place in the queue at its price level instead.

## Why these widths

- **`UserId` is `u128`.** The backend uses UUIDs, from the `uuid` crate, and a UUID is 128 bits. A `u64` would cut off half of it, and cut-off UUIDs can collide. Holding the raw 128 bits also lets the backend pass one in with `Uuid::as_u128()`. The engine then needs no `uuid` dependency for a value it only carries around.
- **`CoinId` is `u32`.** It points into a small list of coins, not an endless stream, so 32 bits is plenty.

## Derives: only what each type needs

Every type derives `Clone, Copy, PartialEq, Eq`. All but `IdempotencyKey` also derive `Debug`. The extras differ:

| type | extra derives | why |
|---|---|---|
| `OrderId` | `Hash` | it is the key of the book's cancel index, `HashMap<OrderId, (Side, Price)>` |
| `TradeId` | none | nothing looks up or sorts by it |
| `SeqNo` | `PartialOrd`, `Ord` | sequence numbers are compared to decide what happened first |
| `CoinId` | `Hash` | used to send a command to the right coin's worker |
| `UserId` | `Hash` | per-user lookups |
| `IdempotencyKey` | `Hash`, and a hand-written `Debug` | it is the key of the duplicate check. `Debug` is explained below. |

**Why not give every type the same list?** Take `Ord` on `OrderId`. It would make `order_a < order_b` compile. Order ids come from a counter, so that line quietly compares creation times. It reads like a real operation, and it is not one. Leaving `Ord` off makes it a compile error. Adding it later then has to be a choice, not an accident.

## `new` and `value`

Every numeric id has `new(value)` to build one and `value()` to read it back. Both are `const fn`, which means they can run at compile time, for example to build a constant.

The inner field is private. So a rule can be added to `new` later without changing any code that uses the type. That matters for `IdempotencyKey`, and it is cheap insurance for the rest.

## `IdempotencyKey`

An idempotency key lets a client safely retry. Alice sends "buy 0.5 BTC" with key `order-1`. Her network drops before she hears back, so she sends it again with the same key. The engine sees `order-1` a second time and does not place a second order.

This is the one id whose value comes from outside the engine. It becomes a map key and it shows up in log lines, and neither should hold arbitrary bytes. So it has the only constructor here that can fail:

```rust
pub fn new(value: &str) -> Result<Self>
```

### The two rules

- **Length: 1 to `MAX_LEN` bytes.** `MAX_LEN` is 64, which has room for a 36-character UUID. The cap matters. The engine must remember every key it has seen in order to reject a retry. Without a cap, the client would choose how much memory each one costs.
- **Characters: only `A-Z`, `a-z`, `0-9`, `-` and `_`.** That is enough for a UUID or a base64url token, and nothing else.

```text
"b7f3c1a2-4d5e-4f60-9a1b-2c3d4e5f6a7b"   ok
"order-1"                                ok
""                                       rejected: empty
"has space"                              rejected: bad character
"kéy"                                    rejected: not ASCII
```

### Bytes versus characters

The length rule counts **bytes**, because `str::len` counts bytes. For non-English text, bytes and characters differ. `"kéy"` is 3 characters but 4 bytes. That could make the rule ambiguous. It never does here, because the character rule allows only ASCII, and every ASCII character is one byte. Any multi-byte input fails the character rule first.

### Stored inline, so it is `Copy`

The key is stored as a fixed array of 64 bytes plus a one-byte length:

```rust
pub struct IdempotencyKey {
    bytes: [u8; Self::MAX_LEN],
    len: u8,
}
```

A `String` would keep the text somewhere else in memory (the heap) and hold a pointer to it. The fixed array keeps the text inside the value itself. Two things follow:

- **No heap allocation** when a key is created or copied.
- **The type can be `Copy`.** `Copy` means "duplicating this is just copying the bits". That is false for a `String`: two copies of the same pointer would both try to free the same memory. It is true for a plain byte array.

The cost is size. Every key takes 65 bytes, even `"order-1"`, which needs 7. The unused bytes are always zero, so the derived `PartialEq` and `Hash` still give correct results.

### Reading it back

`as_str()` returns the key as a `&str`. It cannot fail, because the constructor only lets ASCII in.

`Debug` is written by hand, not derived. A derived `Debug` would print all 64 bytes as numbers. The hand-written one prints `IdempotencyKey("order-1")`.

### One error, not three

Empty, too long and bad character all return `EngineError::IdempotencyKeyInvalid`. To the client they are one rule: "that is not a well-formed key". The message states the whole rule, rather than which half failed: `idempotency key must be 1 to 64 characters of [A-Za-z0-9_-]`. If the backend ever needs to tell a client exactly which part it broke, that is the time to split the variant.

`error.rs` builds that message from `IdempotencyKey::MAX_LEN`, so the limit and the text can never disagree. The cost is a cycle: `error` refers to `ids`, and `ids` refers to `error`. Rust allows module cycles inside one crate, so this compiles. It is still worth knowing about before adding another.

## Tests

`tests/ids.rs` has eight tests, all on `IdempotencyKey`:

- a UUID-shaped key, and a key using every allowed character, both round-trip through `as_str()`
- empty is rejected
- exactly 64 bytes is accepted, and 65 is rejected
- a space, a `/` and a `%` are rejected
- non-ASCII (`"kéy"`) is rejected
- a `HashSet` treats equal keys as one entry and different keys as two. This checks the `Hash` and `Eq` pair that the duplicate check relies on.

The five numeric ids have no tests, on purpose. They are wrappers with no behaviour. A test like `OrderId::new(7).value() == 7` only repeats the definition.

## Not done yet

- **Nothing forgets old keys.** Checking that one key is well-formed is not the duplicate rule. The set that remembers keys is, and it grows forever. It needs an eviction policy, such as a time window or a limit per user. That belongs with the book, not here. Under #21 this becomes the `CommandId` command cache.
- **No `Display`.** Keys reach log lines through `Debug` today. The character set is already safe to print, so this is cosmetic.
