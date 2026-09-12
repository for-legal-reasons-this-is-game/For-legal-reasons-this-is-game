import { type Market } from "./data";
import { useState } from "react";
import { type SubmitEvent } from "react";

// `Market &` means all the properties of Market plus...
type HeaderProps = Market & {
    onBuy: (coin: string, amount: number) => void;
    onSell: (coin: string, amount: number) => void;
};

// function handleFormSubmit(event: SubmitEvent) {
//     const value = Number(SubmitEvent. amount);
//     if (!isFinite(value) || value <= 0) {
//         console.log("Not a valid value!");
//         return;
//     }
//     onBuy(coin, value);
// }

export function Header({ coin, price, change, onBuy, onSell }: HeaderProps) {
    const [amount, setAmount] = useState("");
    return (
        <header>
            <p>{coin}</p>
            <p>{price}</p>
            <p>{change.toFixed(2)}%</p>
            <p>{change > 0 ? "UP" : "DOWN"}</p>
            <p>{change > 0 && <span>Market is rising!</span>}</p>
            <div>
                <form>
                    <input
                        value={amount}
                        onChange={(event) => setAmount(event.target.value)}
                    />
                    <button
                        onClick={(event) => {
                            event.preventDefault();
                            const value = Number(amount);
                            if (!isFinite(value) || value <= 0) {
                                console.log("Not a valid value!");
                                return;
                            }
                            onBuy(coin, value);
                        }}>
                        Buy
                    </button>
                    <button
                        onClick={(event) => {
                            event.preventDefault();
                            const value = Number(amount);
                            if (!isFinite(value) || value <= 0) {
                                console.log("Not a valid value!");
                                return;
                            }
                            onSell(coin, value);
                        }}>
                        Buy
                    </button>
                </form>
            </div>
            <p>You entered: {amount}</p>
        </header>
    );
}
