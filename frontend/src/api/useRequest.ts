import { useState } from "react";
import { ApiError, api, errorMessage } from "../api";

export function useRequest(id: string) {
    const [error, setError] = useState<string | null>(null);
    // const [success, setSuccess] = useState<string | null>(null);
    const [loading, setLoading] = useState<boolean>(false);

    async function run() {
        setError(null);
        // setSuccess(null);
        try {
            setLoading(true);
            // await func();
            await api.accounts.close(id);
            // setSuccess(`Closed Account with ID (${accountID})`);
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
    return {
        run,
        error,
        // success,
        loading,
    };
}
