import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

// "Cancel" for one of your open orders (isOpen in order-state.ts). The backend has no cancel
// route yet, so it's shown disabled, with a tooltip saying why (decision 7 in
// frontend/REDESIGN.md). When the route exists: take the order as a prop, add the route's
// call to api/index.ts, use a mutation here, and on success invalidate
// ["users", userID, "orders"], ["markets", order.market] and ["position"] (a cancel gives
// the held amount back).
// `size`: "xs" in tables, the normal size on the order page.
export function CancelOrderButton({ size = "xs" }: { size?: "xs" | "default" }) {
    return (
        <Tooltip>
            <TooltipTrigger asChild>
                {/* A disabled button gets no mouse or focus events, so the tooltip hangs on
                    a wrapper that can be focused. stopPropagation: the row behind it opens
                    the order when clicked. */}
                <span
                    role="button"
                    aria-disabled="true"
                    tabIndex={0}
                    className="inline-flex rounded-lg outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                    aria-label="Cancel order: not available yet"
                    onClick={(event) => event.stopPropagation()}>
                    <Button variant="outline" size={size} disabled tabIndex={-1}>
                        {size === "xs" ? "Cancel" : "Cancel order"}
                    </Button>
                </span>
            </TooltipTrigger>
            <TooltipContent>Cancelling orders isn't available yet</TooltipContent>
        </Tooltip>
    );
}
