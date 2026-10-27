// Hand-written: mirrors OrderStatusFilter in backend/src/v1.rs, which has no ts-rs
// export. Keep it in sync by hand. "open" = every order that can still trade: not
// rejected, and still being submitted, new, partially filled or waiting for its trigger.
export type OrderStatusFilter =
    | "open"
    | "new"
    | "partially_filled"
    | "filled"
    | "cancelled"
    | "expired"
    | "pending_trigger";
