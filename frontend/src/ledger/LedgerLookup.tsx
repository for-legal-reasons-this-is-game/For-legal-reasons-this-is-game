import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, useRequest } from "../api";

export function LedgerLookup() {
    const [symbol, setSymbol] = useState("");
    // only for form validation; the request's own error lives in the hook
    const [validationError, setValidationError] = useState<string | null>(null);
    const lookup = useRequest(() => api.ledgers.get(symbol.trim()));

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (!symbol.trim()) {
            setValidationError("Symbol field can't be empty");
            return;
        }
        setValidationError(null);
        await lookup.run();
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setSymbol(event.target.value);
    }

    const error = validationError ?? lookup.error;
    const ledger = lookup.data;

    return (
        <>
            <h3>Ledger</h3>
            <form onSubmit={handleSubmit}>
                <input value={symbol} onChange={handleChange} placeholder="Symbol" />
                <button disabled={lookup.loading} type="submit">
                    Get Ledger
                </button>
            </form>
            {error && <div style={{ color: "red" }}>{error}</div>}
            {/* !error: the hook keeps the last ledger when a newer lookup fails */}
            {ledger && !error && (
                <>
                    <div>Name: {ledger.name} </div>
                    <div>Ledger ID: {ledger.ledger_id} </div>
                    <div>Symbol: {ledger.symbol} </div>
                    <div>Decimals: {ledger.decimals} </div>
                    <div>Status: {ledger.enabled ? "Enabled" : "Disabled"} </div>
                </>
            )}
        </>
    );
}
