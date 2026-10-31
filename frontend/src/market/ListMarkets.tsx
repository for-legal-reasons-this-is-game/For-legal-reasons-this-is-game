import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";
import { MarketInfo } from "./MarketLookup";

export function ListMarkets() {
    const { isFetching, isError, data, error, refetch } = useQuery({
        queryKey: ["markets"],
        queryFn: () => api.markets.list(),
    });

    return (
        <>
            <h3>Markets</h3>
            <button onClick={() => refetch()}>Refresh</button>
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
            {data?.map((market) => (
                <div key={market.market_id}>
                    <MarketInfo market={market} />
                    <br />
                </div>
            ))}
        </>
    );
}
