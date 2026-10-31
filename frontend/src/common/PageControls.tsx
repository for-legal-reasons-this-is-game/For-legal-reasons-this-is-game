// Paging controls shared by every list endpoint that returns { items, next_cursor }
// (transfers, orders, trades). The backend pages with a cursor: each page comes with
// a next_cursor, and sending it back as `cursor` returns the page after it.
// next_cursor is null on the last page.

// Page sizes to choose from (the backend's default is 50, its maximum 100).
const PAGE_SIZES = [10, 20, 50, 100];

type PageQuery = { limit: number | null; cursor: string | null };

// New TypeScript: a generic with a constraint. <Q extends PageQuery> means "Q can be
// any type, as long as it has at least limit and cursor". So the same component
// works for TransferQuery, OrderQuery, MarketOrderQuery..., and setQuery gets back
// the full query, with its other fields (status, market) still in it.
export function PageControls<Q extends PageQuery>({
    query,
    setQuery,
    nextCursor,
    isPlaceholderData,
}: {
    query: Q;
    setQuery: (query: Q) => void;
    nextCursor: string | null | undefined;
    // true while the next page is loading and the previous one is still shown
    isPlaceholderData: boolean;
}) {
    return (
        <div>
            {/* New: `a ?? b` means "a, unless a is null or undefined, then b".
                limit is null until the user picks one: the backend then uses 50. */}
            <select
                value={query.limit ?? 50}
                onChange={(event) =>
                    // a new page size starts again from the first page
                    setQuery({
                        ...query,
                        limit: Number(event.target.value),
                        cursor: null,
                    })
                }>
                {PAGE_SIZES.map((size) => (
                    <option key={size} value={size}>
                        {size} per page
                    </option>
                ))}
            </select>
            <button
                type="button"
                disabled={query.cursor === null}
                onClick={() => setQuery({ ...query, cursor: null })}>
                First page
            </button>
            {/* Disabled on the last page, and while the next page is loading so a
                second click can't skip a page using the old next_cursor. */}
            <button
                type="button"
                disabled={!nextCursor || isPlaceholderData}
                onClick={() => nextCursor && setQuery({ ...query, cursor: nextCursor })}>
                Next page
            </button>
        </div>
    );
}
