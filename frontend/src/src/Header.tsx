import { type Market } from "./data";

export function Header({ coin, price, change }: Market) {
    return (
        <header>
            <p>{coin}</p>
            <p>{price}</p>
            <p>{change.toFixed(2)}%</p>
        </header>
    );
}
