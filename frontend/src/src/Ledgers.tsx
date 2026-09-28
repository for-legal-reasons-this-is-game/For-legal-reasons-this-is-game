import { useEffect, useState } from "react";
import { type Ledger } from "./types/Ledger";

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

    useEffect(() => {
        async function fetchLedgers() {
            try {
                const res = await fetch("http://localhost:8000/api/v1/ledgers");
                const resJson = await res.json();
                setLedgers(resJson);
                console.log(resJson);
            } catch (error) {
                console.log(`error is ${error}`);
            }
        }
        fetchLedgers();
    }, []);

    return (
        <div>
            <h3>List of Ledgers</h3>
            {ledgers.map((ledger) => (
                <div key={ledger.ledger_id}>
                    <Ledger ledger={ledger} />
                    <br />
                </div>
            ))}
        </div>
    );
}
