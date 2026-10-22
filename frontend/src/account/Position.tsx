import { type FormEvent, type ChangeEvent, useState } from "react";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";

export function Position() {
    // TODO: put PositionLookup in Account or AccountList, and pass to it AccounID
    const [accountID, setAccountID] = useState<string>("");
    const [submittedAccountID, setSubmittedAccountID] = useState("");
    const [validationError, setValidationError] = useState<string | null>(null);

    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["position", submittedAccountID],
        queryFn: () => api.accounts.position(submittedAccountID),
        enabled: !!submittedAccountID,
    });
    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (!accountID.trim()) {
            setValidationError("AccountID field can't be empty");
            return;
        }
        setSubmittedAccountID(accountID.trim());
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setValidationError(null);
        setAccountID(event.target.value);
    }

    return (
        <>
            <h3>Account Position</h3>
            <form onSubmit={handleSubmit}>
                <input
                    onChange={handleChange}
                    value={accountID}
                    placeholder="AccountID"
                />
                <button type="submit">Get Account Position</button>
            </form>
            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
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
