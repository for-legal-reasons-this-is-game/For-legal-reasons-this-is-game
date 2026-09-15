import { type Market } from "./data";
import { OrderForm } from "./OrderForm";

// `Market &` means all the properties of Market plus...
type HeaderProps = Market & {
    onBuy: (coin: string, amount: number) => void;
    onSell: (coin: string, amount: number) => void;
};

export function Header({ coin, price, change, onBuy, onSell }: HeaderProps) {
    return (
        <header>
            <p>{coin}</p>
            <p>{price}</p>
            <p>{change.toFixed(2)}%</p>
            <p>
                {change > 0 ? "UP" : "DOWN"}
                {change > 0 && <span>, Market is rising!</span>}
            </p>
            <OrderForm coin={coin} onBuy={onBuy} onSell={onSell} />
        </header>
    );
}
