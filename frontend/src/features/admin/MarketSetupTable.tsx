import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";
import { errorMessage } from "@/api";
import { marketQueries } from "@/api/queries";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { MarketStatusBadge } from "@/features/market/MarketStatusBadge";
import { formatBps, formatTrimmed } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Market } from "@/types/Market";

const NUMBER = "text-right";
// narrower screens keep the market, its status and the fees
const MINIMUMS = "hidden md:table-cell";
const STEPS = "hidden xl:table-cell";

// "0.0001" and a muted "BTC"
function Rule({ value, asset }: { value: string; asset: string }) {
    return (
        <>
            {formatTrimmed(value)} <span className="text-muted-foreground">{asset}</span>
        </>
    );
}

function MarketSetupRow({ market }: { market: Market }) {
    const navigate = useNavigate();
    const base = market.base_asset.symbol;
    const quote = market.quote_asset.symbol;

    return (
        // the whole row opens the market; the symbol is the link for keyboard users
        <TableRow
            className="cursor-pointer"
            onClick={() => navigate(`/markets/${market.symbol}`)}>
            <TableCell>
                <Link
                    to={`/markets/${market.symbol}`}
                    className="font-semibold hover:underline"
                    onClick={(event) => event.stopPropagation()}>
                    {market.symbol}
                </Link>
                {/* table cells don't wrap; this line may, so phones keep more columns */}
                <div className="min-w-36 text-xs whitespace-normal text-muted-foreground">
                    {market.base_asset.name} in {market.quote_asset.name}, prices with{" "}
                    {market.price_decimals} decimals
                </div>
            </TableCell>
            <TableCell>
                <MarketStatusBadge status={market.status} />
                {/* a new market is halted until the engine confirms it */}
                {market.status === "halted" && (
                    <div className="mt-1 text-xs text-muted-foreground">
                        Waiting for the trading engine
                    </div>
                )}
            </TableCell>
            <TableCell className={cn(NUMBER, MINIMUMS)}>
                <Rule value={market.min_base_quantity} asset={base} />
            </TableCell>
            <TableCell className={cn(NUMBER, MINIMUMS)}>
                <Rule value={market.min_quote_quantity} asset={quote} />
            </TableCell>
            <TableCell className={cn(NUMBER, STEPS)}>
                <Rule value={market.quantity_step} asset={base} />
            </TableCell>
            <TableCell className={cn(NUMBER, STEPS)}>
                <Rule value={market.price_step} asset={quote} />
            </TableCell>
            <TableCell className={NUMBER}>
                {formatBps(market.maker_fee_bps)}{" "}
                <span className="text-muted-foreground">/</span>{" "}
                {formatBps(market.taker_fee_bps)}
            </TableCell>
        </TableRow>
    );
}

// Every market with the rules its orders follow. Polled while a market waits for the
// trading engine, so it turns to "Trading" by itself.
export function MarketSetupTable() {
    const markets = useQuery({
        ...marketQueries.list(),
        refetchInterval: (query) =>
            query.state.data?.some((market) => market.status === "halted")
                ? 5_000
                : false,
    });
    const columns = ["", "", MINIMUMS, MINIMUMS, STEPS, STEPS, NUMBER];

    if (markets.isError) {
        return (
            <p role="alert" className="p-2 text-destructive">
                Couldn't load the markets: {errorMessage(markets.error)}
            </p>
        );
    }
    if (markets.isSuccess && markets.data.length === 0) {
        return (
            <p className="p-2 text-muted-foreground">
                No markets yet. Create one from two ledgers, like BTC priced in USD.
            </p>
        );
    }

    return (
        <Table className="font-stretch-semi-condensed">
            <TableHeader>
                <TableRow>
                    <TableHead>Market</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className={cn(NUMBER, MINIMUMS)}>
                        Smallest amount
                    </TableHead>
                    <TableHead className={cn(NUMBER, MINIMUMS)}>Smallest value</TableHead>
                    <TableHead className={cn(NUMBER, STEPS)}>Amount step</TableHead>
                    <TableHead className={cn(NUMBER, STEPS)}>Price step</TableHead>
                    <TableHead className={NUMBER}>Fees (maker / taker)</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {markets.isPending
                    ? [0, 1].map((row) => (
                          <TableRow key={row}>
                              {columns.map((className, cell) => (
                                  <TableCell key={cell} className={className}>
                                      <Skeleton className="h-4 w-full" />
                                  </TableCell>
                              ))}
                          </TableRow>
                      ))
                    : markets.data.map((market) => (
                          <MarketSetupRow key={market.market_id} market={market} />
                      ))}
            </TableBody>
        </Table>
    );
}
