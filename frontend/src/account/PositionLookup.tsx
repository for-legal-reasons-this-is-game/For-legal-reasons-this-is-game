import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";

export function PositionLookup({ accountID }: { accountID: string }) {
    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["position", accountID],
        queryFn: () => api.accounts.position(accountID),
        enabled: !!accountID,
    });

    return (
        <>
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
            {isSuccess && (
                <>
                    <div>Received Position...</div>
                    <div>Account ID: {data.account_id} </div>
                    <div>Account Status: {data.account_status} </div>
                    <div>Symbol: {data.symbol} </div>
                    <div>Decimals ID: {data.decimals} </div>
                    <div>Debits Posted: {data.debits_posted} </div>
                    <div>Credits Posted: {data.credits_posted} </div>
                    <div>Debits Pending: {data.debits_pending} </div>
                    <div>Credits Pending: {data.credits_pending} </div>
                    <div>Net Posted: {data.net_posted} </div>
                </>
            )}
        </>
    );
}
