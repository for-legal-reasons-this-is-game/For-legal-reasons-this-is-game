// One function per backend route (backend/src/main.rs). If a route changes,
// this is the only file to update. Types come from ts-rs.
import { http } from "./client";
import type { User } from "../types/User";
import type { Account } from "../types/Account";
import type { AccountPayload } from "../types/AccountPayload";
import type { Position } from "../types/Position";
import type { Ledger } from "../types/Ledger";
import type { LedgerPayload } from "../types/LedgerPayload";
import type { UserPayload } from "../types/UserPayload";

export { ApiError, errorMessage } from "./client";

// encodeURIComponent so a symbol like "BTC/USD" can't break the URL.
const seg = encodeURIComponent;

export const api = {
    users: {
        list: () => http.get<User[]>("/users"),
        get: (userId: string) => http.get<User>(`/users/${seg(userId)}`),
        create: (payload: UserPayload) => http.post<User>("/users", payload),
        createAccount: (userId: string, payload: AccountPayload) =>
            http.post<Account>(`/users/${seg(userId)}/accounts`, payload),
    },
    accounts: {
        get: (accountId: string) => http.get<Account>(`/accounts/${seg(accountId)}`),
        position: (accountId: string) =>
            http.get<Position>(`/accounts/${seg(accountId)}/position`),
    },
    ledgers: {
        list: () => http.get<Ledger[]>("/ledgers"),
        get: (symbol: string) => http.get<Ledger>(`/ledgers/${seg(symbol)}`),
        create: (payload: LedgerPayload) => http.post<Ledger>("/ledgers", payload),
        setEnabled: (symbol: string, enabled: boolean) =>
            http.patch<Ledger>(`/ledgers/${seg(symbol)}`, { enabled }),
    },
};
