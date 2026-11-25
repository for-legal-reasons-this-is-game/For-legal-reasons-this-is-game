import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";
import { errorMessage } from "@/api";
import { marketQueries } from "@/api/queries";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { formatDecimal } from "@/lib/format";
import type { Market } from "@/types/Market";
import { ChangeText } from "./ChangeText";
import { MarketStatusBadge } from "./MarketStatusBadge";

// Numbers are right-aligned so their digits line up (tabular figures are on everywhere)
const NUMBER = "text-right";

function MarketRow({ market }: { market: Market }) {
    const navigate = useNavigate();
    // each row polls its own ticker; the key is shared with the trade page's header
    const ticker = useQuery(marketQueries.ticker(market.symbol));
    const t = ticker.data;
    const dash = <span className="text-muted-foreground">-</span>;

    return (
        // the whole row opens the market; the symbol is the link for keyboard users
        <TableRow
            className="cursor-pointer"
            onClick={() => navigate(`/markets/${market.symbol}`)}>
            <TableCell>
                <Link
                    to={`/markets/${market.symbol}`}
                    className="font-semibold hover:underline"
                    // the row already navigates; don't do it twice
                    onClick={(event) => event.stopPropagation()}>
                    {market.symbol}
                </Link>
                <div className="text-xs text-muted-foreground">
                    {market.base_asset.name} in {market.quote_asset.name}
                </div>
            </TableCell>
            <TableCell className={NUMBER}>
                {ticker.isPending ? (
                    <Skeleton className="ml-auto h-4 w-20" />
                ) : t?.last_price ? (
                    formatDecimal(t.last_price)
                ) : (
                    dash
                )}
            </TableCell>
            <TableCell className={NUMBER}>
                {t ? <ChangeText change={t.change} percent={t.change_percent} /> : dash}
            </TableCell>
            <TableCell className={NUMBER}>
                {t?.high ? formatDecimal(t.high) : dash}
            </TableCell>
            <TableCell className={NUMBER}>
                {t?.low ? formatDecimal(t.low) : dash}
            </TableCell>
            <TableCell className={NUMBER}>
                {t ? (
                    <>
                        {formatDecimal(t.base_volume)}{" "}
                        <span className="text-muted-foreground">
                            {market.base_asset.symbol}
                        </span>
                    </>
                ) : (
                    dash
                )}
            </TableCell>
            <TableCell>
                <MarketStatusBadge status={market.status} />
            </TableCell>
        </TableRow>
    );
}

export function MarketsTable() {
    const markets = useQuery(marketQueries.list());

    if (markets.isError) {
        return (
            <p role="alert" className="text-destructive">
                Couldn't load the markets: {errorMessage(markets.error)}
            </p>
        );
    }

    if (markets.isSuccess && markets.data.length === 0) {
        return (
            <EmptyState>
                <p>There are no markets yet. Admins create them from two ledgers.</p>
                <Button asChild className="mt-4">
                    <Link to="/admin/markets">Open market setup</Link>
                </Button>
            </EmptyState>
        );
    }

    return (
        <div className="rounded-2xl border bg-card">
            <Table className="font-stretch-semi-condensed">
                <TableHeader>
                    <TableRow>
                        <TableHead>Market</TableHead>
                        <TableHead className={NUMBER}>Last price</TableHead>
                        <TableHead className={NUMBER}>24h change</TableHead>
                        <TableHead className={NUMBER}>24h high</TableHead>
                        <TableHead className={NUMBER}>24h low</TableHead>
                        <TableHead className={NUMBER}>24h volume</TableHead>
                        <TableHead>Status</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {markets.isPending
                        ? // skeleton rows while the list loads
                          [0, 1, 2].map((row) => (
                              <TableRow key={row}>
                                  {[0, 1, 2, 3, 4, 5, 6].map((cell) => (
                                      <TableCell key={cell}>
                                          <Skeleton className="h-4 w-full" />
                                      </TableCell>
                                  ))}
                              </TableRow>
                          ))
                        : markets.data.map((market) => (
                              <MarketRow key={market.market_id} market={market} />
                          ))}
                </TableBody>
            </Table>
        </div>
    );
}
