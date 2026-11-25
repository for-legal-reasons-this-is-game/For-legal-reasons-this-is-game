import type { Order } from "@/types/Order";

// An order that can still be cancelled: not rejected, and not finished as far as we know.
// Without the engine's events, an accepted order has no status yet, so it counts as open.
export function isOpen(order: Order): boolean {
    if (order.submission === "rejected") return false;
    return (
        order.status === null ||
        order.status === "new" ||
        order.status === "partially_filled" ||
        order.status === "pending_trigger"
    );
}
