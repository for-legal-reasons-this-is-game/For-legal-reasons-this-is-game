import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";

export function Account() {
    const [accountID, setAccountID] = useState<string>("");
    const [submittedAccountID, setSubmittedAccountID] = useState("");
    const [validationError, setValidationError] = useState<string | null>(null);

    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["account", submittedAccountID],
        queryFn: () => api.accounts.get(submittedAccountID),
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
            <h3>Account</h3>
            <form onSubmit={handleSubmit}>
                <input
                    onChange={handleChange}
                    value={accountID}
                    placeholder="AccountID"
                />
                <button type="submit">Get Account</button>
            </form>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
            {isSuccess && (
                <>
                    <div>Received account...</div>
                    <div>Name: {data.account_name} </div>
                    <div>ID: {data.account_id} </div>
                    <div>Code Type: {data.account_code_type} </div>
                    <div>Ledger ID: {data.account_ledger_id} </div>
                    <div>Status: {data.account_status} </div>
                    <div>User ID: {data.account_user_id} </div>
                </>
            )}
        </>
    );
}
