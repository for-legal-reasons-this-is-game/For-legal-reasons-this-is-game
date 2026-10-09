# QuoteScale (`src/quote.rs`)

`QuoteScale` turns a price and a base amount into a quote amount. It answers "how much money does this much coin cost?". Each market has one, because each market has its own decimals.

## The formula

The contract defines it:
```
k     = price_decimals + base_decimals - quote_decimals
quote = floor(price * base_qty / 10^k)
```
`floor` means round down to a whole number.

**An example.** In BTC-USD the price is in cents (2 decimals), base is in satoshis (8 decimals), and quote is in cents (2 decimals). So:
```
k = 2 + 8 - 2 = 8
```
Alice buys half a bitcoin at $29,000:
```
price    = 2_900_000     ($29,000.00, in cents)
base_qty = 50_000_000    (half a BTC, in satoshis)

quote = 2_900_000 * 50_000_000 / 10^8
      = 1_450_000        ($14,500.00, in cents)
```
In code:
```rust
let btc_usd = QuoteScale::new(2, 8, 2)?;     // exponent() == 8
btc_usd.quote_for(price, half_a_bitcoin)     // Ok(1_450_000)
```

## Why a type and not a function

`k` comes from the market. It never changes while the market has orders. So `QuoteScale::new` works it out and checks it once. After that, every quote can only fail on a real overflow.

`new` is the only way to build a `QuoteScale`, so one with a bad exponent cannot exist. It stores the exponent `k` and the divisor `10^k`. `exponent()` gives `k` back. The `Market` type will hold one.

## What `new` rejects

All of these return `DecimalsOutOfRange`.

- **`k` below zero.** This happens when `quote_decimals` is bigger than `price_decimals + base_decimals`. For example `new(2, 2, 5)` gives `2 + 2 - 5 = -1`. The formula would turn into a multiply. The data model proposes rejecting this as the contract's `BAD_DEFINITION`, and this is where that check lives.
- **`k` above 38.** `10^38` is the largest power of ten a `u128` can hold. `QuoteScale::MAX_EXPONENT` is 38. So `new(38, 0, 0)` works and `new(39, 0, 0)` fails.
- **Decimals too big to add.** `price_decimals + base_decimals` is added with a check. Without it, absurd values like `new(u32::MAX, 1, 0)` could wrap around to a small, valid-looking `k`.

## `quote_for`

It multiplies in 256 bits, then divides, then rounds down. All three are contract rules.

- **256 bits.** `price * base_qty` can be too big for 128 bits even when the final quote fits easily. With 8 base decimals, a 1 BTC order at a high price is already close to that edge. Multiplying in `u128` would either overflow, or force you to divide first, which loses precision.
- **Round down.** The backend reserves Alice's money using the same formula. The sum of all her fills has to fit inside that reservation. Rounding up, or to the nearest, could go over by one cent per fill.
  ```
  with k = 1:  price 15, base 1  ->  1.5  ->  1
               price 19, base 1  ->  1.9  ->  1
  ```
- **`Overflow`** is returned when the result itself does not fit in 128 bits. At place time, this is what the contract's `BAD_QUANTITY` check ("still fits in 128 bits after the division") will use.

**A quote can round to zero.** With `k = 1`, price 3 times base 3 is 9, and 9 / 10 rounds down to 0. The contract says this should never happen, because the backend's minimum order size prevents it. Matching will log it rather than reject it.

## The 256-bit arithmetic

It lives in `src/wide.rs`, which is private to the crate. It has one operation, `mul_div_floor(a, b, d)`, meaning `a * b / d` rounded down:

1. Multiply `a * b` into a 256-bit number, kept as two `u128` halves `(high, low)`. This is school-style long multiplication, with four 64-bit partial products.
2. If `d` is zero, there is nothing to divide by. Return `None`. If `high >= d`, the answer cannot fit in 128 bits. Also return `None`.
3. If `high` is zero, the product fits in 128 bits, so plain division works.
4. Otherwise do long division, one bit at a time, like on paper.

It is hand-written instead of a library because the crate has no dependencies, and this is the only wide operation the contract needs.

It is tested through `quote_for` and `fee_on`. The tests include products past 128 bits, like `10^30 * 10^30`. One test uses a divisor near the top bit. That exercises the tricky step in the long division, where the remainder briefly needs more than 128 bits.

## The derives

`Debug, Clone, Copy, PartialEq, Eq`.
- **No `Ord` and no `Hash`.** Nothing sorts scales or keys a map by one.
- **No `Default`.** A default would skip `new` and its checks.
