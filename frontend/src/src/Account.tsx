import { useState, type FormEvent, type ChangeEvent } from "react";
import { type Account } from "./types/Account";

export function Account() {
    const [accountID, setAccountID] = useState("");
    const [res, setRes] = useState<Account>({
        account_id: "",
        account_name: "",
        account_code_type: "Cash",
        account_ledger_id: -1,
        account_status: "active",
        account_user_id: "",
    });

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        try {
            const res = await fetch(`http://localhost:8000/api/v1/accounts/${accountID}`);
            const account: Account = await res.json();
            setRes(account);
            console.log(
                `Received Account:\n name:${account.account_name}\n id:${account.account_id}`,
            );
        } catch (error) {
            console.log(`ERROR: ${error}`);
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setAccountID(event.target.value);
    }

    return (
        <>
            <h3>Account</h3>
            <form onSubmit={handleSubmit}>
                <input onChange={handleChange} />
                <button type="submit">Get Account</button>
            </form>
            {res.account_id && (
                <>
                    <div>Received account...</div>
                    <div>Name: {res.account_name} </div>
                    <div>ID: {res.account_id} </div>
                    <div>Code Type: {res.account_code_type} </div>
                    <div>Ledger ID: {res.account_ledger_id} </div>
                    <div>Status: {res.account_status} </div>
                    <div>User ID: {res.account_user_id} </div>
                </>
            )}
        </>
    );
}
