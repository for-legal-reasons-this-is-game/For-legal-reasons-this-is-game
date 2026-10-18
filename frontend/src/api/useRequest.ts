import { useState } from "react";
import { ApiError, errorMessage } from "../api";

export function useRequest<T>(request: () => Promise<T> | void) {
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState<boolean>(false);

    async function run(): Promise<boolean> {
        setError(null);
        try {
            setLoading(true);
            await request();
        } catch (e) {
            if (e instanceof ApiError) {
                if (e.status === 404) setError("No Account with this ID");
                else if (e.status === 401) setError("Only User Accounts can be closed");
                else setError(errorMessage(e));
            } else setError(errorMessage(e));
            return true;
        } finally {
            setLoading(false);
        }
        return true;
    }
    return {
        run,
        error,
        loading,
    };
}
