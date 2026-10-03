import { useState, type FormEvent, type ChangeEvent } from "react";
import { type Ledger } from "./types/Ledger";

export function Ledger() {
    const [symbol, setSymbol] = useState("");
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
            const res = await fetch(`http://localhost:8000/api/v1/ledgers/${symbol}`);
            const ledger: Ledger = await res.json();
            setRes(ledger);
            console.log(
                `Received Ledger:\n name:${ledger.name}\n id:${ledger.ledger_id}`,
            );
        } catch (error) {
            console.log(`ERROR: ${error}`);
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setSymbol(event.target.value);
    }

    return (
        <>
            <h3>Ledger</h3>
            <form onSubmit={handleSubmit}>
                <input onChange={handleChange} />
                <button type="submit">Get Ledger</button>
            </form>
            {res.name && (
                <>
                    <div>Received Ledger...</div>
                    <div>Name: {res.name} </div>
                    <div>Ledger ID: {res.ledger_id} </div>
                    <div>Symbol: {res.symbol} </div>
                    <div>Decimals: {res.decimals} </div>
                    <div>Status: {res.enabled ? "Enabled" : "Disabled"} </div>
                </>
            )}
        </>
    );
}
