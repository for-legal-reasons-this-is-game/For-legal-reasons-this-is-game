// Hand-written: mirrors MarketOrderQuery in backend/src/v1.rs (query string of
// GET /markets/{symbol}/orders). null = not sent = no filter / backend default.
import type { OrderStatusFilter } from "./OrderStatusFilter";

export type MarketOrderQuery = {
    status: OrderStatusFilter | null;
    limit: number | null;
    cursor: string | null;
};
