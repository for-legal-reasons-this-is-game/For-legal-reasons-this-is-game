# Price (`src/price.rs`)

A `Price` says how much quote money one whole unit of the base coin costs. It is stored as a whole number, a `u128`, never as a decimal. On the wire it is the contract's `Uint128`.

## What it is

The number counts the smallest unit of the quote currency. These smallest units are called "minor units". For USD the minor unit is the cent.

**An example.** In the BTC-USD market, a price is counted in cents per whole BTC:
```
$29,000.00 per BTC  =  2_900_000   (cents)
```
So `Price::from_minor_units(2_900_000)` means "one bitcoin costs $29,000".

How many decimals a price has is set by the market, in `price_decimals`. BTC-USD uses 2, because cents have 2 decimals. Other markets can use a different number.

## Why not a float

A float (`f64`) is a number with a decimal point, stored in binary. There are two problems with it.

- **Floats are not exact.** In binary, `0.1 + 0.2` is `0.30000000000000004`. A price that looks exact is not.
- **Order changes the answer.** With floats, `(a + b) + c` can differ from `a + (b + c)` in the last digits. This is the problem that decides it.

The second problem matters because the engine must be "replayable". Feed it the same commands in the same order, and it must end in exactly the same state. Crash recovery and audits depend on this. With floats, two runs that add the same trades in a slightly different order can disagree. No test will reliably catch that.

Integers do not have this problem. `(a + b) + c` always equals `a + (b + c)`. Replay works.

**A bonus.** Rust will not let you derive `Eq` or `Hash` on a wrapper around `f64`. The reason is `NaN` ("not a number"), which is not equal to itself. Because `Price` holds an integer, it gets exact equality, exact ordering and hashing for free.

## The rule: a price is always above zero

The number inside `Price` is private. The only way to build one is `from_minor_units`. It returns an error for zero:
```rust
Price::from_minor_units(0)          // Err(EngineError::PriceNotPositive)
Price::from_minor_units(2_900_000)  // Ok(Price), $29,000
```
A negative price is already impossible, because `u128` has no negative numbers. Zero is rejected because a price of zero means "I will sell this for nothing". That is not a price.

`minor_units()` gives the number back out.

## Why `from_minor_units` and not `new`

Look at `Price::new(150)`. Is that 150 dollars or 150 cents? You cannot tell. Putting the unit in the name removes the guess. If a `from_major_units` is ever needed, it would sit next to it.

## The derives

A "derive" asks the compiler to write a standard trait for the type.

| derive | why |
|---|---|
| `Debug` | `assert_eq!` does not compile without it |
| `Clone` | needed before `Copy` is allowed |
| `Copy` | it is one `u128`. Without `Copy`, every use would move the value and you would fight the borrow checker for nothing |
| `PartialEq`, `Eq` | comparing prices. Exact, because the inside is an integer |
| `PartialOrd`, `Ord` | sorting prices. The most important one, see below |
| `Hash` | for maps keyed by price |

**`Ord` is what makes the order book work.** The book is a `BTreeMap` keyed by `Price`. A `BTreeMap` keeps its keys sorted, and it needs `Ord` to do that. So the best prices come out of the data structure for free:
- The lowest key is the best ask, the cheapest seller.
- The highest key is the best bid, the highest buyer.

`tests/price.rs` checks this. It inserts $29,002, $29,000 and $29,001 out of order, and reads them back sorted.

**`Price` must never derive `Default`.** `Default` would hand out `Price(0)`, which is invalid. It would also skip the constructor completely. The private field blocks every other way in, but not that one.

## Where the decimals live

In the market, not in this file. The contract's `Market` carries `price_decimals`, `base_decimals` and `quote_decimals`, and two markets can differ. The old crate-wide constants `SCALE = 8` and `ONE = 10^8` are gone. `Price` is just the integer. What it means in dollars depends on which market it belongs to.

To turn a price and a base amount into a quote amount, use `QuoteScale::quote_for`. See [quote.md](quote.md).

## Not done yet

- **`Display` and decimal parsing.** These are needed at the API edge, where a price travels as a string. Sending it as a JSON number would pass it through an `f64`, and the exactness would be lost. The book does not need them.
- **No tick check, on purpose.** A "tick" is the smallest allowed price step, for example $0.50. Under the contract, ticks are the backend's rule and never go on the wire. The engine does not check them.
- **No arithmetic.** There is no adding or subtracting prices yet. When spreads arrive, remember that the gap between two prices can be zero or negative. So the gap cannot itself be a `Price`.
