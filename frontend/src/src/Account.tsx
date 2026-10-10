import { useState, type FormEvent, type ChangeEvent } from "react";
import { type Account } from "./types/Account";
import { api, errorMessage } from "./api";

export function Account() {
    const [accountID, setAccountID] = useState<string>("");
    const [account, setAccount] = useState<Account | null>(null);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setError(null);
        try {
            if (!accountID) throw Error("AccountID field can't be empty");
            setAccount(await api.accounts.get(accountID));
        } catch (e) {
            setError(errorMessage(e));
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setAccountID(event.target.value);
    }

    return (
        <>
            <h3>Account</h3>
            <form onSubmit={handleSubmit}>
                <input onChange={handleChange} placeholder="AccountID" />
                <button type="submit">Get Account</button>
            </form>

            {error && <div style={{ color: "red" }}>{error}</div>}
            {account && (
                <>
                    <div>Received account...</div>
                    <div>Name: {account.account_name} </div>
                    <div>ID: {account.account_id} </div>
                    <div>Code Type: {account.account_code_type} </div>
                    <div>Ledger ID: {account.account_ledger_id} </div>
                    <div>Status: {account.account_status} </div>
                    <div>User ID: {account.account_user_id} </div>
                </>
            )}
        </>
    );
}
