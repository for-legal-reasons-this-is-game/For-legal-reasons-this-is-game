import type { Trade } from "@/types/Trade";
import { parseTime } from "./format";

// There's no candles endpoint yet, so the chart builds candles in the browser from the
// market's recent trades (frontend/REDESIGN.md, "Chart data").

export const INTERVALS = ["1m", "5m", "15m", "1h", "1d"] as const;
// New TypeScript: `as const` keeps the exact strings, and typeof INTERVALS[number] turns
// the list into the type "1m" | "5m" | "15m" | "1h" | "1d".
export type Interval = (typeof INTERVALS)[number];

const SECONDS: Record<Interval, number> = {
    "1m": 60,
    "5m": 5 * 60,
    "15m": 15 * 60,
    "1h": 60 * 60,
    "1d": 24 * 60 * 60,
};

// time: the candle's start, in seconds since 1970 (what the chart library expects),
// shifted to the viewer's time zone (see localSeconds)
export type Candle = {
    time: number;
    open: number;
    high: number;
    low: number;
    close: number;
    volume: number;
};

// The chart library shows every time as UTC. Its docs suggest shifting the times by the
// viewer's time zone instead, so the chart shows the same clock as the tables (and a
// "1d" candle starts at local midnight): 14:02 in Vienna becomes 14:02 "UTC".
function localSeconds(date: Date): number {
    return (
        Date.UTC(
            date.getFullYear(),
            date.getMonth(),
            date.getDate(),
            date.getHours(),
            date.getMinutes(),
            date.getSeconds(),
        ) / 1000
    );
}

// Groups trades into one candle per interval: first price (open), highest, lowest, last
// price (close) and the base quantity traded. Prices become numbers here because the
// chart library draws with numbers; a drawing doesn't need exact decimals, and the
// numbers printed on the page never come from here.
export function toCandles(trades: Trade[], interval: Interval): Candle[] {
    const size = SECONDS[interval];
    // the API sends newest first; candles are built oldest first
    const chronological = [...trades].sort(
        (a, b) => parseTime(a.engine_time).getTime() - parseTime(b.engine_time).getTime(),
    );

    const candles: Candle[] = [];
    for (const trade of chronological) {
        const seconds = localSeconds(parseTime(trade.engine_time));
        const time = seconds - (seconds % size);
        const price = Number(trade.price);
        const quantity = Number(trade.base_quantity);
        const last = candles.at(-1);
        if (last && last.time === time) {
            last.high = Math.max(last.high, price);
            last.low = Math.min(last.low, price);
            last.close = price;
            last.volume += quantity;
        } else {
            candles.push({
                time,
                open: price,
                high: price,
                low: price,
                close: price,
                volume: quantity,
            });
        }
    }
    return candles;
}
