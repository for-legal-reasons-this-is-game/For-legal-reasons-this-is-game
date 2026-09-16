import { markets } from "./data";
import { MarketList } from "./MarketList";
import { useState } from "react";

function App() {
    const [totalOrders, setTotalOrders] = useState(0);
    return (
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
    );
}

export default App;
