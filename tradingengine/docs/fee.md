# FeeBps (`src/fee.rs`)

`FeeBps` is a fee rate, counted in basis points. It also holds the one formula that applies the rate to an amount.

## What a basis point is

A basis point ("bps") is one hundredth of a percent. So:
```
1 bps       = 0.01%
10 bps      = 0.10%
10_000 bps  = 100%, the whole amount
```
`FeeBps::DENOMINATOR` names that `10_000`. `FeeBps::ZERO` is a rate of zero.

The formula is:
```
fee = floor(quote * rate_bps / 10_000)
```
`floor` means round down to a whole number.

**An example.** Bob's trade in BTC-USD is worth $10,000.00, which is 1_000_000 cents. His fee rate is 10 bps:
```
fee = 1_000_000 * 10 / 10_000
    = 1_000 cents         ($10.00)
```
In code:
```rust
FeeBps::from_bps(10).fee_on(QuoteQuantity::from_minor_units(1_000_000))   // Ok(1_000)
```

## The engine applies rates, it never picks them

The contract puts `maker_fee_bps` and `taker_fee_bps` on the place request. The backend reads them from its own market table at that moment. They stay with the order for its whole life. The engine only multiplies.

That is why the constructor does no checking. `from_bps` never fails. `bps()` gives the number back out.

- **Zero is legal.** It means no fee. On the wire the field is a plain `uint32`, so "not sent" also reads as zero. The engine cannot tell the two apart.
- **Above `10_000` is accepted for now.** Whether to reject a rate over 100% is an open question in the data model. If it is decided, the check goes in place validation, not here.

## `fee_on`

It takes a quote amount and returns a quote amount. Fees are always paid in quote. In a `TradeSettlement`, the buyer's fee and the seller's fee both come out of their quote accounts.

- **Per fill.** The caller charges each fill separately, at the rate for the role the order played in that fill. A "maker" order was already resting in the book. A "taker" order arrived and matched against it. One order can be taker for part of its size and maker for the rest.
- **Round down**, for the same reason as the quote itself. The backend reserves money with the same formula, and the fills must fit inside it. See [quote.md](quote.md).
  ```
  1 bps of 19_999 cents  =  1.9999  ->  1
  1 bps of  9_999 cents  =  0.9999  ->  0
  ```
- **256 bits in the middle.** `quote * bps` can be too big for 128 bits when the quote is large. For example, `u128::MAX * 5_000` does not fit, even though the fee, half of `u128::MAX`, does. It uses the same `mul_div_floor` as `quote_for`.
- **`Overflow`** can only happen with a rate above 100% on an amount near `u128::MAX`. At or below 100%, the fee is never more than the quote. For example, 20_000 bps (200%) of `u128::MAX` overflows.

## The derives

`Debug, Clone, Copy, PartialEq, Eq`.
- **No `Ord`.** The engine never compares rates. It picks the rate by role, maker or taker, not by size.
- **No `Hash`.** Nothing keys a map by a rate.
