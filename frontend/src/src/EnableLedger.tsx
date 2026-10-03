import { type FormEvent, type ChangeEvent, useState } from "react";
import { type Ledger } from "./types/Ledger";

export function EnableLedger() {
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

        const action = (event.nativeEvent as SubmitEvent)
            .submitter as HTMLButtonElement | null;

        try {
            const res = await fetch(`http://localhost:8000/api/v1/ledgers/${symbol}`, {
                method: "PATCH",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    enabled: action?.name === "enable" ? true : false,
                }),
            });
            const ledger: Ledger = await res.json();
            setRes(ledger);
            console.log(
                `${ledger.enabled ? "Enabled " : "Disabled "} Ledger:\n name:${ledger.name}\n id:${ledger.ledger_id}`,
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
            <h3>Enable/Disable Ledger</h3>
            <form onSubmit={handleSubmit}>
                <input name="symbol" onChange={handleChange} value={symbol} />
                <button name="enable" type="submit">
                    Enable Ledger
                </button>
                <button name="disable" type="submit">
                    Disable Ledger
                </button>
            </form>
            {res.name && (
                <>
                    <div>{res.enabled ? "Enabled" : "Disabled"} Ledger...</div>
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
