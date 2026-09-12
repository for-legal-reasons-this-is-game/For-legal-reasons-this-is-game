import { type Market } from "./data";
import { Header } from "./Header";
import React from "react";

type MarketListProps = {
    markets: Market[];
    onBuy: (coin: string, amount: number) => void;
    onSell: (coin: string, amount: number) => void;
};

export function MarketList({ markets, onBuy, onSell }: MarketListProps) {
    return markets.map((market) => (
        <div key={market.coin}>
            <Header {...market} onBuy={onBuy} onSell={onSell} />
            <br />
        </div>
    ));
}
