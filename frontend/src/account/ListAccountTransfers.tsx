import { useState, type FormEvent, type ChangeEvent } from "react";
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
    // TODO: Do we even need this state? just use accountID directly
    const [submittedAccountID, setSubmittedAccountID] = useState("");
    const [query, setQuery] = useState<TransferQuery>({
        limit: null,
        cursor: null,
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["transfers", "account", submittedAccountID, query],
        queryFn: () => api.accounts.transfers(submittedAccountID, query),
        enabled: !!submittedAccountID || !!query.cursor || !!query.limit,
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (!accountID.trim()) {
            setValidationError("AccountID field can't be empty");
            return;
        }
        setSubmittedAccountID(accountID.trim());
    }

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
            <form onSubmit={handleSubmit}>
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
                <button type="submit">Get Transfers</button>
            </form>

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
