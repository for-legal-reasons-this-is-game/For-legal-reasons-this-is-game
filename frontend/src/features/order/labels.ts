import type { CancelReason } from "@/types/CancelReason";
import type { OrderStatusFilter } from "@/types/OrderStatusFilter";
import type { OrderType } from "@/types/OrderType";
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
