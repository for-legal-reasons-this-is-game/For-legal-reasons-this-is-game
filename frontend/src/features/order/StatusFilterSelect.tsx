import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import type { OrderStatusFilter } from "@/types/OrderStatusFilter";
import { STATUS_FILTER_LABELS } from "./labels";

// The select's items can't have "" as a value, so "all" stands for no filter (null)
const ALL = "all";

// Status filter for the order lists. null = all statuses (the filter isn't sent). "open"
// = every order that can still trade: not rejected, and still being placed, new, partly
// filled or waiting for its trigger.
export function StatusFilterSelect({
    status,
    onChange,
}: {
    status: OrderStatusFilter | null;
    onChange: (status: OrderStatusFilter | null) => void;
}) {
    return (
        <Select
            value={status ?? ALL}
            // New TypeScript: `as` is a type assertion. The select gives a plain string;
            // `as OrderStatusFilter` tells TypeScript it's one of the filter values. That's
            // safe here because the items below are exactly those values, but `as` is never
            // checked, so only use it when you're sure.
            onValueChange={(value) =>
                onChange(value === ALL ? null : (value as OrderStatusFilter))
            }>
            <SelectTrigger size="sm" aria-label="Order status">
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <SelectItem value={ALL}>All statuses</SelectItem>
                {Object.entries(STATUS_FILTER_LABELS).map(([value, label]) => (
                    <SelectItem key={value} value={value}>
                        {label}
                    </SelectItem>
                ))}
            </SelectContent>
        </Select>
    );
}
