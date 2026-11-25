import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { errorMessage } from "@/api";
import { accountQueries } from "@/api/queries";
import { TablePager } from "@/components/TablePager";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import { formatDecimal, formatLedgerWhen } from "@/lib/format";
import type { Transfer } from "@/types/Transfer";
import type { TransferQuery } from "@/types/TransferQuery";

const NUMBER = "text-right";

// The backend names each transfer by its ledger code (transfer_kind() in backend/src/v1.rs)
const KIND_LABELS: Record<string, string> = {
    deposit: "Deposit",
    trade: "Trade",
    fee: "Fee",
    hold: "Held for an order",
    refund: "Returned from an order",
};

function kindOf(transfer: Transfer): string {
    return KIND_LABELS[transfer.kind] ?? "Other";
}

// An account's transfers, newest first, as ledger lines: each amount sits in the Out
// (debit) or In (credit) column, like the balance above it.
export function TransfersTable({ accountID }: { accountID: string }) {
    const [query, setQuery] = useState<TransferQuery>({ limit: 20, cursor: null });
    const transfers = useQuery(accountQueries.transfers(accountID, query));

    return (
        <section
            aria-labelledby="transfers-heading"
            className="rounded-2xl border bg-card p-4">
            <h2 id="transfers-heading" className="font-semibold">
                Transfers
            </h2>
            {transfers.isError ? (
                <p role="alert" className="py-4 text-destructive">
                    Couldn't load the transfers: {errorMessage(transfers.error)}
                </p>
            ) : transfers.data?.items.length === 0 && query.cursor === null ? (
                <p className="py-4 text-muted-foreground">
                    No transfers yet. A deposit is the first one.
                </p>
            ) : (
                <div className="mt-2 flex flex-col gap-3">
                    <Table className="font-stretch-semi-condensed">
                        <TableHeader>
                            <TableRow>
                                <TableHead>Time</TableHead>
                                <TableHead>What</TableHead>
                                <TableHead className={NUMBER}>Out</TableHead>
                                <TableHead className={NUMBER}>In</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {transfers.data
                                ? transfers.data.items.map((transfer) => (
                                      <TableRow key={transfer.transfer_id}>
                                          <TableCell>
                                              {formatLedgerWhen(transfer.timestamp)}
                                          </TableCell>
                                          <TableCell>{kindOf(transfer)}</TableCell>
                                          <TableCell className={NUMBER}>
                                              {transfer.direction === "out" &&
                                                  formatDecimal(transfer.amount)}
                                          </TableCell>
                                          <TableCell className={NUMBER}>
                                              {transfer.direction === "in" &&
                                                  formatDecimal(transfer.amount)}
                                          </TableCell>
                                      </TableRow>
                                  ))
                                : [0, 1, 2].map((row) => (
                                      <TableRow key={row}>
                                          {[0, 1, 2, 3].map((cell) => (
                                              <TableCell key={cell}>
                                                  <Skeleton className="h-4 w-full" />
                                              </TableCell>
                                          ))}
                                      </TableRow>
                                  ))}
                        </TableBody>
                    </Table>
                    <TablePager
                        query={query}
                        setQuery={setQuery}
                        nextCursor={transfers.data?.next_cursor}
                        isPlaceholderData={transfers.isPlaceholderData}
                    />
                </div>
            )}
        </section>
    );
}
