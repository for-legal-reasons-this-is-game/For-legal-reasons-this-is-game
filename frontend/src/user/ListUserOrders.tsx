import { useState } from "react";
import { api, errorMessage } from "../api";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { PageControls } from "../common/PageControls";
import { OrderInfo } from "../order/OrderLookup";
import { OrderStatusSelect } from "../order/OrderStatusSelect";
import type { OrderQuery } from "../types/OrderQuery";

export function ListUserOrders({ userID }: { userID: string }) {
    const [query, setQuery] = useState<OrderQuery>({
        market: null,
        status: null,
        limit: null,
        cursor: null,
    });

    // Two queries in one component: this result is kept as one object (markets.data,
    // markets.isError, ...) instead of being destructured, so its names don't clash
    // with the orders query below.
    // It uses the same key as ListMarkets, so both share one cached markets list:
    // whichever opens second shows the cached list instantly.
    const markets = useQuery({
        queryKey: ["markets"],
        queryFn: () => api.markets.list(),
    });

    const { isFetching, isError, isSuccess, isPlaceholderData, data, error } = useQuery({
        queryKey: ["users", userID, "orders", query],
        queryFn: () => api.users.orders(userID, query),
        enabled: !!userID,
        // keeps the current page on screen while the next one loads
        // (explained in ListAccountTransfers)
        placeholderData: keepPreviousData,
    });

    // after the hooks: hooks must run on every render, so an early return has to
    // come after all of them
    if (!userID) {
        return (
            <div style={{ color: "red" }}>
                No user selected. Click "Select User" in List Users first
            </div>
        );
    }

    return (
        <>
            <h3>Orders of user __{userID}__</h3>
            {/* a new filter starts again from the first page.
                "" is "all markets": `|| null` turns it into null, which isn't sent */}
            <select
                value={query.market ?? ""}
                onChange={(event) =>
                    setQuery({
                        ...query,
                        market: event.target.value || null,
                        cursor: null,
                    })
                }>
                <option value="">all markets</option>
                {markets.data?.map((market) => (
                    <option key={market.market_id} value={market.symbol}>
                        {market.symbol}
                    </option>
                ))}
            </select>
            <OrderStatusSelect
                status={query.status}
                onChange={(status) => setQuery({ ...query, status, cursor: null })}
            />
            <PageControls
                query={query}
                setQuery={setQuery}
                nextCursor={data?.next_cursor}
                isPlaceholderData={isPlaceholderData}
            />
            {markets.isError && (
                <div style={{ color: "red" }}>
                    Couldn't load the markets filter: {errorMessage(markets.error)}
                </div>
            )}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, { 404: "This user doesn't exist" })}
                </div>
            )}
            {isFetching && <div>Loading...</div>}
            {isSuccess && data.items.length === 0 && <div>No orders</div>}
            {isSuccess &&
                data.items.map((order) => (
                    <div key={order.order_id}>
                        <OrderInfo order={order} />
                        <br />
                    </div>
                ))}
        </>
    );
}
