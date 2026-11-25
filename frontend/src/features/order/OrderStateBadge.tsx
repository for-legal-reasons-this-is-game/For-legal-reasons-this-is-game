import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { Order } from "@/types/Order";
import { rejectionText } from "./labels";

// `hint`: shown in a tooltip, for the states that need a word of explanation
type State = {
    label: string;
    dot: "live" | "done" | "failed" | "waiting";
    hint?: string;
};

// An order has two states: `submission` (the backend placing it with the engine) and
// `status` (the engine's view, written by the event consumer). Between the engine accepting
// an order and the consumer recording what happened to it, there's no status yet: those
// orders show as "Accepted" rather than guessing whether they filled.
function stateOf(order: Order): State {
    if (order.submission === "rejected")
        return { label: "Rejected", dot: "failed", hint: rejectionText(order) };
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
                ? {
                      label: "Accepted",
                      dot: "live",
                      hint: "The trading engine has the order. Its status shows here once the engine reports it.",
                  }
                : {
                      label: "Placing",
                      dot: "waiting",
                      hint: "The exchange is still handing the order to the trading engine.",
                  };
    }
}

// Muted like the market status badge; the dot tells working orders (green) from finished
// (grey), waiting (hollow) and rejected (red) ones.
export function OrderStateBadge({ order }: { order: Order }) {
    const { label, dot, hint } = stateOf(order);
    const badge = (
        <Badge
            variant="outline"
            className="gap-1.5 font-normal"
            // focusable when it has a hint, so keyboard users can read it too
            tabIndex={hint ? 0 : undefined}>
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
    if (!hint) return badge;
    return (
        <Tooltip>
            <TooltipTrigger asChild>{badge}</TooltipTrigger>
            <TooltipContent className="max-w-64">{hint}</TooltipContent>
        </Tooltip>
    );
}
