type Market = {
    coin: string;
    price: number;
    change: number;
};

function Header({ coin, price, change }: Market) {
    return (
        <header>
            <p>{coin}</p>
            <p>{price}</p>
            <p>{change.toFixed(2)}%</p>
        </header>
    );
}

function App() {
    const markets: Market[] = [
        {
            coin: "BTC/USD",
            price: 67000,
            change: 2.5,
        },
        {
            coin: "ETH/USD",
            price: 4500,
            change: -1.2,
        },
        {
            coin: "SOL/USD",
            price: 180,
            change: 4.7,
        },
    ];
    return (
        <>
            <h1>My Trading Platform</h1>
            {markets.map((market) => (
                <div key={market.coin}>
                    <Header {...market} />
                    <br />
                </div>
            ))}
        </>
    );
}

export default App;
