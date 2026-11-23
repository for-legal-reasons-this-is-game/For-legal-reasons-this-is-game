import { useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
    CandlestickSeries,
    ColorType,
    CrosshairMode,
    HistogramSeries,
    createChart,
    type IChartApi,
    type ISeriesApi,
    type UTCTimestamp,
} from "lightweight-charts";
import { errorMessage } from "@/api";
import { marketQueries } from "@/api/queries";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { INTERVALS, toCandles, type Interval } from "@/lib/candles";
import type { Market } from "@/types/Market";

// The chart's colours come from the theme's CSS variables, so index.css stays the one
// place that defines them.
function cssColor(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

// "#0d7c51" + 0.35 -> "rgba(13, 124, 81, 0.35)", for the paler volume bars
function withAlpha(hex: string, alpha: number): string {
    const value = hex.replace("#", "");
    const [r, g, b] = [0, 2, 4].map((i) => parseInt(value.slice(i, i + 2), 16));
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function applyTheme(chart: IChartApi, candles: ISeriesApi<"Candlestick">) {
    const ruling = cssColor("--border");
    chart.applyOptions({
        layout: {
            background: { type: ColorType.Solid, color: cssColor("--card") },
            textColor: cssColor("--muted-foreground"),
        },
        grid: { vertLines: { color: ruling }, horzLines: { color: ruling } },
        rightPriceScale: { borderColor: ruling },
        timeScale: { borderColor: ruling },
        crosshair: {
            vertLine: {
                color: cssColor("--ring"),
                labelBackgroundColor: cssColor("--ring"),
            },
            horzLine: {
                color: cssColor("--ring"),
                labelBackgroundColor: cssColor("--ring"),
            },
        },
    });
    const gain = cssColor("--gain");
    const loss = cssColor("--loss");
    candles.applyOptions({
        upColor: gain,
        downColor: loss,
        wickUpColor: gain,
        wickDownColor: loss,
    });
}

// Candlesticks and volume, built from the market's recent trades (lib/candles.ts).
export function PriceChart({ market }: { market: Market }) {
    const [candleSize, setCandleSize] = useState<Interval>("5m");
    // counts theme changes, so the volume bars (coloured with the data) are drawn again
    const [themeVersion, setThemeVersion] = useState(0);
    const trades = useQuery(marketQueries.recentTrades(market.symbol));
    // useMemo: rebuild the candles only when the trades or the interval change, not on
    // every render
    const candles = useMemo(
        () => (trades.data ? toCandles(trades.data, candleSize) : []),
        [trades.data, candleSize],
    );

    // New React: useRef holds a value that survives re-renders without causing one. Here:
    // the chart's <div>, and the chart objects the library creates inside it.
    const containerRef = useRef<HTMLDivElement>(null);
    const chartRef = useRef<{
        chart: IChartApi;
        candles: ISeriesApi<"Candlestick">;
        volume: ISeriesApi<"Histogram">;
    } | null>(null);

    // Create the chart once, when the component appears; remove it when it goes away.
    useEffect(() => {
        const container = containerRef.current;
        if (!container) return;
        const chart = createChart(container, {
            // follow the size of the <div> (the library watches it for resizes)
            autoSize: true,
            layout: {
                fontFamily: "Archivo Variable, ui-sans-serif, system-ui, sans-serif",
                fontSize: 12,
                // the TradingView logo link is required by the license (see the note
                // under the chart); it's on by default, this keeps it explicit
                attributionLogo: true,
            },
            crosshair: { mode: CrosshairMode.Normal },
            timeScale: { timeVisible: true, secondsVisible: false },
        });
        const candles = chart.addSeries(CandlestickSeries, { borderVisible: false });
        // volume on its own scale, in the bottom fifth of the chart
        const volume = chart.addSeries(HistogramSeries, {
            priceFormat: { type: "volume" },
            priceScaleId: "volume",
            // no label or line for it on the price axis: the axis is for prices
            lastValueVisible: false,
            priceLineVisible: false,
        });
        chart
            .priceScale("volume")
            .applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
        applyTheme(chart, candles);
        chartRef.current = { chart, candles, volume };

        // Re-colour when the theme changes: next-themes switches the "dark" class on
        // <html>. New browser API: a MutationObserver calls back when the observed
        // element changes (here: its class attribute).
        const observer = new MutationObserver(() => {
            applyTheme(chart, candles);
            setThemeVersion((version) => version + 1);
        });
        observer.observe(document.documentElement, {
            attributes: true,
            attributeFilter: ["class"],
        });

        return () => {
            observer.disconnect();
            chart.remove();
            chartRef.current = null;
        };
    }, []);

    // Show prices like the rest of the page ("63,712.40"), with the market's decimals,
    // and let the axis step by the market's price step
    useEffect(() => {
        const current = chartRef.current;
        if (!current) return;
        const price = new Intl.NumberFormat("en-US", {
            minimumFractionDigits: market.price_decimals,
            maximumFractionDigits: market.price_decimals,
        });
        current.chart.applyOptions({
            localization: { priceFormatter: (value: number) => price.format(value) },
        });
        current.candles.applyOptions({
            priceFormat: {
                type: "price",
                precision: market.price_decimals,
                minMove: Number(market.price_step),
            },
        });
    }, [market.price_decimals, market.price_step]);

    // Hand new candles to the chart; again after a theme change, for the bar colours
    useEffect(() => {
        const current = chartRef.current;
        if (!current) return;
        const gain = withAlpha(cssColor("--gain"), 0.35);
        const loss = withAlpha(cssColor("--loss"), 0.35);
        current.candles.setData(
            candles.map((c) => ({
                // the library's time type: seconds since 1970, marked as UTC
                time: c.time as UTCTimestamp,
                open: c.open,
                high: c.high,
                low: c.low,
                close: c.close,
            })),
        );
        current.volume.setData(
            candles.map((c) => ({
                time: c.time as UTCTimestamp,
                value: c.volume,
                color: c.close >= c.open ? gain : loss,
            })),
        );
    }, [candles, themeVersion]);

    const empty = trades.isSuccess && candles.length === 0;

    return (
        <div className="flex flex-col gap-3 p-4">
            <div className="flex items-center justify-between gap-2">
                <h2 className="font-semibold">Price</h2>
                <ToggleGroup
                    type="single"
                    size="sm"
                    variant="outline"
                    value={candleSize}
                    // a single toggle group sends "" when the active item is clicked again;
                    // keep the current interval then
                    onValueChange={(value) => value && setCandleSize(value as Interval)}
                    aria-label="Candle size">
                    {INTERVALS.map((value) => (
                        <ToggleGroupItem key={value} value={value} className="px-2">
                            {value}
                        </ToggleGroupItem>
                    ))}
                </ToggleGroup>
            </div>
            <div className="relative h-72 md:h-96">
                {/* isolate: the chart's canvases have their own z-index; this keeps
                    them under the loading and empty messages below */}
                <div ref={containerRef} className="absolute inset-0 isolate" />
                {trades.isPending && <Skeleton className="absolute inset-0" />}
                {(empty || trades.isError) && (
                    <div className="absolute inset-0 flex items-center justify-center p-6 text-center">
                        <p className="max-w-[45ch] rounded-lg bg-card/90 px-3 py-2 text-muted-foreground">
                            {trades.isError
                                ? `Couldn't load the trades: ${errorMessage(trades.error)}`
                                : `No trades in ${market.symbol} yet. The chart starts with the first trade.`}
                        </p>
                    </div>
                )}
            </div>
            {/* attribution notice required by the Lightweight Charts license (REDESIGN.md) */}
            <p className="text-xs text-muted-foreground">
                Charts by{" "}
                <a
                    href="https://www.tradingview.com/"
                    target="_blank"
                    rel="noreferrer"
                    className="underline underline-offset-2">
                    TradingView
                </a>{" "}
                Lightweight Charts™, Copyright (c) 2025 TradingView, Inc.
            </p>
        </div>
    );
}
