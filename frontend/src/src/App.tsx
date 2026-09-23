import { markets } from "./data";
import { MarketList } from "./MarketList";
import { useState } from "react";

type Ledger = {
    ledger_id: number;
    symbol: string;
    name: string;
    decimals: number;
    enabled: boolean;
};

function Backend() {
    const [ledgers, setLedgers] = useState<Ledger[]>([]);

    async function getJson() {
        try {
            const response = await fetch("http://localhost:8000/api/v1/ledgers");
            const hehe = await response.json();
            setLedgers(hehe);
            console.log(hehe);
        } catch (error) {
            console.log(`error is ${error}`);
        }
    }

    return (
        <>
            <button onClick={getJson}>Get Data</button>
            {ledgers.map((ledger) => (
                <div key={ledger.ledger_id}>
                    <span>Name: {ledger.name}</span>
                    <span>Id: {ledger.ledger_id} </span>
                    <span>Symbol: {ledger.symbol} </span>
                    <span>Decimal: {ledger.decimals} </span>
                    <span>Enabled: {ledger.enabled} </span>
                    <br />
                </div>
            ))}
        </>
    );
    // return (
    //     <>
    //         <div>{jsonData ? jsonData[0]["ledger_id"] : ""}</div>
    //     </>
    // );
}

function App() {
    const [totalOrders, setTotalOrders] = useState(0);
    return (
        <>
            <Backend />
            <div className="market_list">
                <h1>My Trading Platform</h1>
                {totalOrders > 0 && <h2>Total Orders: {totalOrders}</h2>}
                <MarketList
                    markets={markets}
                    onBuy={(coin: string, amount: number) => {
                        setTotalOrders((current) => current + 1);
                        console.log(`Buying ${amount} ${coin}`);
                    }}
                    onSell={(coin: string, amount: number) => {
                        setTotalOrders((current) => current + 1);
                        console.log(`Selling ${amount} ${coin}`);
                    }}
                />
            </div>
        </>
    );
}

export default App;
