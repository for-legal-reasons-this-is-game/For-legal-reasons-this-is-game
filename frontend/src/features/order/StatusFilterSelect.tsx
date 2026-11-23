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

// Status filter for the order lists: the shadcn version of order/OrderStatusSelect.
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
            // `as` is safe: the items below are exactly the filter values (see
            // OrderStatusSelect for what `as` does)
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
