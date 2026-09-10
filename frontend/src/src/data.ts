export type Market = {
    coin: string;
    price: number;
    change: number;
};

export const markets: Market[] = [
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
