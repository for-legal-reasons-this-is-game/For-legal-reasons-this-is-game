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
import { formatDecimal, formatWhen } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";
import { CancelOrderButton } from "./CancelOrderButton";
import { ORDER_TYPE_LABELS, SIDE_LABELS } from "./labels";
import { isOpen } from "./order-state";
import { OrderStateBadge } from "./OrderStateBadge";

const NUMBER = "text-right";
// Columns phones leave out: the market moves under the time, the order type and the
// filled amount are on the order's page
const MARKET_COLUMN = "hidden sm:table-cell";
const TYPE_COLUMN = "hidden md:table-cell";
const FILLED_COLUMN = "hidden sm:table-cell";

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

function OrderRow({
    order,
    market,
    showMarket,
    cancellable,
}: {
    order: Order;
    // undefined while the markets are loading
    market: Market | undefined;
    showMarket: boolean;
    cancellable: boolean;
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
                {showMarket && (
                    <div className="text-xs text-muted-foreground sm:hidden">
                        {order.market}
                    </div>
                )}
            </TableCell>
            {showMarket && (
                <TableCell className={cn(MARKET_COLUMN, "font-semibold")}>
                    {order.market}
                </TableCell>
            )}
            <TableCell className={order.side === "buy" ? "text-gain" : "text-loss"}>
                {SIDE_LABELS[order.side]}
            </TableCell>
            <TableCell className={TYPE_COLUMN}>
                {ORDER_TYPE_LABELS[order.order_type]}
            </TableCell>
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
            <TableCell className={cn(NUMBER, FILLED_COLUMN)}>
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
    const marketBySymbol = new Map(markets?.map((m) => [m.symbol, m]));

    // each column's classes, shared by the headings and the loading rows
    const columns = [
        "",
        ...(showMarket ? [MARKET_COLUMN] : []),
        "",
        TYPE_COLUMN,
        NUMBER,
        NUMBER,
        cn(NUMBER, FILLED_COLUMN),
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
        <Table className="font-stretch-semi-condensed">
            <TableHeader>
                <TableRow>
                    <TableHead>Time</TableHead>
                    {showMarket && (
                        <TableHead className={MARKET_COLUMN}>Market</TableHead>
                    )}
                    <TableHead>Side</TableHead>
                    <TableHead className={TYPE_COLUMN}>Type</TableHead>
                    <TableHead className={NUMBER}>
                        Price{market && ` (${market.quote_asset.symbol})`}
                    </TableHead>
                    <TableHead className={NUMBER}>
                        Amount{market && ` (${market.base_asset.symbol})`}
                    </TableHead>
                    <TableHead className={cn(NUMBER, FILLED_COLUMN)}>Filled</TableHead>
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
                          <OrderRow
                              key={order.order_id}
                              order={order}
                              market={market ?? marketBySymbol.get(order.market)}
                              showMarket={showMarket}
                              cancellable={cancellable}
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
    );
}
