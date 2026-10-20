import { useState, useEffect, type DependencyList } from "react";
import { errorMessage } from "./client";

export function useFetch<T>(request: () => Promise<T>, deps: DependencyList = []) {
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    const [data, setData] = useState<T | null>(null);
    const [reloadKey, setReloadKey] = useState(0);

    useEffect(() => {
        let ignore = false;
        request()
            .then((res) => {
                if (ignore) return;
                setData(res);
                setError(null);
            })
            .catch((e) => !ignore && setError(errorMessage(e)))
            .finally(() => !ignore && setLoading(false));
        return () => {
            ignore = true;
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [reloadKey, ...deps]);

    function refresh() {
        setLoading(true);
        setReloadKey((k) => k + 1);
    }

    return {
        data,
        error,
        loading,
        refresh,
    };
}
