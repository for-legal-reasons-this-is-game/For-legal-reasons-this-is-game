import { useState, type FormEvent, type ChangeEvent } from "react";
// import { errorMessage, api, ApiError } from "../api";
import { useRequest } from "../api";

export function CloseAccount() {
    const [accountID, setAccountID] = useState<string>("");
    const close = useRequest(accountID.trim());

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (!accountID.trim()) {
            // setError("AccountID field can't be empty");
            return;
        }
        await close.run();
        // await close;
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
            {close.error && <div style={{ color: "red" }}>{close.error}</div>}
            {/* {close.success && <div style={{ color: "green" }}>{success}</div>} */}
        </div>
    );
}
