import { useState } from "react";
import { api, errorMessage } from "../api";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { PageControls } from "../common/PageControls";
import { type Transfer } from "../types/Transfer";
import { type TransferQuery } from "../types/TransferQuery";

function Transfer({ transfer }: { transfer: Transfer }) {
    return (
        <>
            <div>Id: {transfer.transfer_id}</div>
            <div>Amount: {transfer.amount}</div>
            <div>Direction: {transfer.direction}</div>
            <div>Kind: {transfer.kind}</div>
            <div>Time: {transfer.timestamp}</div>
        </>
    );
}

export function ListAccountTransfers({ accountID }: { accountID: string }) {
    const [query, setQuery] = useState<TransferQuery>({
        limit: null,
        cursor: null,
    });

    const { isFetching, isError, isSuccess, isPlaceholderData, data, error } = useQuery({
        queryKey: ["transfers", "account", accountID, query],
        queryFn: () => api.accounts.transfers(accountID, query),
        // New TanStack: every page has its own query key (the cursor is in `query`),
        // so moving to the next page would normally empty the list until it loads.
        // keepPreviousData keeps showing the old page meanwhile; isPlaceholderData
        // is true during that time.
        placeholderData: keepPreviousData,
    });

    return (
        <>
            <h3>Account Transfers for account __{accountID}__</h3>
            <PageControls
                query={query}
                setQuery={setQuery}
                nextCursor={data?.next_cursor}
                isPlaceholderData={isPlaceholderData}
            />
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
            {isSuccess && data.items.length === 0 && <div>No transfers yet</div>}
            {isSuccess &&
                data.items.map((transfer) => (
                    <div key={transfer.transfer_id}>
                        <Transfer transfer={transfer} />
                        <br />
                    </div>
                ))}
        </>
    );
}
