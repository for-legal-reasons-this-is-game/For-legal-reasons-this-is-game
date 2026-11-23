import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Order } from "@/types/Order";

type State = { label: string; dot: "live" | "done" | "failed" | "waiting" };

// An order has two states: `submission` (the backend placing it with the engine) and
// `status` (the engine's view, written by the event consumer). The consumer can't run
// yet (REDESIGN.md, "Backend dependencies"), so accepted orders still have no status;
// they show as "Accepted" rather than guessing whether they filled.
function stateOf(order: Order): State {
    if (order.submission === "rejected") return { label: "Rejected", dot: "failed" };
    switch (order.status) {
        case "new":
            return { label: "Open", dot: "live" };
        case "partially_filled":
            return { label: "Partly filled", dot: "live" };
        case "pending_trigger":
            return { label: "Waiting for trigger", dot: "waiting" };
        case "filled":
            return { label: "Filled", dot: "done" };
        case "cancelled":
            return { label: "Cancelled", dot: "done" };
        case "expired":
            return { label: "Expired", dot: "done" };
        case null:
            return order.submission === "accepted"
                ? { label: "Accepted", dot: "live" }
                : { label: "Placing", dot: "waiting" };
    }
}

// Muted like the market status badge; the dot tells working orders (green) from finished
// (grey), waiting (hollow) and rejected (red) ones.
export function OrderStateBadge({ order }: { order: Order }) {
    const { label, dot } = stateOf(order);
    return (
        <Badge variant="outline" className="gap-1.5 font-normal">
            <span
                aria-hidden
                className={cn(
                    "size-1.5 rounded-full",
                    dot === "live" && "bg-gain",
                    dot === "done" && "bg-muted-foreground",
                    dot === "waiting" && "border border-muted-foreground",
                    dot === "failed" && "bg-destructive",
                )}
            />
            {label}
        </Badge>
    );
}
