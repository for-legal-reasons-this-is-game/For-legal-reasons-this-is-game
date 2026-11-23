import type { ReactNode } from "react";
import { Link, useNavigate } from "react-router";
import { errorMessage, type ApiError } from "@/api";
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
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";
import { ORDER_TYPE_LABELS, SIDE_LABELS } from "./labels";
import { OrderStateBadge } from "./OrderStateBadge";

const NUMBER = "text-right";
const COLUMNS = 7;

// What a list query gives this table (a useQuery result fits)
type OrdersResult = {
    data: { items: Order[] } | undefined;
    isPending: boolean;
    isError: boolean;
    error: ApiError | null;
};

function OrderRow({ order, market }: { order: Order; market: Market }) {
    const navigate = useNavigate();
    const base = market.base_asset.symbol;
    const quote = market.quote_asset.symbol;
    const muted = (text: string) => <span className="text-muted-foreground">{text}</span>;

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
            <TableCell className={order.side === "buy" ? "text-gain" : "text-loss"}>
                {SIDE_LABELS[order.side]}
            </TableCell>
            <TableCell>{ORDER_TYPE_LABELS[order.order_type]}</TableCell>
            <TableCell className={NUMBER}>
                {order.limit_price ? formatDecimal(order.limit_price) : muted("Market")}
            </TableCell>
            <TableCell className={NUMBER}>
                {order.base_quantity ? (
                    <>
                        {formatDecimal(order.base_quantity)} {muted(base)}
                    </>
                ) : order.quote_quantity ? (
                    // market buys say how much to spend
                    <>
                        {formatDecimal(order.quote_quantity)} {muted(quote)}
                    </>
                ) : null}
            </TableCell>
            <TableCell className={NUMBER}>
                {formatDecimal(order.filled_base_quantity)}
            </TableCell>
            <TableCell>
                <OrderStateBadge order={order} />
            </TableCell>
        </TableRow>
    );
}

// One page of orders in a market, with loading, empty and error states. The caller
// owns the query and puts filters and paging around it.
export function OrdersTable({
    result,
    market,
    empty,
}: {
    result: OrdersResult;
    market: Market;
    empty: ReactNode;
}) {
    if (result.isError) {
        return (
            <p role="alert" className="py-4 text-destructive">
                Couldn't load the orders: {errorMessage(result.error)}
            </p>
        );
    }
    if (result.data && result.data.items.length === 0) {
        return <p className="py-4 text-muted-foreground">{empty}</p>;
    }

    return (
        <Table className="font-stretch-semi-condensed">
            <TableHeader>
                <TableRow>
                    <TableHead>Time</TableHead>
                    <TableHead>Side</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead className={NUMBER}>Price</TableHead>
                    <TableHead className={NUMBER}>Amount</TableHead>
                    <TableHead className={NUMBER}>Filled</TableHead>
                    <TableHead>Status</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {result.data
                    ? result.data.items.map((order) => (
                          <OrderRow key={order.order_id} order={order} market={market} />
                      ))
                    : [0, 1, 2].map((row) => (
                          <TableRow key={row}>
                              {Array.from({ length: COLUMNS }, (_, cell) => (
                                  <TableCell key={cell}>
                                      <Skeleton className="h-4 w-full" />
                                  </TableCell>
                              ))}
                          </TableRow>
                      ))}
            </TableBody>
        </Table>
    );
}
