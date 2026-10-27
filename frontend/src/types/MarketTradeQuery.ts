// Hand-written: mirrors MarketTradeQuery in backend/src/v1.rs (query string of
// GET /markets/{symbol}/trades). null = not sent = backend default.
export type MarketTradeQuery = {
    limit: number | null;
    cursor: string | null;
};
