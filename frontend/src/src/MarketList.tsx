import { type Market } from "./data";
import { Header } from "./Header";
import { OrderForm } from "./OrderForm";

type MarketListProps = {
    markets: Market[];
    onBuy: (coin: string, amount: number) => void;
    onSell: (coin: string, amount: number) => void;
};

export function MarketList({ markets, onBuy, onSell }: MarketListProps) {
    return markets.map((market) => (
        <div key={market.coin}>
            <Header {...market}>
                <OrderForm coin={market.coin} onBuy={onBuy} onSell={onSell} />
            </Header>
            <br />
        </div>
    ));
}
