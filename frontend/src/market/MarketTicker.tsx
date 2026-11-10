import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";

export function MarketTicker({ symbol }: { symbol: string }) {
    const { isLoading, isError, isSuccess, data, error } = useQuery({
        queryKey: ["markets", symbol, "ticker"],
        queryFn: () => api.markets.ticker(symbol),
        // New TanStack: refetchInterval re-runs the query every 5 seconds while this
        // component is on screen, so prices stay current without a Refresh button.
        // It pauses automatically while the browser tab is in the background.
        refetchInterval: 5000,
    });

    // New TanStack: isLoading is only true during the *first* load (no data yet AND
    // fetching). isFetching would also be true on every 5-second refresh, which would
    // make "Loading..." flash.
    // The open/high/low/change values are null when there were no trades in the last
    // 24 hours, and last_price when the market never had a trade.
    // change_percent arrives with ~25 decimals, so it's rounded to 2 for display.
    // Number() loses precision on big or very precise values: fine for a percentage
    // on screen, but never do it with amounts you calculate with (keep those strings).
    return (
        <>
            <h3>Ticker for {symbol} (last 24h)</h3>
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isLoading && <div>Loading...</div>}
            {isSuccess && (
                <>
                    <div>Last Price: {data.last_price ?? "no trades yet"} </div>
                    <div>
                        Change: {data.change ?? "-"} (
                        {data.change_percent === null
                            ? "-"
                            : Number(data.change_percent).toFixed(2)}
                        %)
                    </div>
                    <div>Open: {data.open ?? "-"} </div>
                    <div>High: {data.high ?? "-"} </div>
                    <div>Low: {data.low ?? "-"} </div>
                    <div>Base Volume: {data.base_volume} </div>
                    <div>Quote Volume: {data.quote_volume} </div>
                    <div>Trade Count: {data.trade_count} </div>
                </>
            )}
        </>
    );
}
