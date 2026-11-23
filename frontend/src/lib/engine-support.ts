import type { OrderType } from "@/types/OrderType";
import type { TimeInForce } from "@/types/TimeInForce";

// What the trading engine accepts today: tradingengine/src/place.rs rejects every other
// order type and time in force as UNSUPPORTED. The order form still shows the others,
// marked "not available yet" (decision 5 in frontend/REDESIGN.md). When the engine
// gains one, add it here and the form enables it.
export const ENGINE_SUPPORTS: { orderTypes: OrderType[]; timeInForce: TimeInForce[] } = {
    orderTypes: ["limit"],
    timeInForce: ["gtc"],
};
