import { useState } from "react";
import { api, errorMessage } from "../api";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { PageControls } from "../common/PageControls";
import type { Trade } from "../types/Trade";
import type { MarketTradeQuery } from "../types/MarketTradeQuery";

export function TradeInfo({ trade }: { trade: Trade }) {
    return (
        <>
            <div>Trade ID: {trade.trade_id} </div>
            <div>Market: {trade.market} </div>
            <div>Price: {trade.price} </div>
            <div>Base Quantity: {trade.base_quantity} </div>
            <div>Quote Quantity: {trade.quote_quantity} </div>
            <div>Taker Side: {trade.taker_side} </div>
            <div>Buyer Order: {trade.buyer_order_id} </div>
            <div>Buyer Fee: {trade.buyer_fee} </div>
            <div>Seller Order: {trade.seller_order_id} </div>
            <div>Seller Fee: {trade.seller_fee} </div>
            <div>Time: {trade.engine_time} </div>
        </>
    );
}

export function ListMarketTrades({ symbol }: { symbol: string }) {
    const [query, setQuery] = useState<MarketTradeQuery>({
        limit: null,
        cursor: null,
    });

    const { isFetching, isError, isSuccess, isPlaceholderData, data, error } = useQuery({
        queryKey: ["markets", symbol, "trades", query],
        queryFn: () => api.markets.trades(symbol, query),
        // keeps the current page on screen while the next one loads
        // (explained in ListAccountTransfers)
        placeholderData: keepPreviousData,
    });

    return (
        <>
            <h3>Trades in {symbol}</h3>
            <PageControls
                query={query}
                setQuery={setQuery}
                nextCursor={data?.next_cursor}
                isPlaceholderData={isPlaceholderData}
            />
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
            {isSuccess && data.items.length === 0 && <div>No trades yet</div>}
            {isSuccess &&
                data.items.map((trade) => (
                    <div key={trade.trade_id}>
                        <TradeInfo trade={trade} />
                        <br />
                    </div>
                ))}
        </>
    );
}
