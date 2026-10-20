import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, useRequest } from "../api";

export function CloseAccount() {
    const [accountID, setAccountID] = useState<string>("");
    const [error, setError] = useState<string | null>(null);
    const [success, setSuccess] = useState<string | null>(null);
    const close = useRequest<void>(() => api.accounts.close(accountID.trim()));

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        setError(null);
        setSuccess(null);
        if (!accountID.trim()) {
            setError("AccountID field can't be empty");
            return;
        }
        const res = await close.run();
        if (res.ok) {
            setSuccess(`Closed Account (${accountID})`);
        } else {
            setError(res.error);
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setAccountID(event.target.value);
    }

    return (
        <div>
            <h3>Close Account</h3>
            <form onSubmit={handleSubmit}>
                <input
                    onChange={handleChange}
                    value={accountID}
                    placeholder="AccountID"
                />
                <button disabled={close.loading} type="submit">
                    Close Account
                </button>
            </form>
            {error && <div style={{ color: "red" }}>{error}</div>}
            {success && <div style={{ color: "green" }}>{success}</div>}
        </div>
    );
}
