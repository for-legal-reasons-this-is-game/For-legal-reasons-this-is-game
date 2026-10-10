import { useEffect, useState } from "react";
import { type Ledger } from "./types/Ledger";
import { api, errorMessage } from "./api";

function Ledger({ ledger }: { ledger: Ledger }) {
    return (
        <div>
            <span>{ledger.name}</span>
            <br />
            <span>Id: {ledger.ledger_id} </span>
            <br />
            <span>{ledger.symbol} </span>
            <br />
            <span>Decimal: {ledger.decimals} </span>
            <br />
            <span>Enabled: {ledger.enabled ? "true" : "false"} </span>
            <br />
        </div>
    );
}

export function Ledgers() {
    const [ledgers, setLedgers] = useState<Ledger[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    const [reloadKey, setReloadKey] = useState(0);

    useEffect(() => {
        let ignore = false;
        api.ledgers
            .list()
            .then((ledgers) => {
                if (ignore) return;
                setLedgers(ledgers);
                setError(null);
            })
            .catch((e) => !ignore && setError(errorMessage(e)))
            .finally(() => !ignore && setLoading(false));
        return () => {
            ignore = true;
        };
    }, [reloadKey]);

    function refresh() {
        setLoading(true);
        setReloadKey((k) => k + 1);
    }

    return (
        <div>
            <h3>List of Ledgers</h3>
            <button onClick={refresh}>Refresh</button>
            {error && <div style={{ color: "red" }}>{error}</div>}
            {loading && <div>Loading...</div>}
            {ledgers.map((ledger) => (
                <div key={ledger.ledger_id}>
                    <Ledger ledger={ledger} />
                    <br />
                </div>
            ))}
        </div>
    );
}
