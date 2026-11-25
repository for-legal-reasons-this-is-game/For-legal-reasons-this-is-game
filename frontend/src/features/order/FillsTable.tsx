import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { errorMessage } from "@/api";
import { orderQueries } from "@/api/queries";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableFooter,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { fromUnits, toUnits } from "@/lib/decimal";
import { formatDecimal, formatWhen } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";
import type { Trade } from "@/types/Trade";
import { isOpen } from "./order-state";

const NUMBER = "text-right";
// The columns a narrow panel leaves out, by the panel's width (container queries, see
// OrdersTable): the value from 24rem (384 px) on, the fee and the role from 28rem (448 px).
// The order's details above already say the filled value.
const VALUE_COLUMN = "hidden @sm:table-cell";
const FEE_COLUMN = "hidden @md:table-cell";
const ROLE_COLUMN = "hidden @md:table-cell";

// What a trade was for this order: which fee it paid, and whether it was the maker (it was
// waiting in the book) or the taker (it traded as it arrived). Fees are in the quote asset.
function sideOf(trade: Trade, order: Order) {
    const bought = trade.buyer_order_id === order.order_id;
    return {
        fee: bought ? trade.buyer_fee : trade.seller_fee,
        role: trade.taker_side === order.side ? "Taker" : "Maker",
    };
}

// Adds up a column of decimal strings exactly; null if one doesn't parse
function sum(values: string[], decimals: number): string | null {
    let total = 0n;
    for (const value of values) {
        const units = toUnits(value, decimals);
        if (units === null) return null;
        total += units;
    }
    return fromUnits(total, decimals);
}

// A big order can fill hundreds of times (the endpoint isn't paged): the table shows the
// latest few until asked for all
const SHOWN_FILLS = 20;

// The trades that filled an order, oldest first, with their totals under a double rule
// (like the foot of a ledger page). Polled while the order can still trade.
export function FillsTable({ order, market }: { order: Order; market: Market }) {
    const trades = useQuery(orderQueries.trades(order.order_id, isOpen(order)));
    const [showAll, setShowAll] = useState(false);
    const base = market.base_asset;
    const quote = market.quote_asset;

    if (trades.isError) {
        return (
            <p role="alert" className="py-2 text-destructive">
                Couldn't load the fills: {errorMessage(trades.error)}
            </p>
        );
    }
    if (trades.data && trades.data.length === 0) {
        return (
            <p className="py-2 text-muted-foreground">
                {order.submission === "rejected"
                    ? "A rejected order never trades."
                    : isOpen(order)
                      ? "No fills yet. A fill shows here each time the order trades."
                      : "None. The order ended without trading."}
            </p>
        );
    }

    const fills = trades.data?.map((trade) => ({ trade, ...sideOf(trade, order) }));
    const hidden = fills && !showAll ? Math.max(fills.length - SHOWN_FILLS, 0) : 0;
    // the totals always cover every fill, shown or not
    const totals = fills && {
        amount: sum(
            fills.map(({ trade }) => trade.base_quantity),
            base.decimals,
        ),
        value: sum(
            fills.map(({ trade }) => trade.quote_quantity),
            quote.decimals,
        ),
        fee: sum(
            fills.map(({ fee }) => fee),
            quote.decimals,
        ),
    };

    return (
        <div className="@container">
            {/* the hidden fills are the older ones, above the shown rows */}
            {fills && hidden > 0 && (
                <p className="py-1 text-sm text-muted-foreground">
                    The latest {SHOWN_FILLS} of {fills.length} fills.{" "}
                    <Button
                        variant="link"
                        className="h-auto p-0"
                        onClick={() => setShowAll(true)}>
                        Show all
                    </Button>
                </p>
            )}
            <Table className="font-stretch-semi-condensed">
                <TableHeader>
                    <TableRow>
                        <TableHead>Time</TableHead>
                        <TableHead className={NUMBER}>Price ({quote.symbol})</TableHead>
                        <TableHead className={NUMBER}>Amount ({base.symbol})</TableHead>
                        <TableHead className={cn(NUMBER, VALUE_COLUMN)}>
                            Value ({quote.symbol})
                        </TableHead>
                        <TableHead className={cn(NUMBER, FEE_COLUMN)}>
                            Fee ({quote.symbol})
                        </TableHead>
                        <TableHead className={ROLE_COLUMN}>Role</TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {fills
                        ? fills.slice(hidden).map(({ trade, fee, role }) => (
                              <TableRow key={trade.trade_id}>
                                  <TableCell>{formatWhen(trade.engine_time)}</TableCell>
                                  <TableCell className={NUMBER}>
                                      {formatDecimal(trade.price)}
                                  </TableCell>
                                  <TableCell className={NUMBER}>
                                      {formatDecimal(trade.base_quantity)}
                                  </TableCell>
                                  <TableCell className={cn(NUMBER, VALUE_COLUMN)}>
                                      {formatDecimal(trade.quote_quantity)}
                                  </TableCell>
                                  <TableCell className={cn(NUMBER, FEE_COLUMN)}>
                                      {formatDecimal(fee)}
                                  </TableCell>
                                  <TableCell className={ROLE_COLUMN}>{role}</TableCell>
                              </TableRow>
                          ))
                        : [0, 1].map((row) => (
                              <TableRow key={row}>
                                  {[
                                      "",
                                      NUMBER,
                                      NUMBER,
                                      VALUE_COLUMN,
                                      FEE_COLUMN,
                                      ROLE_COLUMN,
                                  ].map((className, cell) => (
                                      <TableCell key={cell} className={className}>
                                          <Skeleton className="h-4 w-full" />
                                      </TableCell>
                                  ))}
                              </TableRow>
                          ))}
                </TableBody>
                {totals && fills.length > 1 && (
                    <TableFooter className="border-t-[3px] border-double border-foreground/70 bg-transparent">
                        <TableRow className="hover:bg-transparent">
                            <TableCell className="text-muted-foreground">Total</TableCell>
                            {/* one cell per column: a colSpan would count hidden ones */}
                            <TableCell />
                            <TableCell className={NUMBER}>
                                {totals.amount && formatDecimal(totals.amount)}
                            </TableCell>
                            <TableCell className={cn(NUMBER, VALUE_COLUMN)}>
                                {totals.value && formatDecimal(totals.value)}
                            </TableCell>
                            <TableCell className={cn(NUMBER, FEE_COLUMN)}>
                                {totals.fee && formatDecimal(totals.fee)}
                            </TableCell>
                            <TableCell className={ROLE_COLUMN} />
                        </TableRow>
                    </TableFooter>
                )}
            </Table>
            {fills && (
                <p className="mt-3 text-sm text-muted-foreground">
                    Maker: the order was waiting in the book when it traded. Taker: it
                    traded as it arrived. Fees are paid in {quote.symbol}.
                </p>
            )}
        </div>
    );
}
