import { useId, useState, type FormEvent, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { api, errorMessage } from "@/api";
import { ledgerQueries, marketQueries } from "@/api/queries";
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
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { fromUnits, toUnits } from "@/lib/decimal";
import { formatBps } from "@/lib/format";
import type { Ledger } from "@/types/Ledger";
import type { MarketPayload } from "@/types/MarketPayload";

// Every field as typed; the numbers are parsed when the form is sent
type Form = {
    base: string;
    quote: string;
    symbol: string;
    price_decimals: string;
    min_base_quantity: string;
    min_quote_quantity: string;
    quantity_step: string;
    price_step: string;
    maker_fee_bps: string;
    taker_fee_bps: string;
};
type Errors = Partial<Record<keyof Form, string>>;

const EMPTY: Form = {
    base: "",
    quote: "",
    symbol: "",
    price_decimals: "",
    min_base_quantity: "",
    min_quote_quantity: "",
    quantity_step: "",
    price_step: "",
    maker_fee_bps: "10",
    taker_fee_bps: "20",
};

// The symbol is the market's address ("/markets/BTC-USD"): capitals, digits, dashes
const SYMBOL = /^[A-Z0-9]+(-[A-Z0-9]+)*$/;

function wholeNumber(value: string): number | null {
    return /^\d{1,6}$/.test(value.trim()) ? Number(value) : null;
}

// The price decimals the backend accepts for this pair: price + base - quote decimals
// must be from 0 to 38 (that's how many digits it scales prices by), within 0 to 18.
function priceDecimalsRange(base: Ledger, quote: Ledger): [number, number] {
    return [
        Math.max(0, quote.decimals - base.decimals),
        Math.min(18, 38 - base.decimals + quote.decimals),
    ];
}

// An amount above 0 that fits `decimals`, or a message saying what's wrong
function amountError(
    value: string,
    decimals: number,
    required: string,
): string | undefined {
    if (!value.trim()) return required;
    const units = toUnits(value, decimals);
    if (units === null || units === 0n)
        return `Enter an amount above 0, with at most ${decimals} decimals.`;
    return undefined;
}

// The same checks as the backend's create_market, so most mistakes show at the field
function validate(
    form: Form,
    base: Ledger | undefined,
    quote: Ledger | undefined,
): Errors {
    const errors: Errors = {};
    if (!base) errors.base = "Choose the asset that's traded.";
    if (!quote) errors.quote = "Choose the asset prices are in.";
    else if (base && base.ledger_id === quote.ledger_id)
        errors.quote = "Choose an asset other than the base.";

    if (!form.symbol) errors.symbol = "Enter a symbol, like BTC-USD.";
    else if (!SYMBOL.test(form.symbol))
        errors.symbol = "Use capital letters, digits and dashes, like BTC-USD.";

    const priceDecimals = wholeNumber(form.price_decimals);
    if (priceDecimals === null || priceDecimals > 18)
        errors.price_decimals = "Enter a whole number from 0 to 18.";
    else if (base && quote) {
        const [lowest, highest] = priceDecimalsRange(base, quote);
        if (priceDecimals < lowest || priceDecimals > highest)
            errors.price_decimals = `For ${base.symbol} in ${quote.symbol}, use ${lowest} to ${highest}.`;
    }

    if (base) {
        errors.min_base_quantity = amountError(
            form.min_base_quantity,
            base.decimals,
            "Enter the smallest amount an order can have.",
        );
        if (form.quantity_step.trim())
            errors.quantity_step = amountError(form.quantity_step, base.decimals, "");
    }
    if (quote)
        errors.min_quote_quantity = amountError(
            form.min_quote_quantity,
            quote.decimals,
            "Enter the smallest value an order can have.",
        );
    if (priceDecimals !== null && form.price_step.trim())
        errors.price_step = amountError(form.price_step, priceDecimals, "");

    for (const fee of ["maker_fee_bps", "taker_fee_bps"] as const) {
        const bps = wholeNumber(form[fee]);
        if (bps === null || bps > 10_000)
            errors[fee] = "Enter whole basis points from 0 to 10,000.";
    }
    return errors;
}

// Creates a market from two enabled ledgers. The backend asks the trading engine to set it
// up: it's "trading" if the engine confirmed it in time, otherwise "halted" until the
// backend's relay gets the engine's answer.
export function CreateMarketDialog({ trigger }: { trigger: ReactNode }) {
    const [open, setOpen] = useState(false);
    const [form, setForm] = useState<Form>(EMPTY);
    const [errors, setErrors] = useState<Errors>({});
    // the symbol and price decimals follow the chosen assets until someone types in them
    const [typed, setTyped] = useState({ symbol: false, price_decimals: false });
    const id = useId();
    const navigate = useNavigate();

    const ledgers = useQuery({ ...ledgerQueries.list(), enabled: open });
    // only enabled ledgers can be used (the backend answers 400 otherwise)
    const assets = (ledgers.data ?? [])
        .filter((ledger) => ledger.enabled)
        .sort((a, b) => a.symbol.localeCompare(b.symbol));
    const base = assets.find((ledger) => ledger.symbol === form.base);
    const quote = assets.find((ledger) => ledger.symbol === form.quote);
    const priceDecimals = wholeNumber(form.price_decimals);

    const queryClient = useQueryClient();
    const createMarket = useMutation({
        mutationFn: (payload: MarketPayload) => api.markets.create(payload),
        onSuccess: (market) => {
            queryClient.setQueryData(marketQueries.list().queryKey, (list) =>
                list ? [...list, market] : list,
            );
            queryClient.invalidateQueries({
                queryKey: marketQueries.list().queryKey,
                exact: true,
            });
            if (market.status === "trading") {
                toast.success("Market created", {
                    description: `${market.symbol} is open for trading.`,
                    action: {
                        label: `Open ${market.symbol}`,
                        onClick: () => navigate(`/markets/${market.symbol}`),
                    },
                });
            } else {
                toast.success("Market created", {
                    description: `${market.symbol} opens for trading once the trading engine confirms it. The exchange keeps asking.`,
                });
            }
            handleOpenChange(false);
        },
    });

    function handleOpenChange(next: boolean) {
        setOpen(next);
        if (!next) {
            setForm(EMPTY);
            setErrors({});
            setTyped({ symbol: false, price_decimals: false });
            createMarket.reset();
        }
    }

    function change(field: keyof Form, value: string) {
        setForm((prev) => ({ ...prev, [field]: value }));
        setErrors((prev) => ({ ...prev, [field]: undefined }));
        // the backend's answer was about the old values
        if (createMarket.isError) createMarket.reset();
    }

    // Choosing an asset also fills in what follows from it, unless it was typed
    function chooseAsset(field: "base" | "quote", symbol: string) {
        if (createMarket.isError) createMarket.reset();
        const next = { ...form, [field]: symbol };
        if (!typed.symbol && next.base && next.quote)
            next.symbol = `${next.base}-${next.quote}`;
        const chosenQuote = assets.find((ledger) => ledger.symbol === next.quote);
        if (field === "quote" && chosenQuote && !typed.price_decimals)
            next.price_decimals = String(chosenQuote.decimals);
        setForm(next);
        setErrors((prev) => ({ ...prev, [field]: undefined, symbol: undefined }));
    }

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        event.stopPropagation();
        const found = validate(form, base, quote);
        setErrors(found);
        if (Object.values(found).some(Boolean) || !base || !quote) return;
        createMarket.mutate({
            symbol: form.symbol,
            base_ledger_symbol: base.symbol,
            quote_ledger_symbol: quote.symbol,
            price_decimals: Number(form.price_decimals),
            min_base_quantity: form.min_base_quantity.trim(),
            min_quote_quantity: form.min_quote_quantity.trim(),
            // left empty, the backend uses the smallest unit
            quantity_step: form.quantity_step.trim() || null,
            price_step: form.price_step.trim() || null,
            maker_fee_bps: Number(form.maker_fee_bps),
            taker_fee_bps: Number(form.taker_fee_bps),
        });
    }

    // a fee's percentage under its field, once it's a valid number
    const feeHint = (field: "maker_fee_bps" | "taker_fee_bps", who: string) => {
        const bps = wholeNumber(form[field]);
        return bps !== null && bps <= 10_000
            ? `${formatBps(bps)} of each trade's value, paid by ${who}.`
            : `Paid by ${who}. 1 basis point is 0.01%.`;
    };

    const assetSelect = (field: "base" | "quote", label: string, hint: string) => (
        <div className="grid gap-1.5">
            <Label htmlFor={`${id}-${field}`}>{label}</Label>
            <Select
                value={form[field]}
                onValueChange={(symbol) => chooseAsset(field, symbol)}>
                <SelectTrigger
                    id={`${id}-${field}`}
                    className="w-full"
                    aria-invalid={!!errors[field]}
                    aria-describedby={`${id}-${field}-note`}>
                    <SelectValue
                        placeholder={ledgers.isPending ? "Loading assets..." : "Choose"}
                    />
                </SelectTrigger>
                <SelectContent>
                    {assets.map((ledger) => (
                        <SelectItem key={ledger.ledger_id} value={ledger.symbol}>
                            {ledger.name} ({ledger.symbol})
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            <p
                id={`${id}-${field}-note`}
                className={
                    errors[field]
                        ? "text-xs text-destructive"
                        : "text-xs text-muted-foreground"
                }>
                {errors[field] ?? hint}
            </p>
        </div>
    );

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogTrigger asChild>{trigger}</DialogTrigger>
            {/* a long form: on short screens the dialog scrolls */}
            <DialogContent className="max-h-[calc(100svh-2rem)] overflow-y-auto sm:max-w-2xl">
                <form onSubmit={handleSubmit} noValidate className="grid gap-5">
                    <DialogHeader>
                        <DialogTitle>Create a market</DialogTitle>
                        <DialogDescription>
                            A market trades one asset (the base) for another (the quote,
                            which prices it). Orders can be placed once the trading engine
                            has confirmed it.
                        </DialogDescription>
                    </DialogHeader>

                    <div className="grid gap-4 sm:grid-cols-2">
                        {assetSelect(
                            "base",
                            "Base asset",
                            "What's bought and sold, like BTC.",
                        )}
                        {assetSelect(
                            "quote",
                            "Quote asset",
                            "What prices are in, like USD.",
                        )}
                        <div className="grid gap-1.5">
                            <Label htmlFor={`${id}-symbol`}>Symbol</Label>
                            <Input
                                id={`${id}-symbol`}
                                value={form.symbol}
                                onChange={(event) => {
                                    const value = event.target.value.toUpperCase();
                                    // emptied: follow the assets again
                                    setTyped((prev) => ({
                                        ...prev,
                                        symbol: value !== "",
                                    }));
                                    change("symbol", value);
                                }}
                                placeholder="BTC-USD"
                                autoComplete="off"
                                autoCapitalize="characters"
                                spellCheck={false}
                                maxLength={25}
                                aria-invalid={!!errors.symbol}
                                aria-describedby={`${id}-symbol-note`}
                            />
                            <p
                                id={`${id}-symbol-note`}
                                className={
                                    errors.symbol
                                        ? "text-xs text-destructive"
                                        : "text-xs text-muted-foreground"
                                }>
                                {errors.symbol ?? "Its name in lists and in its address."}
                            </p>
                        </div>
                        <DecimalField
                            id={`${id}-price-decimals`}
                            label="Price decimals"
                            inputMode="numeric"
                            value={form.price_decimals}
                            onChange={(value) => {
                                setTyped((prev) => ({
                                    ...prev,
                                    price_decimals: value !== "",
                                }));
                                change("price_decimals", value);
                            }}
                            error={errors.price_decimals}
                            hint={
                                base && quote
                                    ? `Decimals a price can have. For ${base.symbol} in ${quote.symbol}: ${priceDecimalsRange(base, quote).join(" to ")}.`
                                    : "Decimals a price can have."
                            }
                        />
                    </div>

                    <fieldset className="grid gap-4 sm:grid-cols-2">
                        <legend className="mb-3 font-medium">Order rules</legend>
                        <DecimalField
                            id={`${id}-min-base`}
                            label="Smallest amount"
                            unit={base?.symbol}
                            value={form.min_base_quantity}
                            onChange={(value) => change("min_base_quantity", value)}
                            error={errors.min_base_quantity}
                            hint="An order can't be for less."
                        />
                        <DecimalField
                            id={`${id}-min-quote`}
                            label="Smallest value"
                            unit={quote?.symbol}
                            value={form.min_quote_quantity}
                            onChange={(value) => change("min_quote_quantity", value)}
                            error={errors.min_quote_quantity}
                            hint="An order can't be worth less."
                        />
                        <DecimalField
                            id={`${id}-quantity-step`}
                            label="Amount step (optional)"
                            unit={base?.symbol}
                            value={form.quantity_step}
                            onChange={(value) => change("quantity_step", value)}
                            error={errors.quantity_step}
                            placeholder={base ? fromUnits(1n, base.decimals) : undefined}
                            hint="Amounts must be multiples of it. Empty: the smallest unit."
                        />
                        <DecimalField
                            id={`${id}-price-step`}
                            label="Price step (optional)"
                            unit={quote?.symbol}
                            value={form.price_step}
                            onChange={(value) => change("price_step", value)}
                            error={errors.price_step}
                            placeholder={
                                priceDecimals !== null && priceDecimals <= 18
                                    ? fromUnits(1n, priceDecimals)
                                    : undefined
                            }
                            hint="Prices must be multiples of it. Empty: the smallest unit."
                        />
                    </fieldset>

                    <fieldset className="grid gap-4 sm:grid-cols-2">
                        <legend className="mb-3 font-medium">Fees</legend>
                        <DecimalField
                            id={`${id}-maker-fee`}
                            label="Maker fee"
                            unit="bps"
                            inputMode="numeric"
                            value={form.maker_fee_bps}
                            onChange={(value) => change("maker_fee_bps", value)}
                            error={errors.maker_fee_bps}
                            hint={feeHint("maker_fee_bps", "the order that was waiting")}
                        />
                        <DecimalField
                            id={`${id}-taker-fee`}
                            label="Taker fee"
                            unit="bps"
                            inputMode="numeric"
                            value={form.taker_fee_bps}
                            onChange={(value) => change("taker_fee_bps", value)}
                            error={errors.taker_fee_bps}
                            hint={feeHint(
                                "taker_fee_bps",
                                "the order that traded on arrival",
                            )}
                        />
                    </fieldset>

                    {ledgers.isError && (
                        <p role="alert" className="text-destructive">
                            Couldn't load the assets: {errorMessage(ledgers.error)}
                        </p>
                    )}
                    {createMarket.isError && (
                        <p role="alert" className="text-destructive">
                            {errorMessage(createMarket.error, {
                                400: "The market wasn't created: the trading engine refused it, or an asset was just switched off.",
                                404: "One of the assets doesn't exist any more.",
                                409: `There's already a market called ${form.symbol}, or one for ${form.base} in ${form.quote}.`,
                            })}
                        </p>
                    )}
                    <DialogFooter>
                        <Button type="submit" disabled={createMarket.isPending}>
                            {createMarket.isPending ? "Creating..." : "Create market"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}
