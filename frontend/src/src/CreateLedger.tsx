import { useState, type FormEvent, type ChangeEvent } from "react";
import { type LedgerPayload } from "./types/LedgerPayload";
import { type Ledger } from "./types/Ledger";
import { api, errorMessage } from "./api";

export function CreateLedger() {
    const [formData, setFormData] = useState<LedgerPayload>({
        symbol: "",
        name: "",
        decimals: 2,
    });
    // null = nothing created yet; no need for a fake "empty" Ledger
    const [ledger, setLedger] = useState<Ledger | null>(null);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setError(null);
        if (!formData.symbol || !formData.name) {
            setError(errorMessage("Can't have empty fields"));
            return;
        }
        try {
            setLedger(await api.ledgers.create(formData));
        } catch (e) {
            setError(errorMessage(e));
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        const { name, value } = event.target;
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
                <button type="submit">Create Ledger</button>
            </form>

            {error && <div style={{ color: "red" }}>{error}</div>}
            {ledger && (
                <div>
                    <div>Created Ledger: </div>
                    <div>Name: {ledger.name}</div>
                    <div>Symbol: {ledger.symbol}</div>
                    <div>Decimals: {ledger.decimals}</div>
                    <div>Ledger ID: {ledger.ledger_id}</div>
                    <div>Enabled: {ledger.enabled ? "true" : "false"}</div>
                </div>
            )}
        </>
    );
}
