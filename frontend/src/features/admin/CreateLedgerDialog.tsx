import { useId, useState, type FormEvent, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api, errorMessage } from "@/api";
import { ledgerQueries } from "@/api/queries";
import { DecimalField } from "@/components/DecimalField";
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
import { fromUnits } from "@/lib/decimal";
import type { LedgerPayload } from "@/types/LedgerPayload";

type Form = { symbol: string; name: string; decimals: string };
type Errors = Partial<Record<keyof Form, string>>;

const EMPTY: Form = { symbol: "", name: "", decimals: "2" };

// Symbols end up in market symbols and addresses ("/markets/BTC-USD"), so they're kept
// to capital letters and digits. The backend itself takes any text.
const SYMBOL = /^[A-Z0-9]{1,12}$/;

function validate(form: Form): Errors {
    const errors: Errors = {};
    if (!form.symbol) errors.symbol = "Enter a symbol, like BTC.";
    else if (!SYMBOL.test(form.symbol))
        errors.symbol = "Use up to 12 capital letters and digits, like BTC.";
    if (!form.name.trim()) errors.name = "Enter a name, like Bitcoin.";
    const decimals = Number(form.decimals);
    if (!/^\d+$/.test(form.decimals) || decimals > 18)
        errors.decimals = "Enter a whole number from 0 to 18.";
    return errors;
}

// Creates a ledger: an asset the exchange keeps accounts in. New ledgers start switched on.
export function CreateLedgerDialog({ trigger }: { trigger: ReactNode }) {
    const [open, setOpen] = useState(false);
    const [form, setForm] = useState<Form>(EMPTY);
    const [errors, setErrors] = useState<Errors>({});
    const id = useId();

    const queryClient = useQueryClient();
    const createLedger = useMutation({
        mutationFn: (payload: LedgerPayload) => api.ledgers.create(payload),
        onSuccess: (ledger) => {
            queryClient.setQueryData(ledgerQueries.list().queryKey, (list) =>
                list ? [...list, ledger] : list,
            );
            queryClient.invalidateQueries({ queryKey: ledgerQueries.list().queryKey });
            toast.success("Ledger created", {
                description: `${ledger.name} (${ledger.symbol}). Users can open ${ledger.symbol} accounts now.`,
            });
            handleOpenChange(false);
        },
    });

    function handleOpenChange(next: boolean) {
        setOpen(next);
        if (!next) {
            setForm(EMPTY);
            setErrors({});
            createLedger.reset();
        }
    }

    function change(field: keyof Form, value: string) {
        setForm((prev) => ({ ...prev, [field]: value }));
        setErrors((prev) => ({ ...prev, [field]: undefined }));
        // the backend's answer was about the old values
        if (createLedger.isError) createLedger.reset();
    }

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        event.stopPropagation();
        const found = validate(form);
        setErrors(found);
        if (Object.values(found).some(Boolean)) return;
        createLedger.mutate({
            symbol: form.symbol,
            name: form.name.trim(),
            decimals: Number(form.decimals),
        });
    }

    const decimals = Number(form.decimals);
    const smallest =
        /^\d+$/.test(form.decimals) && decimals <= 18 ? fromUnits(1n, decimals) : null;

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogTrigger asChild>{trigger}</DialogTrigger>
            <DialogContent>
                <form onSubmit={handleSubmit} noValidate className="grid gap-4">
                    <DialogHeader>
                        <DialogTitle>Create a ledger</DialogTitle>
                        <DialogDescription>
                            A ledger is an asset, like US dollars or bitcoin. Users open
                            accounts in it, and markets pair two of them.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="grid grid-cols-[minmax(0,2fr)_minmax(0,3fr)] gap-3">
                        <div className="grid gap-1.5">
                            <Label htmlFor={`${id}-symbol`}>Symbol</Label>
                            <Input
                                id={`${id}-symbol`}
                                value={form.symbol}
                                // symbols are capitals; typing "btc" gives "BTC"
                                onChange={(event) =>
                                    change("symbol", event.target.value.toUpperCase())
                                }
                                placeholder="ETH"
                                autoComplete="off"
                                autoCapitalize="characters"
                                spellCheck={false}
                                maxLength={12}
                                aria-invalid={!!errors.symbol}
                                aria-describedby={
                                    errors.symbol ? `${id}-symbol-note` : undefined
                                }
                            />
                            {errors.symbol && (
                                <p
                                    id={`${id}-symbol-note`}
                                    className="text-xs text-destructive">
                                    {errors.symbol}
                                </p>
                            )}
                        </div>
                        <div className="grid gap-1.5">
                            <Label htmlFor={`${id}-name`}>Name</Label>
                            <Input
                                id={`${id}-name`}
                                value={form.name}
                                onChange={(event) => change("name", event.target.value)}
                                placeholder="Ether"
                                autoComplete="off"
                                aria-invalid={!!errors.name}
                                aria-describedby={
                                    errors.name ? `${id}-name-note` : undefined
                                }
                            />
                            {errors.name && (
                                <p
                                    id={`${id}-name-note`}
                                    className="text-xs text-destructive">
                                    {errors.name}
                                </p>
                            )}
                        </div>
                    </div>
                    <DecimalField
                        id={`${id}-decimals`}
                        label="Decimals"
                        inputMode="numeric"
                        value={form.decimals}
                        onChange={(value) => change("decimals", value)}
                        error={errors.decimals}
                        hint={
                            <>
                                How many decimal places amounts can have: 2 for US
                                dollars, 8 for bitcoin.
                                {smallest && ` The smallest amount is then ${smallest}.`}
                            </>
                        }
                    />
                    {createLedger.isError && (
                        <p role="alert" className="text-destructive">
                            {errorMessage(createLedger.error, {
                                409: `There's already a ledger with the symbol ${form.symbol}.`,
                                400: "Decimals must be from 0 to 18.",
                            })}
                        </p>
                    )}
                    <DialogFooter>
                        <Button type="submit" disabled={createLedger.isPending}>
                            {createLedger.isPending ? "Creating..." : "Create ledger"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}
