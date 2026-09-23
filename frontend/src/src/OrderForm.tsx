import { useEffect, useState, type FormEvent } from "react";

type OrderFormProps = {
    coin: string;
    onBuy: (coin: string, amount: number) => void;
    onSell: (coin: string, amount: number) => void;
};

function parseAmount(amount: string): number | null {
    const value = Number(amount);
    if (!Number.isFinite(value) || value <= 0) {
        return null;
    }
    return value;
}

export function OrderForm({ coin, onBuy, onSell }: OrderFormProps) {
    const [amount, setAmount] = useState("");

    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            console.log(`{}`, event.key);
        }
        console.log("adding event listener");
        window.addEventListener("keydown", handleKeyDown);
        return () => {
            console.log("removing event listener");
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, []);

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const value = parseAmount(amount);
        if (value === null) {
            console.log("Not a valid value!");
            return;
        }
        const submitter = (event.nativeEvent as SubmitEvent)
            .submitter as HTMLButtonElement | null;
        if (submitter?.value === "buy") onBuy(coin, value);
        else if (submitter?.value === "sell") onSell(coin, value);
    }

    return (
        <form onSubmit={handleSubmit}>
            <input value={amount} onChange={(event) => setAmount(event.target.value)} />
            <button type="submit" value="buy">
                Buy
            </button>
            <button type="submit" value="sell">
                Sell
            </button>
            <p>You entered: {amount}</p>
        </form>
    );
}
