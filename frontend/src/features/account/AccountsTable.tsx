import type { ReactNode } from "react";
import { useQueries, useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";
import { errorMessage } from "@/api";
import { accountQueries, ledgerQueries } from "@/api/queries";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableFooter,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { fromUnits } from "@/lib/decimal";
import { formatDecimal } from "@/lib/format";
import type { Account } from "@/types/Account";
import type { Ledger } from "@/types/Ledger";
import type { Position } from "@/types/Position";
import { AccountStatusBadge } from "./AccountStatusBadge";
import { availableUnits } from "./balance";
import { DepositDialog } from "./DepositDialog";
import { useUserAccounts } from "./use-user-accounts";

const NUMBER = "text-right";
// On phones only the name, what's available and Deposit fit: the asset moves under the
// name, and the status column goes (a "being opened" account still says so)
const ASSET_COLUMN = "hidden sm:table-cell";
const STATUS_COLUMN = "hidden md:table-cell";

function AccountRow({
    account,
    ledger,
    position,
}: {
    account: Account;
    ledger: Ledger | undefined;
    position: Position | undefined;
}) {
    const navigate = useNavigate();
    const units = position ? availableUnits(position) : null;
    const asset =
        ledger ?? (position && { symbol: position.symbol, decimals: position.decimals });

    return (
        // the whole row opens the account; the name is the link for keyboard users
        <TableRow
            className="cursor-pointer"
            onClick={() => navigate(`/accounts/${account.account_id}`)}>
            <TableCell>
                <Link
                    to={`/accounts/${account.account_id}`}
                    className="font-semibold hover:underline"
                    onClick={(event) => event.stopPropagation()}>
                    {account.account_name}
                </Link>
                <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground sm:hidden">
                    {ledger?.symbol}
                    {account.account_status !== "active" && (
                        <AccountStatusBadge status={account.account_status} />
                    )}
                </div>
            </TableCell>
            <TableCell className={ASSET_COLUMN}>
                {ledger ? (
                    <>
                        {ledger.symbol}{" "}
                        <span className="text-muted-foreground">{ledger.name}</span>
                    </>
                ) : (
                    <Skeleton className="h-4 w-16" />
                )}
            </TableCell>
            <TableCell className={STATUS_COLUMN}>
                <AccountStatusBadge status={account.account_status} />
            </TableCell>
            <TableCell className={NUMBER}>
                {units !== null && asset ? (
                    formatDecimal(fromUnits(units, asset.decimals))
                ) : (
                    <Skeleton className="ml-auto h-4 w-20" />
                )}
            </TableCell>
            {/* the dialog's events would bubble to the row and open the account */}
            <TableCell className={NUMBER} onClick={(event) => event.stopPropagation()}>
                {asset && (
                    <DepositDialog
                        account={account}
                        asset={asset}
                        trigger={
                            <Button
                                variant="outline"
                                size="xs"
                                disabled={account.account_status !== "active"}>
                                Deposit
                            </Button>
                        }
                    />
                )}
            </TableCell>
        </TableRow>
    );
}

// The acting user's accounts with what each can spend, grouped by asset, and a total per
// asset at the foot, under a double rule like the foot of a ledger page.
export function AccountsTable({ userID, empty }: { userID: string; empty: ReactNode }) {
    const { accounts, isPending } = useUserAccounts(userID);
    const ledgers = useQuery(ledgerQueries.list());
    const ledgerByID = new Map(ledgers.data?.map((ledger) => [ledger.ledger_id, ledger]));

    const sorted = [...accounts].sort((a, b) => {
        const symbolA = ledgerByID.get(a.account_ledger_id)?.symbol ?? "";
        const symbolB = ledgerByID.get(b.account_ledger_id)?.symbol ?? "";
        return (
            symbolA.localeCompare(symbolB) || a.account_name.localeCompare(b.account_name)
        );
    });

    // every account's balance, in the same order as `sorted` (see use-user-accounts.ts
    // for useQueries)
    const positions = useQueries({
        queries: sorted.map((account) => accountQueries.position(account.account_id)),
        combine: (results) => results.map((result) => result.data),
    });

    // totals per asset, in exact units; only once every balance of that asset is in
    const totals = new Map<number, { units: bigint; complete: boolean }>();
    sorted.forEach((account, index) => {
        const position = positions[index];
        const units = position ? availableUnits(position) : null;
        const total = totals.get(account.account_ledger_id) ?? {
            units: 0n,
            complete: true,
        };
        totals.set(account.account_ledger_id, {
            units: total.units + (units ?? 0n),
            complete: total.complete && units !== null,
        });
    });

    if (ledgers.isError) {
        return (
            <p role="alert" className="text-destructive">
                Couldn't load the assets: {errorMessage(ledgers.error)}
            </p>
        );
    }
    if (!isPending && sorted.length === 0) return empty;

    return (
        <div className="rounded-2xl border bg-card">
            <Table className="font-stretch-semi-condensed">
                <TableHeader>
                    <TableRow>
                        <TableHead>Account</TableHead>
                        <TableHead className={ASSET_COLUMN}>Asset</TableHead>
                        <TableHead className={STATUS_COLUMN}>Status</TableHead>
                        <TableHead className={NUMBER}>Available</TableHead>
                        <TableHead>
                            <span className="sr-only">Actions</span>
                        </TableHead>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {sorted.length === 0
                        ? [0, 1].map((row) => (
                              <TableRow key={row}>
                                  {["", ASSET_COLUMN, STATUS_COLUMN, "", ""].map(
                                      (column, cell) => (
                                          <TableCell key={cell} className={column}>
                                              <Skeleton className="h-4 w-full" />
                                          </TableCell>
                                      ),
                                  )}
                              </TableRow>
                          ))
                        : sorted.map((account, index) => (
                              <AccountRow
                                  key={account.account_id}
                                  account={account}
                                  ledger={ledgerByID.get(account.account_ledger_id)}
                                  position={positions[index]}
                              />
                          ))}
                </TableBody>
                {totals.size > 0 && (
                    <TableFooter className="border-t-[3px] border-double border-foreground/70 bg-transparent">
                        {[...totals].map(([ledgerID, total]) => {
                            const ledger = ledgerByID.get(ledgerID);
                            return (
                                <TableRow key={ledgerID} className="hover:bg-transparent">
                                    <TableCell className="text-muted-foreground">
                                        Total {ledger?.symbol}
                                    </TableCell>
                                    {/* a colSpan would count the columns phones hide */}
                                    <TableCell className={ASSET_COLUMN} />
                                    <TableCell className={STATUS_COLUMN} />
                                    <TableCell className={NUMBER}>
                                        {ledger && total.complete ? (
                                            formatDecimal(
                                                fromUnits(total.units, ledger.decimals),
                                            )
                                        ) : (
                                            <Skeleton className="ml-auto h-4 w-20" />
                                        )}
                                    </TableCell>
                                    <TableCell />
                                </TableRow>
                            );
                        })}
                    </TableFooter>
                )}
            </Table>
        </div>
    );
}
