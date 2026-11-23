import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { ChevronDownIcon } from "lucide-react";
import { marketQueries } from "@/api/queries";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Skeleton } from "@/components/ui/skeleton";
import { formatDecimal } from "@/lib/format";
import type { Market } from "@/types/Market";
import { ChangeText } from "./ChangeText";
import { FlashingPrice } from "./FlashingPrice";
import { MarketStatusBadge } from "./MarketStatusBadge";

// The market's name is the page heading, and a button that opens a menu to switch to
// another market (a button may sit inside a heading, not the other way round).
function MarketSwitcher({ market }: { market: Market }) {
    const navigate = useNavigate();
    const markets = useQuery(marketQueries.list());

    return (
        <DropdownMenu>
            <h1 className="text-2xl font-bold font-stretch-expanded">
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="ghost"
                        className="-ml-2 h-auto px-2 py-1 text-2xl font-bold"
                        aria-label={`${market.symbol}, switch market`}>
                        {market.symbol}
                        <ChevronDownIcon className="text-muted-foreground" />
                    </Button>
                </DropdownMenuTrigger>
            </h1>
            <DropdownMenuContent align="start" className="w-56">
                <DropdownMenuRadioGroup
                    value={market.symbol}
                    onValueChange={(symbol) => navigate(`/markets/${symbol}`)}>
                    {markets.data?.map((m) => (
                        <DropdownMenuRadioItem key={m.market_id} value={m.symbol}>
                            {m.symbol}
                        </DropdownMenuRadioItem>
                    ))}
                </DropdownMenuRadioGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

function Stat({ label, children }: { label: string; children: ReactNode }) {
    return (
        <div>
            <dt className="text-xs text-muted-foreground">{label}</dt>
            <dd className="font-medium">{children}</dd>
        </div>
    );
}

// The top of the trade page: the market, its last price as the one bold element, and
// the 24-hour figures in a quiet row underneath.
export function PriceHeader({ market }: { market: Market }) {
    const ticker = useQuery(marketQueries.ticker(market.symbol));
    const t = ticker.data;
    const base = market.base_asset.symbol;
    const quote = market.quote_asset.symbol;

    return (
        <header className="flex flex-col gap-3">
            <div className="flex flex-wrap items-baseline gap-x-6 gap-y-1">
                <MarketSwitcher market={market} />
                {ticker.isPending ? (
                    <Skeleton className="h-12 w-64" />
                ) : t?.last_price ? (
                    <FlashingPrice
                        raw={t.last_price}
                        text={formatDecimal(t.last_price)}
                        className="text-5xl leading-none font-extrabold font-stretch-expanded"
                    />
                ) : (
                    <span className="text-muted-foreground">No trades yet</span>
                )}
                {/* a market that never traded already says so in place of the price */}
                {t?.last_price && (
                    <ChangeText
                        change={t.change}
                        percent={t.change_percent}
                        className="text-lg"
                    />
                )}
                <MarketStatusBadge status={market.status} />
            </div>
            {t && (
                <dl className="flex flex-wrap gap-x-8 gap-y-2 text-sm">
                    <Stat label="24h high">
                        {t.high ? `${formatDecimal(t.high)} ${quote}` : "-"}
                    </Stat>
                    <Stat label="24h low">
                        {t.low ? `${formatDecimal(t.low)} ${quote}` : "-"}
                    </Stat>
                    <Stat label="24h volume">
                        {formatDecimal(t.base_volume)} {base}
                    </Stat>
                    <Stat label="24h value">
                        {formatDecimal(t.quote_volume)} {quote}
                    </Stat>
                    <Stat label="24h trades">{t.trade_count}</Stat>
                </dl>
            )}
        </header>
    );
}
