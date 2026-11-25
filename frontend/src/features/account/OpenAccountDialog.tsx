import { useId, useRef, useState, type FormEvent, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api, errorMessage } from "@/api";
import { accountQueries, ledgerQueries } from "@/api/queries";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import type { AccountPayload } from "@/types/AccountPayload";
import { rememberAccount } from "./known-accounts";

// Opens an account for the acting user in one asset, and remembers it in this browser
// (there's no endpoint that lists a user's accounts yet). `asset`: a ledger symbol to start
// with, e.g. "BTC" when the order form is missing a BTC account.
export function OpenAccountDialog({
    userID,
    asset,
    trigger,
}: {
    userID: string;
    asset?: string;
    trigger: ReactNode;
}) {
    const [open, setOpen] = useState(false);
    const [form, setForm] = useState<AccountPayload>({
        name: "",
        ledger_symbol: asset ?? "",
    });
    const [problem, setProblem] = useState<string | null>(null);
    const id = useId();
    const nameInput = useRef<HTMLInputElement>(null);

    const ledgers = useQuery({ ...ledgerQueries.list(), enabled: open });
    // only enabled ledgers take new accounts (the backend answers 400 otherwise)
    const assets = ledgers.data?.filter((ledger) => ledger.enabled) ?? [];

    const queryClient = useQueryClient();
    const openAccount = useMutation({
        mutationFn: (payload: AccountPayload) => api.users.createAccount(userID, payload),
        onSuccess: (account) => {
            rememberAccount(userID, account.account_id);
            // put the answer in the cache, so lists show the account without fetching it;
            // while it's "processing", the detail query keeps asking until it's active
            queryClient.setQueryData(
                accountQueries.detail(account.account_id).queryKey,
                account,
            );
            toast.success("Account opened", {
                description:
                    account.account_status === "active"
                        ? account.account_name
                        : `${account.account_name} is being set up and is ready in a moment.`,
            });
            handleOpenChange(false);
        },
    });

    function handleOpenChange(next: boolean) {
        setOpen(next);
        if (!next) {
            // start fresh the next time it opens
            setForm({ name: "", ledger_symbol: asset ?? "" });
            setProblem(null);
            openAccount.reset();
        }
    }

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        // the dialog can sit inside the order form; keep its submit to itself (see
        // AddAccountDialog)
        event.stopPropagation();
        const name = form.name.trim();
        if (!name) return setProblem("Give the account a name.");
        if (!form.ledger_symbol) return setProblem("Choose an asset.");
        openAccount.mutate({ name, ledger_symbol: form.ledger_symbol });
    }

    const message =
        problem ??
        (openAccount.isError
            ? errorMessage(openAccount.error, {
                  400: "This asset is switched off. An admin can switch it on under Ledgers.",
                  404: "The user or the asset doesn't exist.",
              })
            : null);

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogTrigger asChild>{trigger}</DialogTrigger>
            <DialogContent
                // A dialog focuses its first field when it opens. When the asset is
                // already chosen, start on the name instead.
                onOpenAutoFocus={(event) => {
                    if (!asset) return;
                    event.preventDefault();
                    nameInput.current?.focus();
                }}>
                <form onSubmit={handleSubmit} noValidate className="grid gap-4">
                    <DialogHeader>
                        <DialogTitle>Open an account</DialogTitle>
                        <DialogDescription>
                            An account holds one asset. Deposit play money into it, then
                            trade with it.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="grid gap-2">
                        <Label htmlFor={`${id}-asset`}>Asset</Label>
                        <Select
                            value={form.ledger_symbol}
                            onValueChange={(symbol) => {
                                setProblem(null);
                                setForm((prev) => ({ ...prev, ledger_symbol: symbol }));
                            }}>
                            <SelectTrigger id={`${id}-asset`} className="w-full">
                                <SelectValue
                                    placeholder={
                                        ledgers.isPending
                                            ? "Loading assets..."
                                            : "Choose an asset"
                                    }
                                />
                            </SelectTrigger>
                            <SelectContent>
                                {assets.map((ledger) => (
                                    <SelectItem
                                        key={ledger.ledger_id}
                                        value={ledger.symbol}>
                                        {ledger.name} ({ledger.symbol})
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </div>
                    <div className="grid gap-2">
                        <Label htmlFor={`${id}-name`}>Name</Label>
                        <Input
                            id={`${id}-name`}
                            ref={nameInput}
                            value={form.name}
                            onChange={(event) => {
                                setProblem(null);
                                setForm((prev) => ({
                                    ...prev,
                                    name: event.target.value,
                                }));
                            }}
                            placeholder={`For example: Trading ${form.ledger_symbol || "BTC"}`}
                            autoComplete="off"
                            aria-invalid={!!message}
                            aria-describedby={message ? `${id}-problem` : undefined}
                        />
                    </div>
                    {message && (
                        <p id={`${id}-problem`} role="alert" className="text-destructive">
                            {message}
                        </p>
                    )}
                    {ledgers.isError && (
                        <p role="alert" className="text-destructive">
                            Couldn't load the assets: {errorMessage(ledgers.error)}
                        </p>
                    )}
                    <DialogFooter>
                        <Button type="submit" disabled={openAccount.isPending}>
                            {openAccount.isPending ? "Opening..." : "Open account"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}
