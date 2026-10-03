import { useState, type FormEvent, type ChangeEvent } from "react";
import { type LedgerPayload } from "./types/LedgerPayload";
import { type Ledger } from "./types/Ledger";

export function CreateLedger() {
    const [formData, setFormData] = useState<LedgerPayload>({
        symbol: "",
        name: "",
        decimals: -1,
    });

    const [res, setRes] = useState<Ledger>({
        ledger_id: -1,
        symbol: "",
        name: "",
        decimals: -1,
        enabled: false,
    });

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        try {
            console.log(`body: ${JSON.stringify(formData)}`);
            const res = await fetch(`http://localhost:8000/api/v1/ledgers`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(formData),
            });

            const ledger = await res.json();
            setRes(ledger);
        } catch (error) {
            console.log(`ERROR: ${error}`);
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
                    value={formData.decimals}
                    onChange={handleChange}
                    placeholder="Decimals"
                />
                <button type="submit">Create Ledger</button>
            </form>

            {res.name && (
                <div>
                    <div>Created Account: </div>
                    <div>Name: {res.name}</div>
                    <div>Symbol: {res.symbol}</div>
                    <div>Decimals: {res.decimals}</div>
                    <div>Ledger ID: {res.ledger_id}</div>
                    <div>Enabled: {res.enabled ? "true" : "false"}</div>
                </div>
            )}
        </>
    );
}
