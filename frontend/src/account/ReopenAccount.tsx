import { useState, type FormEvent } from "react";
import { api, errorMessage } from "../api";
import { useMutation, useQueryClient } from "@tanstack/react-query";

export function ReopenAccount({ accountID }: { accountID: string }) {
    // TODO: show either CloseAccount or ReopenAccount depending on whether the account is
    // closed. The API doesn't report that yet (AccountStatus is only active/processing),
    // so AccountLookup shows both.
    const [validationError, setValidationError] = useState<string | null>(null);

    const queryClient = useQueryClient();
    const { isPending, isError, isSuccess, variables, error, mutate } = useMutation({
        mutationFn: (id: string) => api.accounts.reopen(id),
        onSuccess: (_data, id) =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: ["account", id] }),
                queryClient.invalidateQueries({ queryKey: ["position", id] }),
            ]),
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        setValidationError(null);
        if (!accountID.trim()) {
            setValidationError("AccountID field can't be empty");
            return;
        }
        mutate(accountID.trim());
    }

    return (
        <div>
            <form onSubmit={handleSubmit}>
                <button disabled={isPending} type="submit">
                    Reopen Account
                </button>
            </form>
            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isSuccess && (
                <div style={{ color: "green" }}>
                    {`Reopened account ${variables} successfully`}
                </div>
            )}
        </div>
    );
}
