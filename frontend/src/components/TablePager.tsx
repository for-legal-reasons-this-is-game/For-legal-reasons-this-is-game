import { Button } from "@/components/ui/button";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";

// Paging for the list endpoints that return { items, next_cursor } (transfers, orders,
// trades): each page comes with next_cursor, and sending it back as `cursor` returns the
// page after it (null on the last page).

// page sizes to choose from (the backend's default is 50, its maximum 100)
const PAGE_SIZES = ["10", "20", "50", "100"];

type PageQuery = { limit: number | null; cursor: string | null };

// New TypeScript: a generic with a constraint. <Q extends PageQuery> means "Q can be any
// type, as long as it has at least limit and cursor". So the same component works for
// TransferQuery, OrderQuery, MarketOrderQuery..., and setQuery gets back the full query,
// with its other fields (status, market) still in it.
export function TablePager<Q extends PageQuery>({
    query,
    setQuery,
    nextCursor,
    isPlaceholderData,
}: {
    query: Q;
    setQuery: (query: Q) => void;
    nextCursor: string | null | undefined;
    // true while the next page loads and the previous one is still shown
    isPlaceholderData: boolean;
}) {
    return (
        <div className="flex flex-wrap items-center justify-end gap-2 text-sm">
            <Select
                // limit is null until a size is picked: the backend then uses 50
                value={String(query.limit ?? 50)}
                onValueChange={(size) =>
                    // a new page size starts again from the first page
                    setQuery({ ...query, limit: Number(size), cursor: null })
                }>
                <SelectTrigger size="sm" aria-label="Rows per page">
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    {PAGE_SIZES.map((size) => (
                        <SelectItem key={size} value={size}>
                            {size} per page
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            <Button
                variant="outline"
                size="sm"
                disabled={query.cursor === null}
                onClick={() => setQuery({ ...query, cursor: null })}>
                First page
            </Button>
            {/* disabled on the last page, and while the next page loads so a second
                click can't skip a page using the old next_cursor */}
            <Button
                variant="outline"
                size="sm"
                disabled={!nextCursor || isPlaceholderData}
                onClick={() => nextCursor && setQuery({ ...query, cursor: nextCursor })}>
                Next page
            </Button>
        </div>
    );
}
