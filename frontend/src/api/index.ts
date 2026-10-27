// One function per backend route (backend/src/main.rs). If a route changes,
// this is the only file to update. Types come from ts-rs.
import { http } from "./client";
import type { User } from "../types/User";
import type { Account } from "../types/Account";
import type { AccountPayload } from "../types/AccountPayload";
import type { Position } from "../types/Position";
import type { Ledger } from "../types/Ledger";
import type { LedgerPayload } from "../types/LedgerPayload";
import type { LedgerEnabledPayload } from "../types/LedgerEnabledPayload";
import type { UserPayload } from "../types/UserPayload";
import type { DepositPayload } from "../types/DepositPayload";
import type { Transfer } from "../types/Transfer";
import type { TransferQuery } from "../types/TransferQuery";
import type { TransferPage } from "../types/TransferPage";
import type { Market } from "../types/Market";
import type { Ticker } from "../types/Ticker";
import type { Order } from "../types/Order";
import type { OrderPage } from "../types/OrderPage";
import type { OrderQuery } from "../types/OrderQuery";
import type { MarketOrderQuery } from "../types/MarketOrderQuery";
import type { Trade } from "../types/Trade";
import type { TradePage } from "../types/TradePage";
import type { MarketTradeQuery } from "../types/MarketTradeQuery";

export { ApiError, errorMessage } from "./client";

// encodeURIComponent so a symbol like "BTC/USD" can't break the URL.
const seg = encodeURIComponent;

// Builds "?limit=20&cursor=..." from a query object like TransferQuery.
// null/"" values are left out entirely: the backend reads a missing parameter as
// "not set" (None), but an empty one ("limit=") as an invalid value -> 400.
//
// New TypeScript: Record<string, V> means "an object with any keys, whose values
// are all V". Every query type here (TransferQuery, OrderQuery, ...) fits that.
// New Web API: URLSearchParams builds a query string and URL-encodes each value,
// so a cursor with characters like ':' or '+' arrives intact.
function toQueryString(query: Record<string, string | number | null>): string {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
        if (value !== null && value !== "") params.append(key, String(value));
    }
    const queryString = params.toString();
    return queryString ? `?${queryString}` : "";
}

export const api = {
    users: {
        list: () => http.get<User[]>("/users"),
        get: (userId: string) => http.get<User>(`/users/${seg(userId)}`),
        create: (payload: UserPayload) => http.post<User>("/users", payload),
        createAccount: (userId: string, payload: AccountPayload) =>
            http.post<Account>(`/users/${seg(userId)}/accounts`, payload),
        orders: (userId: string, query: OrderQuery) =>
            http.get<OrderPage>(`/users/${seg(userId)}/orders${toQueryString(query)}`),
    },
    accounts: {
        get: (accountId: string) => http.get<Account>(`/accounts/${seg(accountId)}`),
        position: (accountId: string) =>
            http.get<Position>(`/accounts/${seg(accountId)}/position`),
        close: (accountId: string) =>
            http.post<void>(`/accounts/${seg(accountId)}/close`),
        reopen: (accountId: string) =>
            http.post<void>(`/accounts/${seg(accountId)}/reopen`),
        deposit: (accountId: string, payload: DepositPayload) =>
            http.post<Transfer>(`/accounts/${seg(accountId)}/deposit`, payload),
        transfers: (accountId: string, query: TransferQuery) =>
            http.get<TransferPage>(
                `/accounts/${seg(accountId)}/transfers${toQueryString(query)}`,
            ),
    },
    ledgers: {
        list: () => http.get<Ledger[]>("/ledgers"),
        get: (symbol: string) => http.get<Ledger>(`/ledgers/${seg(symbol)}`),
        create: (payload: LedgerPayload) => http.post<Ledger>("/ledgers", payload),
        setEnabled: (symbol: string, payload: LedgerEnabledPayload) =>
            http.patch<Ledger>(`/ledgers/${seg(symbol)}`, payload),
    },
    markets: {
        list: () => http.get<Market[]>("/markets"),
        get: (symbol: string) => http.get<Market>(`/markets/${seg(symbol)}`),
        ticker: (symbol: string) => http.get<Ticker>(`/markets/${seg(symbol)}/ticker`),
        orders: (symbol: string, query: MarketOrderQuery) =>
            http.get<OrderPage>(`/markets/${seg(symbol)}/orders${toQueryString(query)}`),
        trades: (symbol: string, query: MarketTradeQuery) =>
            http.get<TradePage>(`/markets/${seg(symbol)}/trades${toQueryString(query)}`),
    },
    orders: {
        get: (orderId: string) => http.get<Order>(`/orders/${seg(orderId)}`),
        trades: (orderId: string) => http.get<Trade[]>(`/orders/${seg(orderId)}/trades`),
    },
};
