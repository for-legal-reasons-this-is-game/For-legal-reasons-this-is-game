import { markets } from "./data";
import { MarketList } from "./MarketList";

function App() {
    return (
        <>
            <h1>My Trading Platform</h1>
            <MarketList markets={markets} />
        </>
    );
}

export default App;
