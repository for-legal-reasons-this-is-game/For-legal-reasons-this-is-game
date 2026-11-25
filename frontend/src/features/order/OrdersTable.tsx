import type { ReactNode } from "react";
import { Link, useNavigate } from "react-router";
import { errorMessage, type ApiError } from "@/api";
import type { ErrorOverrides } from "@/api/client";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { formatDecimal, formatTrimmed, formatWhen } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";
import { CancelOrderButton } from "./CancelOrderButton";
import { describe } from "./describe";
import { ORDER_TYPE_LABELS, SIDE_LABELS } from "./labels";
import { isOpen } from "./order-state";
import { OrderStateBadge } from "./OrderStateBadge";

const NUMBER = "text-right";

// How the list fits its space. It follows the width of the box it sits in, not the
// screen's: Tailwind's container queries (`@container` on the box, then `@xl:...` applies
// from 36rem = 576 px of that box; @2xl = 672 px, @4xl = 896 px). Narrower than `table`,
// each order is a two-line row; from `table` on, a table, which adds the order type and the
// filled amount from `extra` on. Orders from every market need more room, because each
// number names its asset and there's a Market column.
const LAYOUTS = {
    oneMarket: {
        rows: "@xl:hidden",
        table: "hidden @xl:block",
        extra: "hidden @2xl:table-cell",
    },
    anyMarket: {
        rows: "@2xl:hidden",
        table: "hidden @2xl:block",
        extra: "hidden @4xl:table-cell",
    },
};

// What a list query gives this table (a useQuery result fits)
type OrdersResult = {
    data: { items: Order[] } | undefined;
    isPending: boolean;
    isError: boolean;
    error: ApiError | null;
};

// A number with its asset after it in muted text, when there is one to show
function Amount({ value, asset }: { value: string; asset: string | undefined }) {
    return (
        <>
            {formatDecimal(value)}
            {asset && <span className="text-muted-foreground"> {asset}</span>}
        </>
    );
}

function SideText({ order }: { order: Order }) {
    return (
        <span className={order.side === "buy" ? "text-gain" : "text-loss"}>
            {SIDE_LABELS[order.side]}
        </span>
    );
}

// The order in words, like the toast after placing it: "Buy 0.25 BTC at 66,000.00 USD".
// Without the assets while the markets load.
function Summary({ order, market }: { order: Order; market: Market | undefined }) {
    if (market)
        return (
            <>
                <SideText order={order} /> {describe(order, market)}
            </>
        );
    const amount = formatTrimmed(order.base_quantity ?? order.quote_quantity ?? "0");
    return (
        <>
            <SideText order={order} /> {amount}
            {order.limit_price && ` at ${formatDecimal(order.limit_price)}`}
        </>
    );
}

// One order as two lines, for narrow boxes: what it is, then when, and how much of it
// has filled while it can still trade (a finished order's status says the rest).
function OrderListRow({
    order,
    market,
    cancellable,
}: {
    order: Order;
    market: Market | undefined;
    cancellable: boolean;
}) {
    const navigate = useNavigate();
    const to = `/orders/${order.order_id}`;
    const filling = isOpen(order) && /[1-9]/.test(order.filled_base_quantity);

    return (
        // the whole row opens the order; its summary is the link for keyboard users
        <li
            className="flex cursor-pointer items-start gap-3 py-3"
            onClick={() => navigate(to)}>
            <div className="min-w-0 flex-1">
                <Link
                    to={to}
                    className="font-medium hover:underline"
                    onClick={(event) => event.stopPropagation()}>
                    <Summary order={order} market={market} />
                </Link>
                <div className="mt-0.5 flex flex-wrap gap-x-3 text-sm text-muted-foreground">
                    <span>{formatWhen(order.created_at)}</span>
                    {filling && (
                        <span>
                            {formatTrimmed(order.filled_base_quantity)}{" "}
                            {market?.base_asset.symbol} filled
                        </span>
                    )}
                </div>
            </div>
            <div className="flex shrink-0 flex-col items-end gap-1.5">
                <OrderStateBadge order={order} />
                {cancellable && isOpen(order) && <CancelOrderButton />}
            </div>
        </li>
    );
}

function OrderTableRow({
    order,
    market,
    showMarket,
    cancellable,
    extra,
}: {
    order: Order;
    // undefined while the markets are loading
    market: Market | undefined;
    showMarket: boolean;
    cancellable: boolean;
    // the classes of the columns that need more room (see LAYOUTS)
    extra: string;
}) {
    const navigate = useNavigate();
    // in a one-market table the assets are in the headings instead
    const base = showMarket ? market?.base_asset.symbol : undefined;
    const quote = showMarket ? market?.quote_asset.symbol : undefined;

    return (
        // the whole row opens the order; the time is the link for keyboard users
        <TableRow
            className="cursor-pointer"
            onClick={() => navigate(`/orders/${order.order_id}`)}>
            <TableCell>
                <Link
                    to={`/orders/${order.order_id}`}
                    className="hover:underline"
                    onClick={(event) => event.stopPropagation()}>
                    {formatWhen(order.created_at)}
                </Link>
            </TableCell>
            {showMarket && (
                <TableCell className="font-semibold">{order.market}</TableCell>
            )}
            <TableCell>
                <SideText order={order} />
            </TableCell>
            <TableCell className={extra}>{ORDER_TYPE_LABELS[order.order_type]}</TableCell>
            <TableCell className={NUMBER}>
                {order.limit_price ? (
                    <Amount value={order.limit_price} asset={quote} />
                ) : (
                    <span className="text-muted-foreground">Market</span>
                )}
            </TableCell>
            <TableCell className={NUMBER}>
                {/* market buys say how much of the quote asset to spend, so they always
                    name it */}
                {order.base_quantity ? (
                    <Amount value={order.base_quantity} asset={base} />
                ) : order.quote_quantity ? (
                    <Amount
                        value={order.quote_quantity}
                        asset={market?.quote_asset.symbol}
                    />
                ) : null}
            </TableCell>
            <TableCell className={cn(NUMBER, extra)}>
                <Amount value={order.filled_base_quantity} asset={base} />
            </TableCell>
            <TableCell>
                <OrderStateBadge order={order} />
            </TableCell>
            {cancellable && (
                <TableCell className="text-right">
                    {isOpen(order) && <CancelOrderButton />}
                </TableCell>
            )}
        </TableRow>
    );
}

// One page of orders, with loading, empty and error states. The caller owns the query
// and puts filters and paging around it.
// - `market`: every order is in this market, so its assets go in the headings.
// - `markets` instead: the orders can be in any market (all markets, to look them up). The
//   table gets a Market column and each number names its asset.
// - `cancellable`: the acting user's own orders, which get "Cancel" while they're open.
// - `errorOverrides`: wording for error statuses, as in errorMessage().
export function OrdersTable({
    result,
    market,
    markets,
    empty,
    cancellable = false,
    errorOverrides,
}: {
    result: OrdersResult;
    market?: Market;
    markets?: Market[];
    empty: ReactNode;
    cancellable?: boolean;
    errorOverrides?: ErrorOverrides;
}) {
    const showMarket = !market;
    const layout = showMarket ? LAYOUTS.anyMarket : LAYOUTS.oneMarket;
    const marketBySymbol = new Map(markets?.map((m) => [m.symbol, m]));
    const marketOf = (order: Order) => market ?? marketBySymbol.get(order.market);

    // each column's classes, shared by the headings and the loading rows
    const columns = [
        "",
        ...(showMarket ? [""] : []),
        "",
        layout.extra,
        NUMBER,
        NUMBER,
        cn(NUMBER, layout.extra),
        "",
        ...(cancellable ? [""] : []),
    ];

    if (result.isError) {
        return (
            <p role="alert" className="py-4 text-destructive">
                Couldn't load the orders: {errorMessage(result.error, errorOverrides)}
            </p>
        );
    }
    if (result.data && result.data.items.length === 0) {
        return <div className="py-4 text-muted-foreground">{empty}</div>;
    }

    return (
        <div className="@container">
            <ul className={cn("divide-y", layout.rows)}>
                {result.data
                    ? result.data.items.map((order) => (
                          <OrderListRow
                              key={order.order_id}
                              order={order}
                              market={marketOf(order)}
                              cancellable={cancellable}
                          />
                      ))
                    : [0, 1, 2].map((row) => (
                          <li key={row} className="flex items-start gap-3 py-3">
                              <div className="grid flex-1 gap-1.5">
                                  <Skeleton className="h-4 w-48" />
                                  <Skeleton className="h-3.5 w-20" />
                              </div>
                              <Skeleton className="h-5 w-16" />
                          </li>
                      ))}
            </ul>
            <div className={layout.table}>
                <Table className="font-stretch-semi-condensed">
                    <TableHeader>
                        <TableRow>
                            <TableHead>Time</TableHead>
                            {showMarket && <TableHead>Market</TableHead>}
                            <TableHead>Side</TableHead>
                            <TableHead className={layout.extra}>Type</TableHead>
                            <TableHead className={NUMBER}>
                                Price{market && ` (${market.quote_asset.symbol})`}
                            </TableHead>
                            <TableHead className={NUMBER}>
                                Amount{market && ` (${market.base_asset.symbol})`}
                            </TableHead>
                            <TableHead className={cn(NUMBER, layout.extra)}>
                                Filled
                            </TableHead>
                            <TableHead>Status</TableHead>
                            {cancellable && (
                                <TableHead>
                                    <span className="sr-only">Actions</span>
                                </TableHead>
                            )}
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {result.data
                            ? result.data.items.map((order) => (
                                  <OrderTableRow
                                      key={order.order_id}
                                      order={order}
                                      market={marketOf(order)}
                                      showMarket={showMarket}
                                      cancellable={cancellable}
                                      extra={layout.extra}
                                  />
                              ))
                            : [0, 1, 2].map((row) => (
                                  <TableRow key={row}>
                                      {columns.map((className, cell) => (
                                          <TableCell key={cell} className={className}>
                                              <Skeleton className="h-4 w-full" />
                                          </TableCell>
                                      ))}
                                  </TableRow>
                              ))}
                    </TableBody>
                </Table>
            </div>
        </div>
    );
}
