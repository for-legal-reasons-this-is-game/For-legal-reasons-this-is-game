import { formatDecimal, formatTrimmed } from "@/lib/format";
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";

// An order in a few words, "0.2 BTC at 65,000.00 USD", for the toast after placing it and
// the order page's title. Amounts drop trailing zeros; prices keep their decimals.
export function describe(order: Order, market: Market): string {
    const base = market.base_asset.symbol;
    const quote = market.quote_asset.symbol;
    const amount = order.base_quantity
        ? `${formatTrimmed(order.base_quantity)} ${base}`
        : `${formatTrimmed(order.quote_quantity ?? "0")} ${quote} of ${base}`;
    return order.limit_price
        ? `${amount} at ${formatDecimal(order.limit_price)} ${quote}`
        : `${amount} at the market price`;
}
