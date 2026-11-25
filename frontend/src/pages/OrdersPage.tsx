import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router";
import { errorMessage } from "@/api";
import { marketQueries, userQueries } from "@/api/queries";
import { useActingUser } from "@/app/acting-user/context";
import { EmptyState } from "@/components/EmptyState";
import { PageHeader } from "@/components/PageHeader";
import { TablePager } from "@/components/TablePager";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { STATUS_FILTER_LABELS } from "@/features/order/labels";
import { MarketFilterSelect } from "@/features/order/MarketFilterSelect";
import { OrdersTable } from "@/features/order/OrdersTable";
import { StatusFilterSelect } from "@/features/order/StatusFilterSelect";
import { ChooseUserHint } from "@/features/user/ChooseUserHint";
import type { OrderStatusFilter } from "@/types/OrderStatusFilter";

const APP_NAME = "For legal reasons this is game";

// New TypeScript: `value is OrderStatusFilter` makes this a type guard. Where it returns
// true, TypeScript treats `value` as an OrderStatusFilter from then on, so a word read
// from the address can be used as a filter without `as`.
function isStatusFilter(value: string | null): value is OrderStatusFilter {
    return value !== null && Object.hasOwn(STATUS_FILTER_LABELS, value);
}

function YourOrders({
    userID,
    market,
    status,
    clearFilters,
}: {
    userID: string;
    market: string | null;
    status: OrderStatusFilter | null;
    clearFilters: () => void;
}) {
    const markets = useQuery(marketQueries.list());
    const [page, setPage] = useState<{ limit: number | null; cursor: string | null }>({
        limit: 20,
        cursor: null,
    });
    // When the filters change, paging starts again from the first page (the cursor belongs
    // to the old list). React allows setting state during render for this: it renders
    // again with the new state before showing anything.
    const filters = `${market}|${status}`;
    const [pagedFilters, setPagedFilters] = useState(filters);
    if (pagedFilters !== filters) {
        setPagedFilters(filters);
        setPage({ ...page, cursor: null });
    }

    const orders = useQuery(userQueries.orders(userID, { market, status, ...page }));
    const filtered = market !== null || status !== null;

    if (
        !filtered &&
        page.cursor === null &&
        orders.data?.items.length === 0 &&
        !orders.isPlaceholderData
    ) {
        return (
            <EmptyState>
                <p>You haven't placed any orders yet. Choose a market to place one.</p>
                <Button asChild className="mt-4">
                    <Link to="/markets">Go to markets</Link>
                </Button>
            </EmptyState>
        );
    }

    return (
        <div className="flex flex-col gap-3">
            <div className="rounded-2xl border bg-card px-2">
                <OrdersTable
                    result={orders}
                    // filtered to one market, the table looks like the trade page's
                    market={markets.data?.find((m) => m.symbol === market)}
                    markets={markets.data}
                    cancellable
                    errorOverrides={{
                        404: "The user you're acting as doesn't exist any more. Choose another in the top bar.",
                    }}
                    empty={
                        <p className="px-2">
                            No orders match these filters.{" "}
                            <Button
                                variant="link"
                                className="h-auto p-0"
                                onClick={clearFilters}>
                                Show all orders
                            </Button>
                        </p>
                    }
                />
            </div>
            {/* nothing to page through when the first page is empty */}
            {!(page.cursor === null && orders.data?.items.length === 0) && (
                <TablePager
                    query={page}
                    setQuery={setPage}
                    nextCursor={orders.data?.next_cursor}
                    isPlaceholderData={orders.isPlaceholderData}
                />
            )}
        </div>
    );
}

export function OrdersPage() {
    const { userID } = useActingUser();
    const markets = useQuery(marketQueries.list());

    // New React Router: useSearchParams reads the address's query string
    // ("/orders?market=BTC-USD&status=open"), so a filtered list can be bookmarked, and
    // coming back from an order keeps the filters. Anything unexpected in it is ignored.
    const [params, setParams] = useSearchParams();
    const market = params.get("market") || null;
    const statusParam = params.get("status");
    const status = isStatusFilter(statusParam) ? statusParam : null;

    function setFilter(name: "market" | "status", value: string | null) {
        // a function of the current parameters, like setState's; `replace` swaps the
        // address instead of adding a history entry, so Back still leaves the page
        setParams(
            (current) => {
                const next = new URLSearchParams(current);
                if (value) next.set(name, value);
                else next.delete(name);
                return next;
            },
            { replace: true },
        );
    }

    const unknownMarket =
        market !== null &&
        markets.isSuccess &&
        !markets.data.some((m) => m.symbol === market);
    // with a market in the address, check it exists before asking for its orders (the
    // backend's 404 wouldn't say whether the market or the user is missing)
    const checkingMarket = market !== null && markets.isPending;

    return (
        <>
            <title>{`Your orders - ${APP_NAME}`}</title>
            <PageHeader
                title="Your orders"
                description="Every order you placed, in every market, newest first."
            />
            {!userID ? (
                <EmptyState>
                    <ChooseUserHint goal="to see their orders" />
                </EmptyState>
            ) : (
                <>
                    <div className="mb-3 flex flex-wrap gap-2">
                        <MarketFilterSelect
                            market={market}
                            markets={markets.data ?? []}
                            onChange={(symbol) => setFilter("market", symbol)}
                        />
                        <StatusFilterSelect
                            status={status}
                            onChange={(value) => setFilter("status", value)}
                        />
                    </div>
                    {markets.isError && (
                        <p role="alert" className="mb-3 text-destructive">
                            Couldn't load the markets: {errorMessage(markets.error)}
                        </p>
                    )}
                    {checkingMarket ? (
                        <Skeleton className="h-48 rounded-2xl" />
                    ) : unknownMarket ? (
                        <EmptyState>
                            <p>There's no market called {market}.</p>
                            <Button
                                className="mt-4"
                                onClick={() => setFilter("market", null)}>
                                Show every market
                            </Button>
                        </EmptyState>
                    ) : (
                        <YourOrders
                            userID={userID}
                            market={market}
                            status={status}
                            clearFilters={() => setParams({}, { replace: true })}
                        />
                    )}
                </>
            )}
        </>
    );
}
