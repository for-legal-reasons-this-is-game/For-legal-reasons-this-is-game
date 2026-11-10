import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";
import { TradeInfo } from "../market/ListMarketTrades";

export function ListOrderTrades({ orderID }: { orderID: string }) {
    const { isFetching, isError, isSuccess, data, error, refetch } = useQuery({
        queryKey: ["orders", orderID, "trades"],
        queryFn: () => api.orders.trades(orderID),
    });

    return (
        <>
            <h3>Trades of order __{orderID}__</h3>
            <button onClick={() => refetch()}>Refresh</button>
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
            {isSuccess && data.length === 0 && <div>No trades for this order yet</div>}
            {isSuccess &&
                data.map((trade) => (
                    <div key={trade.trade_id}>
                        <TradeInfo trade={trade} />
                        <br />
                    </div>
                ))}
        </>
    );
}
