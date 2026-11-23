import { cn } from "@/lib/utils";
import { directionOf, formatPercent, formatSignedDecimal } from "@/lib/format";

// "+900.00 (+1.39%)" in Gain, Loss, or the muted colour when there's no change or no
// trades in the window.
export function ChangeText({
    change,
    percent,
    className,
}: {
    change: string | null;
    percent: string | null;
    className?: string;
}) {
    if (change === null) {
        return (
            <span className={cn("text-muted-foreground", className)}>
                No trades in 24h
            </span>
        );
    }
    const direction = directionOf(change);
    return (
        <span
            className={cn(
                direction === "up" && "text-gain",
                direction === "down" && "text-loss",
                direction === "flat" && "text-muted-foreground",
                className,
            )}>
            {formatSignedDecimal(change)}
            {percent !== null && ` (${formatPercent(percent)})`}
        </span>
    );
}
