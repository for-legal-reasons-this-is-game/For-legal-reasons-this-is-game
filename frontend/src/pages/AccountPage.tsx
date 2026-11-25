import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { ChevronLeftIcon, CopyIcon } from "lucide-react";
import { Link, useParams } from "react-router";
import { toast } from "sonner";
import { errorMessage } from "@/api";
import { accountQueries, ledgerQueries, userQueries } from "@/api/queries";
import { useActingUser } from "@/app/acting-user/context";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { AccountActions } from "@/features/account/AccountActions";
import { AccountStatusBadge } from "@/features/account/AccountStatusBadge";
import { rememberAccount } from "@/features/account/known-accounts";
import { LedgerSummary } from "@/features/account/LedgerSummary";
import { TransfersTable } from "@/features/account/TransfersTable";
import type { Account } from "@/types/Account";

const APP_NAME = "For legal reasons this is game";

function AccountView({ account }: { account: Account }) {
    const { userID } = useActingUser();
    const ledgers = useQuery(ledgerQueries.list());
    const users = useQuery(userQueries.list());
    const position = useQuery(accountQueries.position(account.account_id));

    const ledger = ledgers.data?.find((l) => l.ledger_id === account.account_ledger_id);
    const owner = users.data?.find((u) => u.user_id === account.account_user_id);
    const mine = account.account_user_id === userID;
    const asset =
        ledger ??
        (position.data && {
            symbol: position.data.symbol,
            decimals: position.data.decimals,
        });

    // Visiting your own account adds it to the accounts this browser knows about (there's
    // no endpoint that lists them yet). An effect, because it changes something outside
    // React (localStorage), which a render must not do.
    useEffect(() => {
        if (mine && account.account_code_type === 1)
            rememberAccount(userID, account.account_id);
    }, [mine, userID, account.account_id, account.account_code_type]);

    async function copyID() {
        try {
            await navigator.clipboard.writeText(account.account_id);
            toast.success("Account ID copied");
        } catch {
            toast.error("Couldn't copy the ID. Select it on the page instead.");
        }
    }

    return (
        <>
            <title>{`${account.account_name} - ${APP_NAME}`}</title>
            <Link
                to="/portfolio"
                className="mb-2 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
                <ChevronLeftIcon className="size-4" />
                Portfolio
            </Link>
            <PageHeader
                title={account.account_name}
                description={
                    <>
                        {ledger ? `${ledger.name} (${ledger.symbol})` : "Asset"} account
                        of {owner?.user_name ?? "..."}
                        {!mine && userID && " (not the user you're acting as)"}
                        <span className="mt-1 flex items-center gap-1 text-sm">
                            ID {account.account_id}
                            <Button
                                variant="ghost"
                                size="icon-xs"
                                onClick={copyID}
                                aria-label="Copy account ID">
                                <CopyIcon />
                            </Button>
                        </span>
                    </>
                }
                actions={
                    <>
                        <AccountStatusBadge status={account.account_status} />
                        <AccountActions account={account} asset={asset} />
                    </>
                }
            />
            <div className="grid gap-4 lg:grid-cols-[minmax(0,22rem)_minmax(0,1fr)] lg:items-start">
                {position.isError ? (
                    <p role="alert" className="text-destructive">
                        Couldn't load the balance: {errorMessage(position.error)}
                    </p>
                ) : position.data ? (
                    <LedgerSummary position={position.data} />
                ) : (
                    <Skeleton className="h-48 rounded-2xl" />
                )}
                <TransfersTable accountID={account.account_id} />
            </div>
        </>
    );
}

export function AccountPage() {
    // New React Router: useParams() reads the ":accountId" part of the route's path
    const { accountId = "" } = useParams();
    const account = useQuery(accountQueries.detail(accountId));

    if (account.isPending) {
        return (
            <div className="grid gap-4">
                <Skeleton className="h-10 w-72" />
                <Skeleton className="h-48" />
            </div>
        );
    }
    if (account.isError) {
        // 400: not even an account ID (the backend can't read it as one)
        const missing = account.error.status === 404 || account.error.status === 400;
        return (
            <>
                <title>{`Account - ${APP_NAME}`}</title>
                {missing ? (
                    <>
                        <PageHeader
                            title="Account not found"
                            description={`There's no account with the ID ${accountId}.`}
                        />
                        <Button asChild>
                            <Link to="/portfolio">Go to portfolio</Link>
                        </Button>
                    </>
                ) : (
                    <p role="alert" className="text-destructive">
                        Couldn't load the account: {errorMessage(account.error)}
                    </p>
                )}
            </>
        );
    }

    // key: another account is a fresh page (paging starts over)
    return <AccountView key={account.data.account_id} account={account.data} />;
}
