import { type FormEvent, type ChangeEvent, useState } from "react";
import { type Position } from "./types/Position";
import { api, errorMessage } from "./api";

export function Position() {
    const [accountID, setAccountID] = useState<string>("");
    const [position, setPosition] = useState<Position | null>(null);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setError(null);
        try {
            if (!accountID) throw Error("AccountID field can't be empty");
            setPosition(await api.accounts.position(accountID));
        } catch (e) {
            setError(errorMessage(e));
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setAccountID(event.target.value);
    }

    return (
        <>
            <h3>Account Position</h3>
            <form onSubmit={handleSubmit}>
                <input onChange={handleChange} placeholder="AccountID" />
                <button type="submit">Get Account Position</button>
            </form>
            {error && <div style={{ color: "red" }}>{error}</div>}
            {position && (
                <>
                    <div>Received account...</div>
                    <div>Account ID: {position.account_id} </div>
                    <div>Account Status: {position.account_status} </div>
                    <div>Symbol: {position.symbol} </div>
                    <div>Decimals ID: {position.decimals} </div>
                    <div>Debits Posted: {position.debits_posted} </div>
                    <div>Credits Posted: {position.credits_posted} </div>
                    <div>Debits Pending: {position.debits_pending} </div>
                    <div>Credits Pending: {position.credits_pending} </div>
                    <div>Net Posted: {position.net_posted} </div>
                </>
            )}
        </>
    );
}
