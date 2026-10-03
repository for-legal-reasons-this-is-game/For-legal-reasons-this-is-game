import { type FormEvent, type ChangeEvent, useState } from "react";
import { type Position } from "./types/Position";

export function Position() {
    const [accountID, setAccountID] = useState("");
    const [res, setRes] = useState<Position>({
        account_id: "",
        account_status: "processing",
        symbol: "",
        decimals: -1,
        debits_posted: "",
        credits_posted: "",
        debits_pending: "",
        credits_pending: "",
        net_posted: "",
    });

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        try {
            const res = await fetch(
                `http://localhost:8000/api/v1/accounts/${accountID}/position`,
            );
            const position: Position = await res.json();
            setRes(position);
            console.log(`Received Position:\n acc_id:${position.account_id}\n`);
        } catch (error) {
            console.log(`ERROR: ${error}`);
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setAccountID(event.target.value);
    }

    return (
        <>
            <h3>Account Position</h3>
            <form onSubmit={handleSubmit}>
                <input onChange={handleChange} placeholder="Account ID" />
                <button type="submit">Get Account Position</button>
            </form>
            {res.account_id && (
                <>
                    <div>Received account...</div>
                    <div>Account ID: {res.account_id} </div>
                    <div>Account Status: {res.account_status} </div>
                    <div>Symbol: {res.symbol} </div>
                    <div>Decimals ID: {res.decimals} </div>
                    <div>Debits Posted: {res.debits_posted} </div>
                    <div>Credits Posted: {res.credits_posted} </div>
                    <div>Debits Pending: {res.debits_pending} </div>
                    <div>Credits Pending: {res.credits_pending} </div>
                    <div>Net Posted: {res.net_posted} </div>
                </>
            )}
        </>
    );
}
