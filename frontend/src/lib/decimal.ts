// Exact arithmetic on the decimal strings the API uses ("65000.00", "0.2"), done the same
// way as the backend: as whole numbers of the asset's smallest unit.
//
// New JavaScript: BigInt (written 10n, 150n, ...) is a whole number of any size, with
// exact +, -, *, and / that rounds down. Regular numbers are floating point, where
// 0.1 + 0.2 is 0.30000000000000004, so money is never computed with them here.

// "1.5" with 2 decimals -> 150n. null if it isn't a plain positive decimal, or has
// more decimals than the asset (the backend would reject it with a 400).
export function toUnits(value: string, decimals: number): bigint | null {
    const match = /^(\d*)(?:\.(\d*))?$/.exec(value.trim());
    if (!match) return null;
    const whole = match[1];
    const fraction = match[2] ?? "";
    if (whole === "" && fraction === "") return null;
    if (fraction.length > decimals) return null;
    return BigInt((whole || "0") + fraction.padEnd(decimals, "0"));
}

// 150n with 2 decimals -> "1.50"
export function fromUnits(units: bigint, decimals: number): string {
    const negative = units < 0n;
    const digits = (negative ? -units : units).toString().padStart(decimals + 1, "0");
    const text =
        decimals === 0
            ? digits
            : `${digits.slice(0, -decimals)}.${digits.slice(-decimals)}`;
    return negative ? `-${text}` : text;
}

// What a limit order of `quantity` at `price` costs in the quote asset, rounded down,
// exactly like the backend's quote_for(). null where the backend can't compute it either
// (when the quote asset has more decimals than the price and the base together).
export function quoteFor(
    priceUnits: bigint,
    baseUnits: bigint,
    priceDecimals: number,
    baseDecimals: number,
    quoteDecimals: number,
): bigint | null {
    const exponent = priceDecimals + baseDecimals - quoteDecimals;
    if (exponent < 0) return null;
    return (priceUnits * baseUnits) / 10n ** BigInt(exponent);
}

// The most a trade of `quoteUnits` can cost in fees at `bps` basis points (1 bps =
// 0.01%), rounded up, like the backend's fee_cap(). Buy orders reserve this on top.
export function feeCap(quoteUnits: bigint, bps: number): bigint {
    return (quoteUnits * BigInt(Math.max(bps, 0)) + 9_999n) / 10_000n;
}
