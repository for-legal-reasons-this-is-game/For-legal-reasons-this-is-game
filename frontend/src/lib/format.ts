// Formatting for what the API sends: decimal strings ("65450.00", "0.80000000") and
// ISO times ("2026-11-23T14:02:11.295729Z").
//
// Amounts stay strings: Intl.NumberFormat formats a numeric string exactly, without
// turning it into a floating-point number first.
import { fromUnits } from "./decimal";

// "." as the decimal separator everywhere, the same one the order form expects
const LOCALE = "en-US";

function decimalsOf(value: string): number {
    const dot = value.indexOf(".");
    return dot === -1 ? 0 : value.length - dot - 1;
}

function formatWith(value: string, options: Intl.NumberFormatOptions): string {
    return new Intl.NumberFormat(LOCALE, options).format(
        // a string is formatted exactly (TypeScript types it as StringNumericLiteral)
        value as Intl.StringNumericLiteral,
    );
}

// "65450.00" -> "65,450.00": thousands separators, the value's own decimals
export function formatDecimal(value: string): string {
    const decimals = decimalsOf(value);
    return formatWith(value, {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals,
    });
}

// For rules and hints, where the asset's full decimals are noise:
// "0.00010000" -> "0.0001", "10.00" -> "10", "1000.50" -> "1,000.5"
export function formatTrimmed(value: string): string {
    const trimmed = value.includes(".") ? value.replace(/\.?0+$/, "") : value;
    return formatDecimal(trimmed);
}

// "900.00" -> "+900.00", "-12.50" -> "-12.50"
export function formatSignedDecimal(value: string): string {
    const decimals = decimalsOf(value);
    return formatWith(value, {
        minimumFractionDigits: decimals,
        maximumFractionDigits: decimals,
        signDisplay: "exceptZero",
    });
}

// A fee rate in basis points as a percentage: 10 -> "0.10%", 250 -> "2.50%"
// (1 bps = 0.01%, so it's the number with two decimals)
export function formatBps(bps: number): string {
    return `${fromUnits(BigInt(bps), 2)}%`;
}

// "1.3942680092951200619674670800" -> "+1.39%"
export function formatPercent(value: string): string {
    return `${formatWith(value, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
        signDisplay: "exceptZero",
    })}%`;
}

export type Direction = "up" | "down" | "flat";

// Direction of a signed decimal string, read from its characters (no conversion)
export function directionOf(value: string | null | undefined): Direction {
    if (!value || !/[1-9]/.test(value)) return "flat";
    return value.startsWith("-") ? "down" : "up";
}

// The API's times have microseconds; JavaScript dates only promise to read
// milliseconds, so the extra digits are dropped first.
export function parseTime(iso: string): Date {
    return new Date(iso.replace(/(\.\d{3})\d+/, "$1"));
}

const TIME = new Intl.DateTimeFormat(LOCALE, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
});
const DATE_TIME = new Intl.DateTimeFormat(LOCALE, {
    dateStyle: "medium",
    timeStyle: "short",
});

// "14:02:11" in the viewer's own time zone
export function formatTime(iso: string): string {
    return TIME.format(parseTime(iso));
}

// "Nov 23, 2026, 2:02 PM"
export function formatDateTime(iso: string): string {
    return DATE_TIME.format(parseTime(iso));
}

const DAY_TIME = new Intl.DateTimeFormat(LOCALE, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
});

function when(date: Date): string {
    return date.toDateString() === new Date().toDateString()
        ? TIME.format(date)
        : DAY_TIME.format(date);
}

// For lists: "14:02:11" today, "Nov 22, 14:02" on other days
export function formatWhen(iso: string): string {
    return when(parseTime(iso));
}

// Transfers come from TigerBeetle, whose timestamps are nanoseconds since 1970 as a string
// ("1795441758269000000"). BigInt divides them down to milliseconds without losing digits.
export function parseLedgerTime(nanoseconds: string): Date {
    return new Date(Number(BigInt(nanoseconds) / 1_000_000n));
}

// formatWhen for a TigerBeetle timestamp
export function formatLedgerWhen(nanoseconds: string): string {
    return when(parseLedgerTime(nanoseconds));
}
