import { type Market } from "./data";
import { Header } from "./Header";

type MarketListProps = {
    markets: Market[];
};

export function MarketList({ markets }: MarketListProps) {
    return markets.map((market) => (
        <div key={market.coin}>
            <Header {...market} />
            <br />
        </div>
    ));
}
