import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { MarketStatus } from "@/types/MarketStatus";

const LABELS: Record<MarketStatus, string> = {
    trading: "Trading",
    cancel_only: "Cancel only",
    halted: "Halted",
};

// A muted badge; the small dot carries the state (green when orders can be placed).
export function MarketStatusBadge({ status }: { status: MarketStatus }) {
    return (
        <Badge variant="outline" className="gap-1.5 font-normal">
            <span
                aria-hidden
                className={cn(
                    "size-1.5 rounded-full",
                    status === "trading" ? "bg-gain" : "bg-muted-foreground",
                )}
            />
            {LABELS[status]}
        </Badge>
    );
}
