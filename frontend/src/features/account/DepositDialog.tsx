import { useId, useState, type FormEvent, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { uuidv7 } from "uuidv7";
import { api, errorMessage } from "@/api";
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
import { toUnits } from "@/lib/decimal";
import { formatDecimal } from "@/lib/format";
import type { Account } from "@/types/Account";
import type { DepositPayload } from "@/types/DepositPayload";

// Adds play money to an account, from the asset's source account in the ledger.
// `asset`: the account's ledger (its symbol and decimals).
export function DepositDialog({
    account,
    asset,
    trigger,
}: {
    account: Account;
    asset: { symbol: string; decimals: number };
    trigger: ReactNode;
}) {
    const [open, setOpen] = useState(false);
    const [amount, setAmount] = useState("");
    const [problem, setProblem] = useState<string | null>(null);
    // The idempotency key, as in the order form: the backend answers a repeated key with
    // the deposit already made instead of a second one. A new key when the amount changes
    // and after an answer; kept only when a request got no answer, so clicking again can't
    // deposit twice.
    const [idempotencyKey, setIdempotencyKey] = useState(() => uuidv7());
    const id = useId();

    const queryClient = useQueryClient();
    const deposit = useMutation({
        mutationFn: (payload: DepositPayload) =>
            api.accounts.deposit(account.account_id, payload),
        onSuccess: (transfer) => {
            setIdempotencyKey(uuidv7());
            toast.success(`Deposited ${formatDecimal(transfer.amount)} ${asset.symbol}`, {
                description: account.account_name,
            });
            handleOpenChange(false);
            return Promise.all([
                queryClient.invalidateQueries({
                    queryKey: ["position", account.account_id],
                }),
                queryClient.invalidateQueries({
                    queryKey: ["transfers", "account", account.account_id],
                }),
            ]);
        },
        onError: (error) => {
            // no answer from the backend: it may or may not have made the deposit
            if (error.kind === "network" || error.status === 502 || error.status === 504)
                return;
            setIdempotencyKey(uuidv7());
        },
    });

    function handleOpenChange(next: boolean) {
        setOpen(next);
        if (!next) {
            setAmount("");
            setProblem(null);
            deposit.reset();
        }
    }

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        // the dialog can sit inside the order form (see AddAccountDialog)
        event.stopPropagation();
        const value = amount.trim();
        const units = toUnits(value, asset.decimals);
        if (!value) return setProblem("Enter an amount.");
        if (units === null)
            return setProblem(
                `Enter a number with up to ${asset.decimals} decimals, like ${asset.decimals > 2 ? "0.5" : "1000"}.`,
            );
        if (units === 0n) return setProblem("Enter an amount above 0.");
        deposit.mutate({ idempotency_key: idempotencyKey, amount: value });
    }

    const message =
        problem ??
        (deposit.isError
            ? errorMessage(deposit.error, {
                  400: `The amount isn't valid for ${asset.symbol}, or the asset is switched off.`,
                  403: "Only trading accounts can receive deposits.",
                  409: "This account can't receive deposits right now: it's closed or still being opened.",
                  503: "The asset isn't ready for deposits yet. Try again in a moment.",
              })
            : null);

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogTrigger asChild>{trigger}</DialogTrigger>
            <DialogContent>
                <form onSubmit={handleSubmit} noValidate className="grid gap-4">
                    <DialogHeader>
                        <DialogTitle>Deposit {asset.symbol}</DialogTitle>
                        <DialogDescription>
                            Into {account.account_name}. It's play money: deposit as much
                            as you like.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="grid gap-2">
                        <Label htmlFor={`${id}-amount`}>
                            Amount<span className="sr-only"> in {asset.symbol}</span>
                        </Label>
                        <div className="relative">
                            <Input
                                id={`${id}-amount`}
                                inputMode="decimal"
                                autoComplete="off"
                                value={amount}
                                onChange={(event) => {
                                    setAmount(event.target.value);
                                    setProblem(null);
                                    // a different amount is a different deposit
                                    setIdempotencyKey(uuidv7());
                                    if (deposit.isError) deposit.reset();
                                }}
                                className="pr-12 text-right"
                                aria-invalid={!!message}
                                aria-describedby={message ? `${id}-problem` : undefined}
                            />
                            <span
                                aria-hidden
                                className="pointer-events-none absolute inset-y-0 right-2.5 flex items-center text-muted-foreground">
                                {asset.symbol}
                            </span>
                        </div>
                    </div>
                    {message && (
                        <p id={`${id}-problem`} role="alert" className="text-destructive">
                            {message}
                        </p>
                    )}
                    <DialogFooter>
                        <Button type="submit" disabled={deposit.isPending}>
                            {deposit.isPending ? "Depositing..." : "Deposit"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}
