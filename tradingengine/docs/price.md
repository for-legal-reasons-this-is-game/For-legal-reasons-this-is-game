# `src/price.rs`

A price, stored as an `i64` count of minor units at the crate's `SCALE` decimal places.

## Why an integer and not a float

Two reasons, and the second is the one that actually decides it.

1. `0.1 + 0.2` is `0.30000000000000004` in binary floating point. Prices that look
   exact are not.
2. **Floating point addition is not associative.** `(a + b) + c` can differ from
   `a + (b + c)` in the last bits. The engine has to be *replayable* — feed the same
   commands in the same order and get identical state, because that is how crash
   recovery and audit work. With `f64`, two runs that process the same trades in a
   slightly different internal order can disagree, and no test will reliably catch it.

Integers add associatively. Replay works.

There is a visible payoff on the derive line: `Eq` and `Hash` **cannot** be derived on a
wrapper around `f64`, because `NaN != NaN` breaks the reflexivity both traits require.
Because the representation is an integer, `Price` gets exact equality, exact ordering
and hashing for free.

## Why a newtype and not a bare `i64`

`Price(i64)` compiles to exactly the same machine code as `i64` — the wrapper is free at
runtime. What it buys is that the compiler refuses to let a price be passed where a
quantity belongs, or two ids be swapped.

## The invariant: strictly positive

**The inner field is private.** That is the whole mechanism. No code outside this file
can write `Price(-5)`; the only door in is `from_minor_units`, which rejects anything
not strictly positive.

The alternative design — a plain `i64` plus an `if price <= 0` check at every call site
— gets forgotten exactly once, and then a negative price is sitting in the book. Pushing
the check into the constructor means the compiler enforces it everywhere, permanently.

Zero is rejected along with negatives: a price of zero means "I will trade this for
nothing", which is not a price.

## Why `from_minor_units` rather than `new`

`Price::new(150)` is ambiguous at the call site — 150 whole units, or 150
hundred-millionths? Putting the unit in the name removes the guess. A `from_major_units`
would sit alongside it if one is ever needed.

## The derives

| derive | why |
|---|---|
| `Debug` | `assert_eq!` will not compile without it |
| `Clone` | prerequisite for `Copy` |
| `Copy` | it is one `i64`; without this every use *moves* and you fight the borrow checker for nothing |
| `PartialEq`, `Eq` | comparing prices; exact because the inside is an integer |
| `PartialOrd`, `Ord` | **the important one** — see below |
| `Hash` | for maps keyed by price |

**`Ord` is what makes the order book work.** The book is a `BTreeMap` keyed by `Price`,
and `BTreeMap` requires `Ord` to keep its keys sorted. Price-time priority then falls
out of the data structure rather than being coded: the lowest key is the best ask, the
highest is the best bid. `price_test.rs` pins this down by inserting three prices out of
order and reading them back sorted.

**`Price` must never derive `Default`.** It would hand out `Price(0)` — an invalid price
— while bypassing the constructor entirely. The private field protects against
everything except that.

## Where `SCALE` and `ONE` live

At the crate root, in `lib.rs`, not here. They are the crate's numeric representation,
shared with `Quantity` and later `Money`; keeping them in `price.rs` would have left
`Quantity` with no defined scale and put a false "quantity depends on price" edge in the
module graph.

A `const` assertion in `lib.rs` fails the build if `ONE` and `SCALE` ever stop agreeing.

## Not here yet

- **`Display` and decimal parsing.** Needed at the API boundary, where a price crosses
  the wire as a *string* — sending it as a JSON number would route it through an `f64`
  and discard the exactness this whole design protects. Not needed by the book.
- **Tick alignment.** A price must be a whole multiple of its coin's tick size. That
  check belongs with the coin's specification, which does not exist yet.
- **Arithmetic.** No addition or subtraction yet. When spreads arrive, note that the
  difference of two prices is legitimately zero or negative, so it cannot itself be a
  `Price`.
