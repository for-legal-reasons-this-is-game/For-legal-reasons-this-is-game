import { useState } from "react";
import { errorMessage } from "./client";

type Result<T> = { ok: true; data: T | null } | { ok: false; error: string };

export function useRequest<T>(request: () => Promise<T>) {
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState<boolean>(false);
    const [data, setData] = useState<T>();

    async function run(): Promise<Result<T>> {
        setError(null);
        try {
            setLoading(true);
            const res = await request();
            setData(res);
            return { ok: true, data: res };
        } catch (e) {
            const message = errorMessage(e);
            setError(message);
            return { ok: false, error: message };
        } finally {
            setLoading(false);
        }
    }
    return {
        run,
        error,
        loading,
        data,
    };
}
