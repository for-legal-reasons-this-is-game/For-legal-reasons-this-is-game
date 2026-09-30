import { type FormEvent, type ChangeEvent, useState } from "react";
import { type Account } from "./types/Account";

export function CreateAccount() {
    const [formData, setFormData] = useState({
        name: "",
        user_id: "",
        ledger_symbol: "",
        code_type: "",
    });
    const [creationRes, setCreationRes] = useState({
        account_id: "",
        account_name: "",
        account_ledger_id: -1,
        account_code_type: "",
        account_status: "",
        account_user_id: "",
    });

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        try {
            if (formData.user_id == "") {
                alert("User ID can't be empty when creating an account!");
                throw Error("User ID can't be empty when creating an account!");
            }
            const res = await fetch(
                `http://localhost:8000/api/v1/users/${formData.user_id}/accounts`,
                {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify(formData),
                },
            );
            const account: Account = await res.json();
            setCreationRes(account);
            console.log(
                `Created Account:\n
				account_id:${account.account_id}\n
				account_name:${account.account_name}\n,
				account_ledger_id:${account.account_ledger_id}\n
				account_code_type:${account.account_code_type}\n
				account_status:${account.account_status}\n
				account_user_id:${account.account_user_id}\n`,
            );
        } catch (error) {
            console.log(`ERROR: ${error}`);
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
                    name="user_id"
                    value={formData.user_id}
                    onChange={handleChange}
                    placeholder="User ID"
                />
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
                <input
                    name="code_type"
                    value={formData.code_type}
                    onChange={handleChange}
                    placeholder="code_type"
                />
                <button type="submit">Create Account</button>
            </form>

            {creationRes.account_name && (
                <div>
                    <span>Created Account: </span>
                    acc_id: {creationRes.account_ledger_id}
                    acc_name: {creationRes.account_name}
                    acc_ledger_id: {creationRes.account_ledger_id}
                    acc_code_type: {creationRes.account_ledger_id}
                    acc_status: {creationRes.account_ledger_id}
                    account_user_id: {creationRes.account_ledger_id}
                </div>
            )}
        </div>
    );
}
