import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { EllipsisIcon } from "lucide-react";
import { toast } from "sonner";
import { api, errorMessage } from "@/api";
import type { ErrorOverrides } from "@/api/client";
import {
    AlertDialog,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { Account } from "@/types/Account";
import { DepositDialog } from "./DepositDialog";

type Change = "close" | "reopen";

// Not found in the ledger: TigerBeetle doesn't have the account (yet)
const NOT_IN_LEDGER =
    "The ledger doesn't have this account yet. If it was just opened, try again in a moment.";

const COPY: Record<
    Change,
    {
        title: string;
        description: (name: string) => string;
        confirm: string;
        pending: string;
        done: string;
        errors: ErrorOverrides;
    }
> = {
    close: {
        title: "Close this account?",
        description: (name) =>
            `Closing ${name} cancels its open orders first. A closed account can't receive deposits or trade until it's reopened.`,
        confirm: "Close account",
        pending: "Closing...",
        done: "Account closed",
        errors: {
            403: "Only trading accounts can be closed.",
            404: NOT_IN_LEDGER,
            502: "The account wasn't closed: its open orders couldn't be cancelled, or the ledger didn't answer. Try again in a moment.",
        },
    },
    reopen: {
        title: "Reopen this account?",
        description: (name) =>
            `${name} can receive deposits and trade again. If it isn't closed, nothing changes.`,
        confirm: "Reopen account",
        pending: "Reopening...",
        done: "Account reopened",
        errors: {
            403: "Only trading accounts can be reopened.",
            404: NOT_IN_LEDGER,
            502: "The ledger didn't answer. Try again in a moment.",
        },
    },
};

// Deposit, plus closing and reopening behind a menu, each confirmed first. The API doesn't
// say yet whether an account is closed (REDESIGN.md, open questions), so both are offered;
// both are safe to repeat.
export function AccountActions({
    account,
    asset,
}: {
    account: Account;
    asset: { symbol: string; decimals: number } | undefined;
}) {
    // `change` stays set while the dialog animates closed, so its text doesn't jump
    const [dialog, setDialog] = useState<{ open: boolean; change: Change }>({
        open: false,
        change: "close",
    });
    const copy = COPY[dialog.change];
    const id = account.account_id;

    const queryClient = useQueryClient();
    const mutation = useMutation({
        mutationFn: (change: Change) =>
            change === "close" ? api.accounts.close(id) : api.accounts.reopen(id),
        onSuccess: (_answer, change) => {
            toast.success(COPY[change].done, { description: account.account_name });
            setDialog((prev) => ({ ...prev, open: false }));
            // closing cancels the account's open orders, so order lists change too
            return Promise.all([
                queryClient.invalidateQueries({ queryKey: ["position", id] }),
                queryClient.invalidateQueries({ queryKey: ["transfers", "account", id] }),
                queryClient.invalidateQueries({ queryKey: ["users"] }),
                queryClient.invalidateQueries({ queryKey: ["markets"] }),
            ]);
        },
    });

    function openDialog(change: Change) {
        mutation.reset();
        setDialog({ open: true, change });
    }

    return (
        <>
            {asset && (
                <DepositDialog
                    account={account}
                    asset={asset}
                    trigger={
                        <Button disabled={account.account_status !== "active"}>
                            Deposit
                        </Button>
                    }
                />
            )}
            {/* modal={false}: a modal menu that opens a dialog as it closes can leave the
                page unclickable (a known Radix issue) */}
            <DropdownMenu modal={false}>
                <DropdownMenuTrigger asChild>
                    <Button
                        variant="outline"
                        size="icon"
                        aria-label="More account actions">
                        <EllipsisIcon />
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                    <DropdownMenuItem onSelect={() => openDialog("close")}>
                        Close account
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => openDialog("reopen")}>
                        Reopen account
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
            {/* New: an alert dialog is a dialog that asks for a decision; it doesn't close
                when you click outside it */}
            <AlertDialog
                open={dialog.open}
                onOpenChange={(open) => setDialog((prev) => ({ ...prev, open }))}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{copy.title}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {copy.description(account.account_name)}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    {mutation.isError && (
                        <p role="alert" className="text-sm text-destructive">
                            {errorMessage(mutation.error, copy.errors)}
                        </p>
                    )}
                    <AlertDialogFooter>
                        <AlertDialogCancel>Not now</AlertDialogCancel>
                        {/* a plain button, not AlertDialogAction: that one closes the dialog
                            right away, and this one stays open until the answer (or error) */}
                        <Button
                            variant={
                                dialog.change === "close" ? "destructive" : "default"
                            }
                            disabled={mutation.isPending}
                            onClick={() => mutation.mutate(dialog.change)}>
                            {mutation.isPending ? copy.pending : copy.confirm}
                        </Button>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    );
}
