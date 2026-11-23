import { useMemo } from "react";
import { useQueries, useQuery } from "@tanstack/react-query";
import { accountQueries, userQueries } from "@/api/queries";
import type { Account } from "@/types/Account";
import { useKnownAccountIDs } from "./known-accounts";

// account_code_type 1: an account a user trades with (2 to 4 are the exchange's own fee,
// source and hold accounts)
const USER_ACCOUNT = 1;

// The user's accounts as far as the browser knows them: remembered ones
// (known-accounts.ts) plus those named in the user's newest 100 orders.
export function useUserAccounts(userID: string): {
    accounts: Account[];
    isPending: boolean;
} {
    const known = useKnownAccountIDs(userID);
    const orders = useQuery({
        ...userQueries.orders(userID, {
            market: null,
            status: null,
            limit: 100,
            cursor: null,
        }),
        // only used to find account IDs, which rarely change: no polling
        refetchInterval: false,
    });

    const ids = useMemo(() => {
        const all = new Set(known);
        for (const order of orders.data?.items ?? []) {
            all.add(order.base_account_id);
            all.add(order.quote_account_id);
        }
        return [...all];
    }, [known, orders.data]);

    // New TanStack: useQueries runs a list of queries whose length can change between
    // renders (useQuery can't be called in a loop). Each account keeps its own cache
    // entry, shared with the rest of the app. `combine` turns the list of results into
    // the one value this hook returns.
    const details = useQueries({
        queries: ids.map((id) => accountQueries.detail(id)),
        combine: (results) => ({
            accounts: results.flatMap((result) => (result.data ? [result.data] : [])),
            isPending: results.some((result) => result.isPending),
        }),
    });

    const accounts = details.accounts.filter(
        (account) =>
            account.account_user_id === userID &&
            account.account_code_type === USER_ACCOUNT,
    );
    return { accounts, isPending: (!!userID && orders.isPending) || details.isPending };
}
