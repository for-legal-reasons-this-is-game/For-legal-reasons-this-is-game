import { type FormEvent, type ChangeEvent, useState } from "react";
import { type Account } from "./types/Account";
import { type AccountPayload } from "./types/AccountPayload";
import { api, errorMessage } from "./api";

type CreateAccountProps = {
    userID: string | undefined;
};

export function CreateAccount({ userID }: CreateAccountProps) {
    const [formData, setFormData] = useState<AccountPayload>({
        name: "",
        ledger_symbol: "",
        code_type: "Cash",
    });
    const [account, setAccount] = useState<Account | null>(null);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setError(null);
        if (!formData.name || !formData.ledger_symbol || !formData.code_type || !userID) {
            setError(
                errorMessage(
                    "Empty field or no userID set. click ListUsers to set userID as last user in the list",
                ),
            );
            return;
        }
        try {
            setAccount(await api.users.createAccount(userID, formData));
        } catch (e) {
            setError(errorMessage(e));
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        const { name, value } = event.target;
        setFormData((prev) => ({
            ...prev,
            [name]: value,
        }));
    }

    return (
        <div>
            <h3>Create Account</h3>
            <form onSubmit={handleSubmit}>
                <input
                    name="name"
                    value={formData.name}
                    onChange={handleChange}
                    placeholder="Account Name"
                />
                <input
                    name="ledger_symbol"
                    value={formData.ledger_symbol}
                    onChange={handleChange}
                    placeholder="ledger_symbol"
                />
                <select>
                    <option value={formData.code_type}>Cash</option>
                    <option value={formData.code_type}>Crypto</option>
                </select>
                {/* <input type="custom-select" value={formData.code_type}></input> */}
                {/* <input
                    name="code_type"
                    value={formData.code_type}
                    onChange={handleChange}
                    placeholder="code_type"
                /> */}
                <button type="submit">Create Account</button>
            </form>

            {error && <div style={{ color: "red" }}>{error}</div>}
            {account && (
                <div>
                    <div>Created Account: </div>
                    <div>Acc ID: {account.account_id}</div>
                    <div>Acc Name: {account.account_name}</div>
                    <div>Ledger ID: {account.account_ledger_id}</div>
                    <div>Code Type: {account.account_code_type}</div>
                    <div>Status: {account.account_status}</div>
                    <div>User ID: {account.account_user_id}</div>
                </div>
            )}
        </div>
    );
}
