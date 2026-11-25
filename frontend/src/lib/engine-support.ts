import type { OrderType } from "@/types/OrderType";
import type { TimeInForce } from "@/types/TimeInForce";

// What the exchange can do today, in one place. Anything switched off here is still shown,
// marked "not available yet" (decisions 5 and 7 in frontend/REDESIGN.md); when it arrives,
// switch it on here and the UI uses it.

// What the trading engine accepts: tradingengine/src/place.rs rejects every other order
// type and time in force as UNSUPPORTED. The order form lists the others as "not available
// yet". When the engine gains one, add it here and the form enables it.
export const ENGINE_SUPPORTS: { orderTypes: OrderType[]; timeInForce: TimeInForce[] } = {
    orderTypes: ["limit"],
    timeInForce: ["gtc"],
};

// Not a flag, because it needs code as well: cancelling orders. The engine's CancelOrder is
// planned before the evaluation, but the backend has no cancel route yet, so
// features/order/CancelOrderButton.tsx shows a disabled "Cancel" until it has one.
