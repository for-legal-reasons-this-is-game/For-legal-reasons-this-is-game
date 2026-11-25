import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import type { Market } from "@/types/Market";

// the select's items can't have "" as a value, so "all" stands for no filter (null)
const ALL = "all";

// Market filter for the order lists, next to StatusFilterSelect.
export function MarketFilterSelect({
    market,
    markets,
    onChange,
}: {
    market: string | null;
    markets: Market[];
    onChange: (market: string | null) => void;
}) {
    return (
        <Select
            value={market ?? ALL}
            onValueChange={(value) => onChange(value === ALL ? null : value)}>
            <SelectTrigger size="sm" aria-label="Market">
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectItem value={ALL}>All markets</SelectItem>
                {markets.map((m) => (
                    <SelectItem key={m.market_id} value={m.symbol}>
                        {m.symbol}
                    </SelectItem>
                ))}
                {/* a market in the address that the list doesn't have still shows its
                    name instead of an empty select */}
                {market && !markets.some((m) => m.symbol === market) && (
                    <SelectItem value={market}>{market}</SelectItem>
                )}
            </SelectContent>
        </Select>
    );
}
