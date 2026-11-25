import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api, errorMessage } from "@/api";
import { ledgerQueries } from "@/api/queries";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import type { Ledger } from "@/types/Ledger";

const NUMBER = "text-right";
// the TigerBeetle number is for the team; phones skip it
const NUMBER_COLUMN = "hidden sm:table-cell";

function LedgerRow({ ledger }: { ledger: Ledger }) {
    const queryClient = useQueryClient();
    const setEnabled = useMutation({
        mutationFn: (enabled: boolean) =>
            api.ledgers.setEnabled(ledger.symbol, { enabled }),
        onSuccess: (updated) => {
            queryClient.setQueryData(ledgerQueries.list().queryKey, (list) =>
                list?.map((l) => (l.ledger_id === updated.ledger_id ? updated : l)),
            );
            // markets carry their two ledgers, including whether each is on
            queryClient.invalidateQueries({ queryKey: ["markets"] });
            toast.success(
                `${updated.symbol} switched ${updated.enabled ? "on" : "off"}`,
                {
                    description: updated.enabled
                        ? `${updated.name} takes new accounts, deposits, orders and markets again.`
                        : `No new accounts, deposits, orders or markets in ${updated.name} until it's switched on again.`,
                },
            );
        },
        onError: (error, enabled) =>
            toast.error(`Couldn't switch ${ledger.symbol} ${enabled ? "on" : "off"}`, {
                description: errorMessage(error, {
                    404: "This ledger doesn't exist any more.",
                }),
            }),
    });

    // New TanStack: while a mutation runs, `variables` holds what was sent, so the switch
    // can show its new position at once instead of waiting for the answer
    const on = setEnabled.isPending ? setEnabled.variables : ledger.enabled;

    return (
        <TableRow>
            <TableCell>
                <span className="font-semibold">{ledger.symbol}</span>{" "}
                <span className="text-muted-foreground">{ledger.name}</span>
            </TableCell>
            <TableCell className={NUMBER}>{ledger.decimals}</TableCell>
            <TableCell className={`${NUMBER} ${NUMBER_COLUMN}`}>
                {ledger.ledger_id}
            </TableCell>
            <TableCell>
                <div className="flex items-center gap-2">
                    <Switch
                        checked={on}
                        disabled={setEnabled.isPending}
                        onCheckedChange={(checked) => setEnabled.mutate(checked)}
                        // read as "BTC ledger, switch, on"
                        aria-label={`${ledger.symbol} ledger`}
                    />
                    {/* the switch says the same to screen readers */}
                    <span aria-hidden className={on ? "" : "text-muted-foreground"}>
                        {on ? "On" : "Off"}
                    </span>
                </div>
            </TableCell>
        </TableRow>
    );
}

// Every ledger by symbol, with a switch to turn each on or off.
export function LedgersTable() {
    const ledgers = useQuery(ledgerQueries.list());

    if (ledgers.isError) {
        return (
            <p role="alert" className="p-2 text-destructive">
                Couldn't load the ledgers: {errorMessage(ledgers.error)}
            </p>
        );
    }
    const sorted = [...(ledgers.data ?? [])].sort((a, b) =>
        a.symbol.localeCompare(b.symbol),
    );
    // the backend starts with USD, EUR and BTC, so only a hand-made database gets here
    if (ledgers.isSuccess && sorted.length === 0) {
        return (
            <p className="p-2 text-muted-foreground">
                No ledgers yet. Create one for each asset users can hold, like USD.
            </p>
        );
    }

    return (
        <Table className="font-stretch-semi-condensed">
            <TableHeader>
                <TableRow>
                    <TableHead>Asset</TableHead>
                    <TableHead className={NUMBER}>Decimals</TableHead>
                    <TableHead className={`${NUMBER} ${NUMBER_COLUMN}`}>
                        TigerBeetle ledger
                    </TableHead>
                    <TableHead>Switched on</TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {ledgers.isPending
                    ? [0, 1, 2].map((row) => (
                          <TableRow key={row}>
                              {["", NUMBER, NUMBER_COLUMN, ""].map((className, cell) => (
                                  <TableCell key={cell} className={className}>
                                      <Skeleton className="h-4 w-full" />
                                  </TableCell>
                              ))}
                          </TableRow>
                      ))
                    : sorted.map((ledger) => (
                          <LedgerRow key={ledger.ledger_id} ledger={ledger} />
                      ))}
            </TableBody>
        </Table>
    );
}
