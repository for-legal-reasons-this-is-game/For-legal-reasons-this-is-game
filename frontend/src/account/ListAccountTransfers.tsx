import { useState, type ChangeEvent } from "react";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";
import { type Transfer } from "../types/Transfer";
import { type TransferQuery } from "../types/TransferQuery";

function Transfer({ transfer }: { transfer: Transfer }) {
    return (
        <>
            <span>Id: {transfer.transfer_id}</span>
            <span>Amount: {transfer.amount}</span>
            <span>Direction: {transfer.direction}</span>
            <span>Kind: {transfer.kind}</span>
            <span>Time: {transfer.timestamp}</span>
        </>
    );
}

export function ListAccountTransfers({ accountID }: { accountID: string }) {
    const [query, setQuery] = useState<TransferQuery>({
        limit: null,
        cursor: null,
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["transfers", "account", accountID, query],
        queryFn: () => api.accounts.transfers(accountID, query),
    });

    function handleChange(event: ChangeEvent<HTMLSelectElement>) {
        setValidationError(null);
        setQuery((prev) => ({
            ...prev,
            limit: Number(event.target.value),
        }));
        console.log(event.target.value);
    }

    return (
        <>
            <h3>Account Transfers for account __{accountID}__</h3>
            <div>Transfers</div>
            <select onChange={handleChange}>
                <option itemType="number" value="10">
                    10
                </option>
                <option itemType="number" value="20">
                    20
                </option>
                <option itemType="number" value="30">
                    30
                </option>
                <option itemType="number" value="40">
                    40
                </option>
                <option itemType="number" value="50">
                    50
                </option>
                <option itemType="number" value="60">
                    60
                </option>
                <option itemType="number" value="70">
                    70
                </option>
                <option itemType="number" value="80">
                    80
                </option>
                <option itemType="number" value="90">
                    90
                </option>
                <option itemType="number" value="100">
                    100
                </option>
            </select>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
            {isSuccess &&
                setQuery((prev) => ({
                    ...prev,
                    cursor: data.next_cursor,
                })) &&
                data?.items.map((transfer) => (
                    <div key={transfer.transfer_id}>
                        <Transfer transfer={transfer} />
                        <br />
                    </div>
                ))}
        </>
    );
}
