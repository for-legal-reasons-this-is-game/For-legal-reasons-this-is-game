import { useState, type FormEvent, type ChangeEvent } from "react";
import { type LedgerPayload } from "../types/LedgerPayload";
import { api, errorMessage } from "../api";
import { useMutation, useQueryClient } from "@tanstack/react-query";

export function CreateLedger() {
    const [formData, setFormData] = useState<LedgerPayload>({
        symbol: "",
        name: "",
        decimals: 2,
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    const queryClient = useQueryClient();
    const { isPending, isError, isSuccess, data, error, mutate } = useMutation({
        mutationFn: (payload: LedgerPayload) => api.ledgers.create(payload),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ["ledgers"] }),
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (!formData.symbol.trim() || !formData.name.trim() || formData.decimals < 0) {
            setValidationError("Can't have empty fields");
            return;
        }
        mutate({
            ...formData,
            symbol: formData.symbol.trim(),
            name: formData.name.trim(),
        });
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        const { name, value } = event.target;
        setValidationError(null);
        setFormData((prev) => ({
            ...prev,
            [name]: name === "decimals" ? Number(value) : value,
        }));
    }

    return (
        <>
            <h3>Create Ledger</h3>
            <form onSubmit={handleSubmit}>
                <input
                    name="name"
                    value={formData.name}
                    onChange={handleChange}
                    placeholder="Name"
                />
                <input
                    name="symbol"
                    value={formData.symbol}
                    onChange={handleChange}
                    placeholder="Symbol"
                />
                <input
                    name="decimals"
                    type="number"
                    value={formData.decimals}
                    onChange={handleChange}
                    placeholder="Decimals"
                />
                <button disabled={isPending} type="submit">
                    Create Ledger
                </button>
            </form>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isSuccess && (
                <div>
                    <div>Created Ledger: </div>
                    <div>Name: {data.name}</div>
                    <div>Symbol: {data.symbol}</div>
                    <div>Decimals: {data.decimals}</div>
                    <div>Ledger ID: {data.ledger_id}</div>
                    <div>Enabled: {data.enabled ? "true" : "false"}</div>
                </div>
            )}
        </>
    );
}
