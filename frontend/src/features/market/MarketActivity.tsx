import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { errorMessage } from "@/api";
import { marketQueries, userQueries } from "@/api/queries";
import { useActingUser } from "@/app/acting-user/context";
import { TablePager } from "@/components/TablePager";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { OrdersTable } from "@/features/order/OrdersTable";
import { StatusFilterSelect } from "@/features/order/StatusFilterSelect";
import { ChooseUserHint } from "@/features/user/ChooseUserHint";
import { formatDecimal, formatWhen } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Market } from "@/types/Market";
import type { MarketOrderQuery } from "@/types/MarketOrderQuery";
import type { MarketTradeQuery } from "@/types/MarketTradeQuery";
import type { OrderQuery } from "@/types/OrderQuery";

const NUMBER = "text-right";
// a narrow panel (under 24rem = 384 px, see the container queries in OrdersTable) leaves
// out a trade's value: price times amount
const VALUE_COLUMN = "hidden @sm:table-cell";

function RecentTrades({ market }: { market: Market }) {
    const [query, setQuery] = useState<MarketTradeQuery>({ limit: 20, cursor: null });
    const trades = useQuery(marketQueries.trades(market.symbol, query));
    const base = market.base_asset.symbol;
    const quote = market.quote_asset.symbol;

    if (trades.isError) {
        return (
            <p role="alert" className="py-4 text-destructive">
                Couldn't load the trades: {errorMessage(trades.error)}
            </p>
        );
    }
    if (trades.data && trades.data.items.length === 0 && query.cursor === null) {
        return (
            <p className="py-4 text-muted-foreground">
                No trades yet. Trades appear here when a buy and a sell order meet.
            </p>
        );
    }

    return (
        <div className="@container flex flex-col gap-3">
            <Table className="font-stretch-semi-condensed">
                <TableHeader>
                    <TableRow>
                        <TableHead>Time</TableHead>
                        <TableHead className={NUMBER}>Price ({quote})</TableHead>
                        <TableHead className={NUMBER}>Amount ({base})</TableHead>
                        <TableHead className={cn(NUMBER, VALUE_COLUMN)}>
                            Value ({quote})
                        </TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {trades.data
                        ? trades.data.items.map((trade) => (
                              <TableRow key={trade.trade_id}>
                                  <TableCell>{formatWhen(trade.engine_time)}</TableCell>
                                  {/* coloured by the side that took the price: a buyer
                                      lifting an offer is Gain, a seller hitting a bid Loss */}
                                  <TableCell
                                      className={cn(
                                          NUMBER,
                                          trade.taker_side === "buy"
                                              ? "text-gain"
                                              : "text-loss",
                                      )}>
                                      {formatDecimal(trade.price)}
                                  </TableCell>
                                  <TableCell className={NUMBER}>
                                      {formatDecimal(trade.base_quantity)}
                                  </TableCell>
                                  <TableCell className={cn(NUMBER, VALUE_COLUMN)}>
                                      {formatDecimal(trade.quote_quantity)}
                                  </TableCell>
                              </TableRow>
                          ))
                        : [0, 1, 2].map((row) => (
                              <TableRow key={row}>
                                  {["", "", "", VALUE_COLUMN].map((className, cell) => (
                                      <TableCell key={cell} className={className}>
                                          <Skeleton className="h-4 w-full" />
                                      </TableCell>
                                  ))}
                              </TableRow>
                          ))}
                </TableBody>
            </Table>
            <TablePager
                query={query}
                setQuery={setQuery}
                nextCursor={trades.data?.next_cursor}
                isPlaceholderData={trades.isPlaceholderData}
            />
        </div>
    );
}

function MarketOrders({ market }: { market: Market }) {
    const [query, setQuery] = useState<MarketOrderQuery>({
        status: null,
        limit: 20,
        cursor: null,
    });
    const orders = useQuery(marketQueries.orders(market.symbol, query));

    return (
        <div className="flex flex-col gap-3">
            <div className="flex justify-end">
                {/* a new filter starts again from the first page */}
                <StatusFilterSelect
                    status={query.status}
                    onChange={(status) => setQuery({ ...query, status, cursor: null })}
                />
            </div>
            <OrdersTable
                result={orders}
                market={market}
                empty={
                    query.status
                        ? "No orders with this status."
                        : `No orders in ${market.symbol} yet. Be the first: place one with the order form.`
                }
            />
            <TablePager
                query={query}
                setQuery={setQuery}
                nextCursor={orders.data?.next_cursor}
                isPlaceholderData={orders.isPlaceholderData}
            />
        </div>
    );
}

function MyOrders({ market }: { market: Market }) {
    const { userID } = useActingUser();
    const [query, setQuery] = useState<OrderQuery>({
        market: market.symbol,
        status: null,
        limit: 20,
        cursor: null,
    });
    const orders = useQuery(userQueries.orders(userID, query));

    if (!userID) {
        return (
            <p className="py-4 text-muted-foreground">
                <ChooseUserHint goal="to see their orders here" />
            </p>
        );
    }

    return (
        <div className="flex flex-col gap-3">
            <div className="flex justify-end">
                <StatusFilterSelect
                    status={query.status}
                    onChange={(status) => setQuery({ ...query, status, cursor: null })}
                />
            </div>
            <OrdersTable
                result={orders}
                market={market}
                cancellable
                empty={
                    query.status
                        ? "None of your orders here have this status."
                        : `You have no orders in ${market.symbol} yet.`
                }
            />
            <TablePager
                query={query}
                setQuery={setQuery}
                nextCursor={orders.data?.next_cursor}
                isPlaceholderData={orders.isPlaceholderData}
            />
        </div>
    );
}

// The tabs under the chart. Each tab keeps its own paging and filter; Radix removes an
// inactive tab's content, so switching tabs starts it again from the first page.
export function MarketActivity({ market }: { market: Market }) {
    return (
        <Tabs defaultValue="trades" className="gap-3 p-4">
            <TabsList variant="line" className="-ml-1.5">
                <TabsTrigger value="trades">Recent trades</TabsTrigger>
                <TabsTrigger value="orders">All orders</TabsTrigger>
                <TabsTrigger value="mine">My orders</TabsTrigger>
            </TabsList>
            <TabsContent value="trades">
                <RecentTrades market={market} />
            </TabsContent>
            <TabsContent value="orders">
                <MarketOrders market={market} />
            </TabsContent>
            <TabsContent value="mine">
                <MyOrders market={market} />
            </TabsContent>
        </Tabs>
    );
}
