// Hand-written: mirrors OrderQuery in backend/src/v1.rs (query string of
// GET /users/{user_id}/orders). null = not sent = no filter / backend default.
import type { OrderStatusFilter } from "./OrderStatusFilter";

export type OrderQuery = {
    market: string | null;
    status: OrderStatusFilter | null;
    limit: number | null;
    cursor: string | null;
};
