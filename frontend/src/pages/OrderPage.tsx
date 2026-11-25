import { useQuery } from "@tanstack/react-query";
import { ChevronLeftIcon } from "lucide-react";
import { Link, useParams } from "react-router";
import { errorMessage } from "@/api";
import { marketQueries, orderQueries, userQueries } from "@/api/queries";
import { useActingUser } from "@/app/acting-user/context";
import { CopyButton } from "@/components/CopyButton";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { CancelOrderButton } from "@/features/order/CancelOrderButton";
import { describe } from "@/features/order/describe";
import { FillsTable } from "@/features/order/FillsTable";
import {
    CANCEL_REASON_LABELS,
    ORDER_TYPE_LABELS,
    rejectionText,
    SIDE_LABELS,
} from "@/features/order/labels";
import { OrderDetails } from "@/features/order/OrderDetails";
import { isOpen } from "@/features/order/order-state";
import { OrderStateBadge } from "@/features/order/OrderStateBadge";
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";

const APP_NAME = "For legal reasons this is game";

// The order in one line, "Buy 0.05 BTC at 64,000.00 USD" (the same words as the toast
// after placing it); just "Buy order" while the market loads
function titleOf(order: Order, market: Market | undefined): string {
    const side = SIDE_LABELS[order.side];
    return market ? `${side} ${describe(order, market)}` : `${side} order`;
}

// Why a finished order ended, when there's something to say
function Outcome({ order }: { order: Order }) {
    let text: string | null = null;
    if (order.submission === "rejected") {
        // the engine's own detail, when it gave one, says more about its reason
        const detail =
            order.reject_reason && order.reject_detail ? ` (${order.reject_detail})` : "";
        text = `Rejected. ${rejectionText(order)}${detail}`;
    } else if (order.status === "cancelled") {
        text = order.cancel_reason
            ? `Cancelled. ${CANCEL_REASON_LABELS[order.cancel_reason]}`
            : "Cancelled.";
    } else if (order.status === "expired") {
        text = "Expired. Its time in force ran out before it filled completely.";
    }
    if (!text) return null;
    return (
        <p
            className={
                order.submission === "rejected"
                    ? "mb-4 max-w-[70ch] rounded-2xl border border-destructive/40 bg-card p-4 text-destructive"
                    : "mb-4 max-w-[70ch] rounded-2xl border bg-card p-4"
            }>
            {text}
        </p>
    );
}

function OrderView({ order }: { order: Order }) {
    const { userID } = useActingUser();
    const markets = useQuery(marketQueries.list());
    const users = useQuery(userQueries.list());
    const market = markets.data?.find((m) => m.symbol === order.market);
    const mine = order.user_id === userID;
    const owner = mine
        ? "you"
        : (users.data?.find((user) => user.user_id === order.user_id)?.user_name ??
          "another user");
    const title = titleOf(order, market);

    return (
        <>
            <title>{`${title} - ${APP_NAME}`}</title>
            {/* your own orders came from the orders page; anyone else's from the market */}
            <Link
                to={mine ? "/orders" : `/markets/${order.market}`}
                className="mb-2 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
                <ChevronLeftIcon className="size-4" />
                {mine ? "Your orders" : order.market}
            </Link>
            <PageHeader
                title={title}
                description={
                    <>
                        {ORDER_TYPE_LABELS[order.order_type]} order in {order.market},
                        placed by {owner}.
                        <span className="mt-1 flex items-center gap-1 text-sm">
                            ID {order.order_id}
                            <CopyButton value={order.order_id} what="order ID" />
                        </span>
                    </>
                }
                actions={
                    <>
                        <OrderStateBadge order={order} />
                        {mine && isOpen(order) && <CancelOrderButton size="default" />}
                    </>
                }
            />
            <Outcome order={order} />
            {/* grid-cols-1 is minmax(0, 1fr): on phones a wide table scrolls inside its
                panel instead of widening the page */}
            <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,24rem)_minmax(0,1fr)] lg:items-start">
                <section
                    aria-labelledby="order-details"
                    className="rounded-2xl border bg-card p-4">
                    <h2 id="order-details" className="mb-1 font-semibold">
                        Details
                    </h2>
                    <OrderDetails order={order} market={market} />
                </section>
                <section
                    aria-labelledby="order-fills"
                    className="rounded-2xl border bg-card p-4">
                    <h2 id="order-fills" className="mb-1 font-semibold">
                        Fills
                    </h2>
                    {market ? (
                        <FillsTable order={order} market={market} />
                    ) : markets.isError ? (
                        <p role="alert" className="text-destructive">
                            Couldn't load the market: {errorMessage(markets.error)}
                        </p>
                    ) : (
                        <Skeleton className="h-24" />
                    )}
                </section>
            </div>
        </>
    );
}

export function OrderPage() {
    // New React Router: useParams() reads the ":orderId" part of the route's path
    const { orderId = "" } = useParams();
    const order = useQuery(orderQueries.detail(orderId));

    if (order.isPending) {
        return (
            <div className="grid gap-4">
                <Skeleton className="h-10 w-96 max-w-full" />
                <Skeleton className="h-64" />
            </div>
        );
    }
    if (order.isError) {
        // 400: not even an order ID (the backend can't read it as one)
        const missing = order.error.status === 404 || order.error.status === 400;
        return (
            <>
                <title>{`Order - ${APP_NAME}`}</title>
                {missing ? (
                    <>
                        <PageHeader
                            title="Order not found"
                            description={`There's no order with the ID ${orderId}.`}
                        />
                        <Button asChild>
                            <Link to="/orders">Go to your orders</Link>
                        </Button>
                    </>
                ) : (
                    <p role="alert" className="text-destructive">
                        Couldn't load the order: {errorMessage(order.error)}
                    </p>
                )}
            </>
        );
    }

    return <OrderView order={order.data} />;
}
