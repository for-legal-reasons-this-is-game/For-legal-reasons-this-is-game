import type { CancelReason } from "@/types/CancelReason";
import type { Order } from "@/types/Order";
import type { OrderStatusFilter } from "@/types/OrderStatusFilter";
import type { OrderType } from "@/types/OrderType";
import type { RejectReason } from "@/types/RejectReason";
import type { Side } from "@/types/Side";
import type { TimeInForce } from "@/types/TimeInForce";
import type { TriggerDirection } from "@/types/TriggerDirection";

// What the API's values are called on screen, in one place.

export const SIDE_LABELS: Record<Side, string> = { buy: "Buy", sell: "Sell" };

export const ORDER_TYPE_LABELS: Record<OrderType, string> = {
    limit: "Limit",
    market: "Market",
    stop_limit: "Stop limit",
    stop_market: "Stop market",
};

export const TIME_IN_FORCE_LABELS: Record<TimeInForce, string> = {
    gtc: "Good till cancelled",
    gtd: "Good till date",
    ioc: "Immediate or cancel",
    fok: "Fill or kill",
};

export const TRIGGER_DIRECTION_LABELS: Record<TriggerDirection, string> = {
    last_at_or_above: "Last price at or above",
    last_at_or_below: "Last price at or below",
};

// the order list filters; "open" is every order that can still trade
export const STATUS_FILTER_LABELS: Record<OrderStatusFilter, string> = {
    open: "Open",
    new: "New",
    partially_filled: "Partly filled",
    filled: "Filled",
    cancelled: "Cancelled",
    expired: "Expired",
    pending_trigger: "Waiting for trigger",
};

// Why the trading engine rejected an order, as a sentence (see rejectionText)
const REJECT_REASON_LABELS: Record<RejectReason, string> = {
    unknown_market: "The trading engine doesn't know this market.",
    bad_quantity: "The amount isn't valid for this market.",
    bad_price: "The price isn't valid for this market.",
    bad_order_params: "The order's fields don't fit its type.",
    unknown_order: "The trading engine doesn't know this order.",
    order_not_working: "The order wasn't open any more.",
    not_order_owner: "The order belongs to someone else.",
    duplicate_command: "The trading engine had already received this order.",
    unsupported: "The trading engine doesn't support this order type yet.",
    internal_error:
        "The trading engine failed while placing it. The held amount was returned.",
};

// Why an order was rejected, as a sentence. The engine gives a reason. Without one, the
// backend refused the order itself while holding its amount, and its detail says why: the
// two details it writes (advance_order in backend/src/v1.rs) get plain wording here.
export function rejectionText(order: Order): string {
    if (order.reject_reason) return REJECT_REASON_LABELS[order.reject_reason];
    switch (order.reject_detail) {
        case "insufficient funds":
            return "There wasn't enough in the account to hold for it.";
        case "the account is closed, so nothing could be reserved":
            return "The account paying for it is closed.";
        default:
            return "Not enough funds, or an account isn't active.";
    }
}

// why the engine cancelled an order, as a sentence
export const CANCEL_REASON_LABELS: Record<CancelReason, string> = {
    user_request: "You cancelled it.",
    ioc_remainder:
        "The part that didn't fill right away was cancelled (immediate or cancel).",
    ioc_no_fill: "Nothing filled right away, so it was cancelled (immediate or cancel).",
    fok_no_full_fill:
        "It couldn't fill completely right away, so it was cancelled (fill or kill).",
    self_trade: "It would have traded with one of your own orders.",
    protection_price: "The price moved past its protection price.",
    admin: "The exchange cancelled it.",
};
