import { useState } from "react";
import { api, errorMessage } from "../api";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { PageControls } from "../common/PageControls";
import { OrderInfo } from "../order/OrderLookup";
import { OrderStatusSelect } from "../order/OrderStatusSelect";
import type { MarketOrderQuery } from "../types/MarketOrderQuery";

export function ListMarketOrders({ symbol }: { symbol: string }) {
    const [query, setQuery] = useState<MarketOrderQuery>({
        status: null,
        limit: null,
        cursor: null,
    });

    const { isFetching, isError, isSuccess, isPlaceholderData, data, error } = useQuery({
        queryKey: ["markets", symbol, "orders", query],
        queryFn: () => api.markets.orders(symbol, query),
        // keeps the current page on screen while the next one loads
        // (explained in ListAccountTransfers)
        placeholderData: keepPreviousData,
    });

    return (
        <>
            <h3>Orders in {symbol}</h3>
            {/* a new filter starts again from the first page */}
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
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
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
