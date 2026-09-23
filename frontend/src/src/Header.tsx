import type { ReactNode } from "react";
import { useState } from "react";
import { type Market } from "./data";

// `Market &` means all the properties of Market plus...
type HeaderProps = Market & {
    children: ReactNode;
};

export function Header({ coin, price, change, children }: HeaderProps) {
    const [isVisible, setIsVisible] = useState(false);

    return (
        <header>
            <p>{coin}</p>
            <p>{price}</p>
            <p>{change.toFixed(2)}%</p>
            <p>
                {change > 0 ? "UP" : "DOWN"}
                {change > 0 && <span>, Market is rising!</span>}
            </p>
            <button onClick={() => setIsVisible(!isVisible)}>
                {(isVisible ? "Hide" : "Show") + " Form"}
            </button>
            {isVisible && children}
        </header>
    );
}
