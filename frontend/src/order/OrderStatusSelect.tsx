import type { OrderStatusFilter } from "../types/OrderStatusFilter";

const ORDER_STATUS_FILTERS: OrderStatusFilter[] = [
    "open",
    "new",
    "partially_filled",
    "filled",
    "cancelled",
    "expired",
    "pending_trigger",
];

// Status filter for the order lists. null = all statuses (the filter isn't sent).
// "open" = every order that can still trade: not rejected, and still being
// submitted, new, partially filled or waiting for its trigger.
export function OrderStatusSelect({
    status,
    onChange,
}: {
    status: OrderStatusFilter | null;
    onChange: (status: OrderStatusFilter | null) => void;
}) {
    return (
        <select
            value={status ?? ""}
            onChange={(event) =>
                // New TypeScript: `as` is a type assertion. event.target.value is typed as
                // a plain string; `as OrderStatusFilter` tells TypeScript it's one of the
                // filter values. That's safe here because the options below are exactly
                // those values, but `as` is never checked, so only use it when you're sure.
                onChange(
                    event.target.value === ""
                        ? null
                        : (event.target.value as OrderStatusFilter),
                )
            }>
            <option value="">all statuses</option>
            {ORDER_STATUS_FILTERS.map((filter) => (
                <option key={filter} value={filter}>
                    {filter}
                </option>
            ))}
        </select>
    );
}
