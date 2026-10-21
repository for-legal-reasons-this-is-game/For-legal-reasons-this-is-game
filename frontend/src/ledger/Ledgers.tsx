import { type Ledger } from "../types/Ledger";
import { api, errorMessage } from "../api";
import { EnableLedger } from "./EnableLedger";
import { useQuery } from "@tanstack/react-query";

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
            <span>
                Enabled: {ledger.enabled ? "yes " : "no "}
                <EnableLedger symbol={ledger.symbol} enabled={ledger.enabled} />
            </span>
            <br />
        </div>
    );
}

export function Ledgers() {
    const { isError, isPending, isFetching, data, error, refetch } = useQuery({
        queryKey: ["ledgers"],
        queryFn: api.ledgers.list,
    });

    return (
        <div>
            <h3>List of Ledgers</h3>
            <button onClick={() => refetch()}>Refresh</button>
            {isError && (
                <div style={{ color: "red" }}> {errorMessage(error.message)} </div>
            )}
            {(isPending || isFetching) && <div>Loading...</div>}
            {data?.map((ledger) => (
                <div key={ledger.ledger_id}>
                    <Ledger ledger={ledger} />
                    <br />
                </div>
            ))}
        </div>
    );
}
