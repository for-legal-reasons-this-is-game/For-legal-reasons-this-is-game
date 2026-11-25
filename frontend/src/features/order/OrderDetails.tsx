import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { accountQueries } from "@/api/queries";
import { Skeleton } from "@/components/ui/skeleton";
import { averagePrice, fromUnits, toUnits } from "@/lib/decimal";
import { formatBps, formatDateTime, formatDecimal } from "@/lib/format";
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";
import {
    ORDER_TYPE_LABELS,
    SIDE_LABELS,
    TIME_IN_FORCE_LABELS,
    TRIGGER_DIRECTION_LABELS,
} from "./labels";
import { isOpen } from "./order-state";

// "0.0500" and a muted "BTC" after it
function Amount({ value, asset }: { value: string; asset: string | undefined }) {
    return (
        <>
            {formatDecimal(value)}
            {asset && <span className="text-muted-foreground"> {asset}</span>}
        </>
    );
}

// An account by its name, linking to its page (the name loads after the order)
function AccountLink({ accountID }: { accountID: string }) {
    const account = useQuery(accountQueries.detail(accountID));
    return (
        <Link to={`/accounts/${accountID}`} className="hover:underline">
            {account.data ? (
                account.data.account_name
            ) : (
                <Skeleton className="inline-block h-4 w-24 align-middle" />
            )}
        </Link>
    );
}

// The average price of what has filled so far, worked out from the order's totals
function averageOf(order: Order, market: Market): string | null {
    const { base_asset: base, quote_asset: quote, price_decimals } = market;
    const baseUnits = toUnits(order.filled_base_quantity, base.decimals);
    const quoteUnits = toUnits(order.filled_quote_quantity, quote.decimals);
    if (baseUnits === null || quoteUnits === null) return null;
    const units = averagePrice(
        quoteUnits,
        baseUnits,
        price_decimals,
        base.decimals,
        quote.decimals,
    );
    return units === null ? null : fromUnits(units, price_decimals);
}

// Everything about an order, as label and value rows ruled like a ledger page. Rows that
// don't apply to the order's type are left out. `market` is undefined while it loads.
export function OrderDetails({
    order,
    market,
}: {
    order: Order;
    market: Market | undefined;
}) {
    const base = market?.base_asset.symbol;
    const quote = market?.quote_asset.symbol;
    const filled = /[1-9]/.test(order.filled_base_quantity);
    const average = filled && market ? averageOf(order, market) : null;

    const rows: [string, ReactNode][] = [
        [
            "Market",
            <Link
                to={`/markets/${order.market}`}
                className="font-semibold hover:underline">
                {order.market}
            </Link>,
        ],
        [
            "Side",
            <span className={order.side === "buy" ? "text-gain" : "text-loss"}>
                {SIDE_LABELS[order.side]}
            </span>,
        ],
        ["Type", ORDER_TYPE_LABELS[order.order_type]],
        ["Time in force", TIME_IN_FORCE_LABELS[order.time_in_force]],
    ];
    if (order.expire_time) rows.push(["Expires", formatDateTime(order.expire_time)]);
    if (order.limit_price)
        rows.push(["Limit price", <Amount value={order.limit_price} asset={quote} />]);
    if (order.trigger_price)
        rows.push([
            "Trigger",
            <>
                {order.trigger_direction &&
                    `${TRIGGER_DIRECTION_LABELS[order.trigger_direction]} `}
                <Amount value={order.trigger_price} asset={quote} />
            </>,
        ]);
    if (order.protection_price)
        rows.push([
            "Protection price",
            <Amount value={order.protection_price} asset={quote} />,
        ]);
    if (order.base_quantity)
        rows.push(["Amount", <Amount value={order.base_quantity} asset={base} />]);
    if (order.quote_quantity)
        rows.push([
            "Amount to spend",
            <Amount value={order.quote_quantity} asset={quote} />,
        ]);
    rows.push([
        "Filled",
        filled ? (
            <>
                <Amount value={order.filled_base_quantity} asset={base} /> for{" "}
                <Amount value={order.filled_quote_quantity} asset={quote} />
            </>
        ) : (
            <span className="text-muted-foreground">
                {isOpen(order) ? "Nothing yet" : "Nothing"}
            </span>
        ),
    ]);
    if (average) rows.push(["Average price", <Amount value={average} asset={quote} />]);
    rows.push(
        [
            // a rejected order held nothing (or got it back at once)
            order.submission === "rejected" ? "Needed to hold" : "Held when placed",
            // a buy holds the quote asset (value and fee), a sell the base asset
            <Amount
                value={order.reserved_amount}
                asset={order.side === "buy" ? quote : base}
            />,
        ],
        [
            "Fees",
            `${formatBps(order.maker_fee_bps)} as maker, ${formatBps(order.taker_fee_bps)} as taker`,
        ],
        [`${base ?? "Base"} account`, <AccountLink accountID={order.base_account_id} />],
        [
            `${quote ?? "Quote"} account`,
            <AccountLink accountID={order.quote_account_id} />,
        ],
        ["Placed", formatDateTime(order.created_at)],
        ["Last change", formatDateTime(order.updated_at)],
    );

    return (
        <dl className="divide-y font-stretch-semi-condensed">
            {rows.map(([label, value]) => (
                <div
                    key={label}
                    className="flex items-baseline justify-between gap-4 py-2">
                    <dt className="shrink-0 text-muted-foreground">{label}</dt>
                    <dd className="min-w-0 text-right break-words">{value}</dd>
                </div>
            ))}
        </dl>
    );
}
