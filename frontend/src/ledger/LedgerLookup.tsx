import { type FormEvent, type ChangeEvent, useState } from "react";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";

export function LedgerLookup() {
    const [symbol, setSymbol] = useState("");
    const [submittedSymbol, setSubmittedSymbol] = useState("");
    const [validationError, setValidationError] = useState<string | null>(null);

    const { isError, isFetching, isSuccess, data, error } = useQuery({
        queryKey: ["ledgers", submittedSymbol],
        queryFn: () => api.ledgers.get(submittedSymbol),
        enabled: !!submittedSymbol,
    });

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (!symbol.trim()) {
            setValidationError("Symbol field can't be empty");
            return;
        }
        setSubmittedSymbol(symbol.trim());
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setValidationError(null);
        setSymbol(event.target.value);
    }

    return (
        <>
            <h3>Ledger</h3>
            <form onSubmit={handleSubmit}>
                <input value={symbol} onChange={handleChange} placeholder="Symbol" />
                <button disabled={isFetching} type="submit">
                    Get Ledger
                </button>
            </form>
            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, {
                        404: "A ledger with this symbol doesn't exist",
                    })}
                </div>
            )}
            {isFetching && <div>Loading...</div>}
            {data && isSuccess && (
                <>
                    <div>Name: {data.name} </div>
                    <div>Ledger ID: {data.ledger_id} </div>
                    <div>Symbol: {data.symbol} </div>
                    <div>Decimals: {data.decimals} </div>
                    <div>Status: {data.enabled ? "Enabled" : "Disabled"} </div>
                </>
            )}
        </>
    );
}
