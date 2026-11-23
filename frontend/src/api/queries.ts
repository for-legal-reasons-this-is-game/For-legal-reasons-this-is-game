// Every query's key and fetch function in one place, so components asking for the same
// data use the same key and share one cache entry. Use them as
// useQuery(marketQueries.ticker(symbol)).
//
// New TanStack: queryOptions() only bundles the options (it fetches nothing). Its point is
// typing: the key and the data type stay linked wherever the options are reused, for
// example in queryClient.invalidateQueries({ queryKey: marketQueries.list().queryKey }).
//
// Keys nest, so invalidating a prefix refreshes everything under it:
//   ["markets"]                              the list
//   ["markets", symbol]                      one market, and below it:
//   ["markets", symbol, "ticker" | "trades" | "orders", ...]
//   ["users"], ["users", id, "orders", query]
//   ["account", id], ["position", id]        (the old components use these keys too)
import { keepPreviousData, queryOptions } from "@tanstack/react-query";
import { api } from "./index";
import type { MarketOrderQuery } from "@/types/MarketOrderQuery";
import type { MarketTradeQuery } from "@/types/MarketTradeQuery";
import type { OrderQuery } from "@/types/OrderQuery";
import type { Trade } from "@/types/Trade";

// How often live market data refreshes. There's no push from the backend yet (the
// engine's market-data stream isn't implemented), so pages poll.
const LIVE_MS = 5_000;

// The newest trades, up to `pages` pages of 100, for building chart candles.
async function fetchRecentTrades(symbol: string, pages = 5): Promise<Trade[]> {
    const trades: Trade[] = [];
    let cursor: string | null = null;
    for (let page = 0; page < pages; page++) {
        const result = await api.markets.trades(symbol, { limit: 100, cursor });
        trades.push(...result.items);
        if (!result.next_cursor) break;
        cursor = result.next_cursor;
    }
    return trades;
}

export const marketQueries = {
    list: () =>
        queryOptions({
            queryKey: ["markets"],
            queryFn: () => api.markets.list(),
        }),
    detail: (symbol: string) =>
        queryOptions({
            queryKey: ["markets", symbol],
            queryFn: () => api.markets.get(symbol),
        }),
    ticker: (symbol: string) =>
        queryOptions({
            queryKey: ["markets", symbol, "ticker"],
            queryFn: () => api.markets.ticker(symbol),
            refetchInterval: LIVE_MS,
        }),
    // for the chart: the latest trades, refreshed while the page is open
    recentTrades: (symbol: string) =>
        queryOptions({
            queryKey: ["markets", symbol, "trades", "recent"],
            queryFn: () => fetchRecentTrades(symbol),
            refetchInterval: LIVE_MS,
        }),
    // one page of trades, for the trades table
    trades: (symbol: string, query: MarketTradeQuery) =>
        queryOptions({
            queryKey: ["markets", symbol, "trades", query],
            queryFn: () => api.markets.trades(symbol, query),
            placeholderData: keepPreviousData,
            refetchInterval: LIVE_MS,
        }),
    orders: (symbol: string, query: MarketOrderQuery) =>
        queryOptions({
            queryKey: ["markets", symbol, "orders", query],
            queryFn: () => api.markets.orders(symbol, query),
            placeholderData: keepPreviousData,
            refetchInterval: LIVE_MS,
        }),
};

export const userQueries = {
    list: () =>
        queryOptions({
            queryKey: ["users"],
            queryFn: () => api.users.list(),
        }),
    orders: (userID: string, query: OrderQuery) =>
        queryOptions({
            queryKey: ["users", userID, "orders", query],
            queryFn: () => api.users.orders(userID, query),
            enabled: !!userID,
            placeholderData: keepPreviousData,
            refetchInterval: LIVE_MS,
        }),
};

export const accountQueries = {
    detail: (accountID: string) =>
        queryOptions({
            queryKey: ["account", accountID],
            queryFn: () => api.accounts.get(accountID),
            enabled: !!accountID,
        }),
    // the balance; placing an order or a deposit invalidates ["position"]
    position: (accountID: string) =>
        queryOptions({
            queryKey: ["position", accountID],
            queryFn: () => api.accounts.position(accountID),
            enabled: !!accountID,
        }),
};
