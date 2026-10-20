import { type Ledger } from "../types/Ledger";
import { useFetch, api } from "../api";

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
    const { data, error, loading, refresh } = useFetch(() => api.ledgers.list());

    return (
        <div>
            <h3>List of Ledgers</h3>
            <button onClick={refresh}>Refresh</button>
            {error && <div style={{ color: "red" }}> {error} </div>}
            {loading && <div>Loading...</div>}
            {data?.map((ledger) => (
                <div key={ledger.ledger_id}>
                    <Ledger ledger={ledger} />
                    <br />
                </div>
            ))}
        </div>
    );
}
