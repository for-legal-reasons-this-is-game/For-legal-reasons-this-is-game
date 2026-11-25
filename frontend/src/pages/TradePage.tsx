import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "react-router";
import { errorMessage } from "@/api";
import { marketQueries } from "@/api/queries";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import {
    Sheet,
    SheetContent,
    SheetDescription,
    SheetHeader,
    SheetTitle,
} from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { MarketActivity } from "@/features/market/MarketActivity";
import { PriceChart } from "@/features/market/PriceChart";
import { PriceHeader } from "@/features/market/PriceHeader";
import { OrderTicket } from "@/features/order/OrderTicket";
import { formatDecimal } from "@/lib/format";
import type { Market } from "@/types/Market";
import type { Side } from "@/types/Side";

const APP_NAME = "For legal reasons this is game";

// The browser tab shows the live price, like most exchanges: "65,450.00 BTC-USD - ..."
function TabTitle({ symbol }: { symbol: string }) {
    const ticker = useQuery(marketQueries.ticker(symbol));
    const price = ticker.data?.last_price;
    return (
        <title>{`${price ? `${formatDecimal(price)} ` : ""}${symbol} - ${APP_NAME}`}</title>
    );
}

// On narrow screens the order form isn't beside the chart: Buy and Sell stay at the
// bottom of the screen and open it in a sheet.
function MobileTradeBar({ market }: { market: Market }) {
    const [open, setOpen] = useState(false);
    const [side, setSide] = useState<Side>("buy");
    const bar = useRef<HTMLDivElement>(null);

    // Tells the toasts how tall this bar is, so they show above it instead of covering
    // Buy and Sell (see components/ui/sonner.tsx); 0 while it's hidden beside a wide page.
    // New Web API: a ResizeObserver calls back whenever an element's size changes,
    // including when it's hidden (its height becomes 0) or shown again.
    useEffect(() => {
        const element = bar.current;
        if (!element) return;
        const root = document.documentElement;
        const observer = new ResizeObserver(() =>
            root.style.setProperty("--toast-lift", `${element.offsetHeight}px`),
        );
        observer.observe(element);
        return () => {
            observer.disconnect();
            root.style.removeProperty("--toast-lift");
        };
    }, []);

    function openTicket(next: Side) {
        setSide(next);
        setOpen(true);
    }

    return (
        <>
            {/* sticky inside the page: it stays at the bottom of the screen while the
                page scrolls. The negative margins reach the page's padding edges. */}
            <div
                ref={bar}
                className="sticky bottom-0 z-10 -mx-4 -mb-4 mt-4 grid grid-cols-2 gap-3 border-t bg-background p-3 md:-mx-6 md:-mb-6 @4xl:hidden">
                <Button
                    size="lg"
                    className="bg-gain text-gain-foreground hover:bg-gain/85"
                    onClick={() => openTicket("buy")}>
                    Buy
                </Button>
                <Button
                    size="lg"
                    className="bg-loss text-loss-foreground hover:bg-loss/85"
                    onClick={() => openTicket("sell")}>
                    Sell
                </Button>
            </div>
            <Sheet open={open} onOpenChange={setOpen}>
                <SheetContent side="bottom" className="max-h-[90svh] overflow-y-auto">
                    <SheetHeader>
                        <SheetTitle>Trade {market.symbol}</SheetTitle>
                        <SheetDescription className="sr-only">
                            Place a buy or sell order in {market.symbol}
                        </SheetDescription>
                    </SheetHeader>
                    <div className="px-4 pb-4">
                        {/* the sheet's content is created when it opens, so the ticket
                            starts on the side that was tapped */}
                        <OrderTicket
                            market={market}
                            initialSide={side}
                            onPlaced={() => setOpen(false)}
                        />
                    </div>
                </SheetContent>
            </Sheet>
        </>
    );
}

function TradeView({ market }: { market: Market }) {
    return (
        // New Tailwind: @container makes this element a size container, so the @4xl:
        // classes inside react to its width instead of the window's. The sidebar can be
        // open or collapsed, so the space the page really has is what matters.
        <div className="@container">
            <TabTitle symbol={market.symbol} />
            <div className="grid gap-4 @4xl:grid-cols-[minmax(0,1fr)_20rem]">
                <div className="@4xl:col-span-2">
                    <PriceHeader market={market} />
                </div>
                <div className="flex min-w-0 flex-col gap-4">
                    <section
                        aria-label="Price chart"
                        className="rounded-2xl border bg-card">
                        <PriceChart market={market} />
                    </section>
                    <section
                        aria-label="Trades and orders"
                        className="rounded-2xl border bg-card">
                        <MarketActivity market={market} />
                    </section>
                </div>
                {/* beside the chart when there's room; the sheet below otherwise */}
                <aside className="hidden @4xl:block">
                    {/* top-18: below the 56 px top bar, plus a gap. On a short screen it
                        scrolls inside itself; otherwise a sticky box taller than the screen
                        would keep its button out of reach. */}
                    <div className="sticky top-18 max-h-[calc(100svh-5.5rem)] overflow-y-auto rounded-2xl border bg-card p-4">
                        <OrderTicket market={market} />
                    </div>
                </aside>
            </div>
            <MobileTradeBar market={market} />
        </div>
    );
}

export function TradePage() {
    // New React Router: useParams() reads the ":symbol" part of the route's path
    const { symbol = "" } = useParams();
    const market = useQuery(marketQueries.detail(symbol));

    if (market.isPending) {
        return (
            <div className="grid gap-4">
                <Skeleton className="h-12 w-80" />
                <Skeleton className="h-96" />
            </div>
        );
    }

    if (market.isError) {
        return (
            <>
                <title>{`${symbol} - ${APP_NAME}`}</title>
                {market.error.status === 404 ? (
                    <>
                        <PageHeader
                            title="Market not found"
                            description={`There's no market called ${symbol}. Check the symbol, or choose a market from the list.`}
                        />
                        <Button asChild>
                            <Link to="/markets">Go to markets</Link>
                        </Button>
                    </>
                ) : (
                    <p role="alert" className="text-destructive">
                        Couldn't load {symbol}: {errorMessage(market.error)}
                    </p>
                )}
            </>
        );
    }

    // key: another market is a fresh page (paging, filters and the form start over)
    return <TradeView key={market.data.symbol} market={market.data} />;
}
