import { useState, type FormEvent, type ChangeEvent } from "react";
import { errorMessage, api, ApiError } from "../api";

export function CloseAccount() {
    const [accountID, setAccountID] = useState<string>("");
    const [error, setError] = useState<string | null>(null);
    const [success, setSuccess] = useState<string | null>(null);
    const [loading, setLoading] = useState<boolean>(false);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const id = accountID.trim();
        if (!id) {
            setError("AccountID field can't be empty");
            return;
        }
        setError(null);
        setSuccess(null);
        try {
            setLoading(true);
            await api.accounts.close(id);
            setSuccess(`Closed Account with ID (${accountID})`);
        } catch (e) {
            if (e instanceof ApiError) {
                if (e.status === 404) setError("No Account with this ID");
                else if (e.status === 401) setError("Only User Accounts can be closed");
                else setError(errorMessage(e));
            } else setError(errorMessage(e));
        } finally {
            setLoading(false);
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
                <button disabled={loading} type="submit">
                    Close Account
                </button>
            </form>
            {error && <div style={{ color: "red" }}>{error}</div>}
            {success && <div style={{ color: "green" }}>{success}</div>}
        </div>
    );
}
