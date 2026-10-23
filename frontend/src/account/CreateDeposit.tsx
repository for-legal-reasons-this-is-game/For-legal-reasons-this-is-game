import { useMutation, useQueryClient } from "@tanstack/react-query";
import { type DepositPayload } from "../types/DepositPayload";
import { api, errorMessage } from "../api";
import { useState, type ChangeEvent, type FormEvent } from "react";
import { uuidv7 } from "uuidv7";

export function CreateDeposit({ accountID }: { accountID: string }) {
    // TODO: put this component in AccountLookup ??
    const [formData, setFormData] = useState<DepositPayload>({
        idempotency_key: "",
        amount: "",
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    const queryClient = useQueryClient();
    const { isPending, isError, isSuccess, variables, error, mutate } = useMutation({
        mutationFn: (payload: DepositPayload) => api.accounts.deposit(accountID, payload),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ["position"] }),
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        setFormData((prev) => ({
            ...prev,
            idempotency_key: uuidv7(),
        }));
        if (!formData.idempotency_key.trim() || !formData.amount.trim()) {
            setValidationError("Amount field can't be empty");
            return;
        }
        mutate({
            ...formData,
            idempotency_key: formData.idempotency_key.trim(),
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
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
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
