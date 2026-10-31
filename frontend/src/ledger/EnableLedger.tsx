import { api, errorMessage } from "../api";
import { useMutation, useQueryClient } from "@tanstack/react-query";

export function EnableLedger({ symbol, enabled }: { symbol: string; enabled: boolean }) {
    const queryClient = useQueryClient();

    const { isPending, isError, error, mutate } = useMutation({
        mutationFn: ({ symbol, enabled }: { symbol: string; enabled: boolean }) =>
            api.ledgers.setEnabled(symbol, { enabled: !enabled }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ["ledgers"] }),
    });

    return (
        <>
            <button
                disabled={isPending}
                onClick={() => mutate({ symbol, enabled })}
                style={{ color: enabled ? "red" : "green" }}
                type="button">
                {enabled ? "Disable" : "Enable"}
            </button>
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isPending && <div>Loading...</div>}
        </>
    );
}
