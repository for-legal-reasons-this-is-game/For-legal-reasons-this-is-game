import { cn } from "@/lib/utils";
import { directionOf, formatPercent, formatSignedDecimal } from "@/lib/format";

// "+900.00 (+1.39%)" in Gain, Loss, or the muted colour when there's no change or no
// trades in the window. `short`: just "+1.39%", for narrow tables.
export function ChangeText({
    change,
    percent,
    short = false,
    className,
}: {
    change: string | null;
    percent: string | null;
    short?: boolean;
    className?: string;
}) {
    if (change === null) {
        return (
            <span className={cn("text-muted-foreground", className)}>
                {short ? "No trades" : "No trades in 24h"}
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
            {short && percent !== null ? (
                formatPercent(percent)
            ) : (
                <>
                    {formatSignedDecimal(change)}
                    {percent !== null && ` (${formatPercent(percent)})`}
                </>
            )}
        </span>
    );
}
