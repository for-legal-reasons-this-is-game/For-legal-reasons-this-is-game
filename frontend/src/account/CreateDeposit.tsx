import { useMutation, useQueryClient } from "@tanstack/react-query";
import { type DepositPayload } from "../types/DepositPayload";
import { api, errorMessage } from "../api";
import { useState, type ChangeEvent, type FormEvent } from "react";
import { uuidv7 } from "uuidv7";

export function CreateDeposit({ accountID }: { accountID: string }) {
    const [formData, setFormData] = useState<DepositPayload>({
        idempotency_key: "",
        amount: "",
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    const queryClient = useQueryClient();
    const { isPending, isError, isSuccess, variables, error, mutate } = useMutation({
        mutationFn: (payload: DepositPayload) => api.accounts.deposit(accountID, payload),
        // onSuccess's 2nd argument is what was passed to mutate() (the payload here),
        // so the account ID comes from the prop instead
        onSuccess: () =>
            Promise.all([
                queryClient.invalidateQueries({ queryKey: ["position", accountID] }),
                queryClient.invalidateQueries({
                    queryKey: ["transfers", "account", accountID],
                }),
            ]),
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (!formData.amount.trim()) {
            setValidationError("Amount field can't be empty");
            return;
        }
        const idempotency_key = uuidv7();
        mutate({
            ...formData,
            idempotency_key: idempotency_key,
            amount: formData.amount.trim(),
        });
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setValidationError(null);
        const { name, value } = event.target;
        setFormData((prev) => ({
            ...prev,
            [name]: value,
        }));
    }

    return (
        <div>
            <h3>Create Deposit</h3>
            <form onSubmit={handleSubmit}>
                <input
                    name="amount"
                    value={formData.amount}
                    type="number"
                    onChange={handleChange}
                    placeholder="Amount"
                />
                <button disabled={isPending} type="submit">
                    Create Deposit
                </button>
            </form>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, {
                        400: "Invalid deposit: the amount must be positive, with no more decimals than the ledger allows, and the ledger must be enabled",
                        403: "Only user accounts can receive deposits",
                        409: "This account can't receive deposits right now: it's closed or still being created",
                    })}
                </div>
            )}
            {isSuccess && (
                <div>
                    <div style={{ color: "green" }}>
                        {`Created Deposit of amount ${variables.amount}`}
                    </div>
                </div>
            )}
        </div>
    );
}
