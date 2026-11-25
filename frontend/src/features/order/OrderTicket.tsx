import { useId, useState, type FormEvent, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { uuidv7 } from "uuidv7";
import { api, errorMessage } from "@/api";
import { accountQueries, marketQueries } from "@/api/queries";
import { useActingUser } from "@/app/acting-user/context";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { AddAccountDialog } from "@/features/account/AddAccountDialog";
import { availableUnits } from "@/features/account/balance";
import { DepositDialog } from "@/features/account/DepositDialog";
import { OpenAccountDialog } from "@/features/account/OpenAccountDialog";
import { useUserAccounts } from "@/features/account/use-user-accounts";
import { feeCap, fromUnits, quoteFor, toUnits } from "@/lib/decimal";
import { ENGINE_SUPPORTS } from "@/lib/engine-support";
import { formatDecimal, formatTrimmed } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { Account } from "@/types/Account";
import type { Market } from "@/types/Market";
import type { Order } from "@/types/Order";
import type { OrderPayload } from "@/types/OrderPayload";
import type { OrderType } from "@/types/OrderType";
import type { Side } from "@/types/Side";
import type { TimeInForce } from "@/types/TimeInForce";
import type { TriggerDirection } from "@/types/TriggerDirection";
import {
    CANCEL_REASON_LABELS,
    ORDER_TYPE_LABELS,
    SIDE_LABELS,
    TIME_IN_FORCE_LABELS,
    TRIGGER_DIRECTION_LABELS,
} from "./labels";

// The order form of the trade page, ported from order/CreateOrder (same payload rules
// and idempotency key handling), for one market and the acting user.

type TicketForm = {
    side: Side;
    order_type: OrderType;
    time_in_force: TimeInForce;
    quantity: string;
    limit_price: string;
    trigger_price: string;
    trigger_direction: TriggerDirection;
    protection_price: string;
    expire_time: string;
    // "" = the first active account on the right ledger (see `pick` below)
    base_account_id: string;
    quote_account_id: string;
};

type Field =
    | "quantity"
    | "limit_price"
    | "trigger_price"
    | "protection_price"
    | "expire_time"
    | "base_account_id"
    | "quote_account_id"
    | "value";

const ORDER_TYPES = Object.keys(ORDER_TYPE_LABELS) as OrderType[];
const TIMES_IN_FORCE = Object.keys(TIME_IN_FORCE_LABELS) as TimeInForce[];
const TRIGGER_DIRECTIONS = Object.keys(TRIGGER_DIRECTION_LABELS) as TriggerDirection[];

// Per-status messages for a rejected order, from CreateOrder. The backend answers 400 both
// for an invalid order and for an order type the engine doesn't support yet, with no body
// to tell them apart.
const ORDER_ERRORS = {
    400: "The order isn't valid for this market (check the amount's minimum and step, the price step, and the fields this order type needs), or the trading engine doesn't support this order type yet.",
    403: "Both accounts must be trading accounts that belong to you.",
    404: "The user, the market or one of the accounts doesn't exist.",
    409: "Order rejected: not enough funds, an account isn't active, or the market isn't trading.",
    503: "The trading engine rejected the order.",
};

// "0.2 BTC at 65,000.00 USD", for the toast
function describe(order: Order, market: Market): string {
    const base = market.base_asset.symbol;
    const quote = market.quote_asset.symbol;
    const amount = order.base_quantity
        ? `${formatTrimmed(order.base_quantity)} ${base}`
        : `${formatTrimmed(order.quote_quantity ?? "0")} ${quote} of ${base}`;
    return order.limit_price
        ? `${amount} at ${formatDecimal(order.limit_price)} ${quote}`
        : `${amount} at the market price`;
}

// The toast after placing an order. The reply carries the engine's immediate answer
// (status and fills after its first matching attempt), so the toast can say right away
// whether the order traded. Later changes arrive through the engine's events.
function placedToast(
    order: Order,
    market: Market,
): { title: string; description: string } {
    const what = `${SIDE_LABELS[order.side]} order`;
    const base = market.base_asset.symbol;
    const quote = market.quote_asset.symbol;
    // "0.05 BTC for 3,200.00 USD"
    const filled = `${formatTrimmed(order.filled_base_quantity)} ${base} for ${formatDecimal(order.filled_quote_quantity)} ${quote}`;
    switch (order.status) {
        case "filled":
            return { title: `${what} filled`, description: filled };
        case "partially_filled":
            return {
                title: `${what} partly filled`,
                description: `${filled} so far. The rest stays open.`,
            };
        case "cancelled":
            return {
                title: `${what} cancelled`,
                description: order.cancel_reason
                    ? CANCEL_REASON_LABELS[order.cancel_reason]
                    : describe(order, market),
            };
        default:
            // 201/200 with status "new": resting in the book. 202 (not accepted yet): the
            // backend saved it and keeps sending it to the engine.
            return {
                title: `${what} placed`,
                description:
                    order.submission === "accepted"
                        ? describe(order, market)
                        : `${describe(order, market)}. Waiting for the trading engine to confirm it.`,
            };
    }
}

// A text input for an amount or a price, with its unit inside the box on the right, a
// hint underneath, and the error in place of the hint when there is one.
function DecimalField({
    id,
    label,
    unit,
    value,
    onChange,
    hint,
    error,
    action,
}: {
    id: string;
    label: string;
    unit: string;
    value: string;
    onChange: (value: string) => void;
    hint?: ReactNode;
    error?: string;
    action?: ReactNode;
}) {
    const note = error ?? hint;
    return (
        <div className="grid gap-1.5">
            <div className="flex items-baseline justify-between gap-2">
                <Label htmlFor={id}>
                    {label}
                    <span className="sr-only"> in {unit}</span>
                </Label>
                {action}
            </div>
            <div className="relative">
                {/* text, not type="number": number inputs round long decimals and
                    accept "1e3"; inputMode="decimal" still opens the number keyboard */}
                <Input
                    id={id}
                    inputMode="decimal"
                    autoComplete="off"
                    value={value}
                    onChange={(event) => onChange(event.target.value)}
                    className="pr-12 text-right"
                    aria-invalid={!!error}
                    aria-describedby={note ? `${id}-note` : undefined}
                />
                <span
                    aria-hidden
                    className="pointer-events-none absolute inset-y-0 right-2.5 flex items-center text-muted-foreground">
                    {unit}
                </span>
            </div>
            {note && (
                <p
                    id={`${id}-note`}
                    className={cn(
                        "text-xs",
                        error ? "text-destructive" : "text-muted-foreground",
                    )}>
                    {note}
                </p>
            )}
        </div>
    );
}

// The accounts of one asset to trade with, or a way to get one when there's none.
function AccountField({
    id,
    userID,
    asset,
    accounts,
    isPending,
    value,
    onChange,
    note,
    error,
}: {
    id: string;
    userID: string;
    asset: string;
    accounts: Account[];
    isPending: boolean;
    value: string;
    onChange: (accountID: string) => void;
    note?: ReactNode;
    error?: string;
}) {
    const text = error ?? note;
    return (
        <div className="grid gap-1.5">
            <Label htmlFor={id}>{asset} account</Label>
            {isPending && accounts.length === 0 ? (
                <Skeleton className="h-8" />
            ) : accounts.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                    No {asset} account yet.{" "}
                    <OpenAccountDialog
                        userID={userID}
                        asset={asset}
                        trigger={
                            <button
                                type="button"
                                className="text-primary underline underline-offset-2">
                                Open one
                            </button>
                        }
                    />
                </p>
            ) : (
                <Select value={value} onValueChange={onChange}>
                    <SelectTrigger
                        id={id}
                        className="w-full"
                        aria-invalid={!!error}
                        aria-describedby={text ? `${id}-note` : undefined}>
                        <SelectValue placeholder={`Choose a ${asset} account`} />
                    </SelectTrigger>
                    <SelectContent>
                        {accounts.map((account) => (
                            // the backend only takes orders from active accounts; one that is
                            // still being opened becomes active within seconds
                            <SelectItem
                                key={account.account_id}
                                value={account.account_id}
                                disabled={account.account_status !== "active"}>
                                {account.account_name}
                                {account.account_status !== "active" && (
                                    <span className="text-muted-foreground">
                                        {" "}
                                        (being opened)
                                    </span>
                                )}
                            </SelectItem>
                        ))}
                    </SelectContent>
                </Select>
            )}
            {text && (
                <p
                    id={`${id}-note`}
                    className={cn(
                        "text-xs",
                        error ? "text-destructive" : "text-muted-foreground",
                    )}>
                    {text}
                </p>
            )}
        </div>
    );
}

// One line of the summary: label on the left, amount on the right
function SummaryRow({ label, children }: { label: ReactNode; children: ReactNode }) {
    return (
        <div className="flex items-baseline justify-between gap-4">
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="text-right">{children}</dd>
        </div>
    );
}

export function OrderTicket({
    market,
    initialSide = "buy",
    onPlaced,
}: {
    market: Market;
    initialSide?: Side;
    // called after an order is placed (the mobile sheet closes itself)
    onPlaced?: () => void;
}) {
    const { userID } = useActingUser();
    const [form, setForm] = useState<TicketForm>({
        side: initialSide,
        order_type: "limit",
        time_in_force: "gtc",
        quantity: "",
        limit_price: "",
        trigger_price: "",
        trigger_direction: "last_at_or_above",
        protection_price: "",
        expire_time: "",
        base_account_id: "",
        quote_account_id: "",
    });
    // When "Place order" was last clicked (null: not yet, or the order went through).
    // Errors show from then on and update as the user types.
    const [submittedAt, setSubmittedAt] = useState<number | null>(null);
    // The idempotency key, exactly as in CreateOrder: a new one when the form changes and
    // when the backend answers; kept only when a request got no answer, so that clicking
    // again can't place the order twice.
    const [idempotencyKey, setIdempotencyKey] = useState(() => uuidv7());
    // New React: useId gives an ID that's unique on the page and stable between renders.
    // The ticket can be on the page twice (the side column, and the sheet on small
    // screens), so fixed IDs like "price" would clash and break the labels.
    const id = useId();

    const ticker = useQuery(marketQueries.ticker(market.symbol));
    const { accounts, isPending: accountsPending } = useUserAccounts(userID);

    const base = market.base_asset;
    const quote = market.quote_asset;
    const hasLimit = form.order_type === "limit" || form.order_type === "stop_limit";
    const isStop = form.order_type === "stop_limit" || form.order_type === "stop_market";
    // market and stop-market buys say how much quote to spend; every other order says how
    // much base to buy or sell (the backend's validate_shape)
    const quoteSized = form.side === "buy" && !hasLimit;
    const amountAsset = quoteSized ? quote : base;

    // The chosen account, or else the first active one on the asset's ledger. A choice
    // that isn't in the list any more (another acting user was picked) doesn't count.
    function pick(chosen: string, ledgerID: number) {
        const options = accounts.filter((a) => a.account_ledger_id === ledgerID);
        const fallback = options.find((a) => a.account_status === "active");
        const valid = options.some((a) => a.account_id === chosen);
        return { options, id: valid ? chosen : (fallback?.account_id ?? "") };
    }
    const baseAccounts = pick(form.base_account_id, base.ledger_id);
    const quoteAccounts = pick(form.quote_account_id, quote.ledger_id);

    // What the order is worth and what it holds, in exact units (lib/decimal.ts),
    // calculated like the backend's reservation_amount()
    const amountUnits = toUnits(form.quantity, amountAsset.decimals);
    const priceUnits = hasLimit ? toUnits(form.limit_price, market.price_decimals) : null;
    const valueUnits = quoteSized
        ? amountUnits
        : priceUnits !== null && amountUnits !== null
          ? quoteFor(
                priceUnits,
                amountUnits,
                market.price_decimals,
                base.decimals,
                quote.decimals,
            )
          : null;
    const feeBps = Math.max(market.maker_fee_bps, market.taker_fee_bps);
    const feeUnits = valueUnits !== null ? feeCap(valueUnits, feeBps) : null;
    const heldUnits =
        form.side === "sell"
            ? amountUnits
            : valueUnits !== null && feeUnits !== null
              ? valueUnits + feeUnits
              : null;
    const heldAsset = form.side === "sell" ? base : quote;

    // The paying account's balance. Placing an order moves the held amount to the order
    // right away (a posted transfer), so the posted balance is what's free to use;
    // pending debits only appear while an account is being closed.
    const payingAccountID = form.side === "sell" ? baseAccounts.id : quoteAccounts.id;
    const payingAccount = (
        form.side === "sell" ? baseAccounts : quoteAccounts
    ).options.find((account) => account.account_id === payingAccountID);
    const position = useQuery(accountQueries.position(payingAccountID));
    const available = position.data ? availableUnits(position.data) : null;
    const tooMuch = heldUnits !== null && available !== null && heldUnits > available;

    // Checks the backend would fail with a 400, worded for the person typing. `now` is
    // the time of the click: the clock can't be read while rendering, because a render
    // must give the same result every time it runs (React's rule of pure components).
    function validate(now: number): Partial<Record<Field, string>> {
        const errors: Partial<Record<Field, string>> = {};

        function checkPrice(field: Field, value: string, name: string) {
            const units = toUnits(value, market.price_decimals);
            const step = toUnits(market.price_step, market.price_decimals);
            if (!value.trim()) errors[field] = `Enter a ${name}.`;
            else if (units === null)
                errors[field] =
                    `Enter a number with up to ${market.price_decimals} decimals, like 65000.50.`;
            else if (units === 0n) errors[field] = `Enter a ${name} above 0.`;
            else if (step && units % step !== 0n)
                errors[field] =
                    `Prices go in steps of ${formatTrimmed(market.price_step)}.`;
        }

        if (!form.quantity.trim()) errors.quantity = "Enter an amount.";
        else if (amountUnits === null)
            errors.quantity = `Enter a number with up to ${amountAsset.decimals} decimals, like 0.25.`;
        else if (amountUnits === 0n) errors.quantity = "Enter an amount above 0.";
        else if (!quoteSized) {
            const min = toUnits(market.min_base_quantity, base.decimals);
            const step = toUnits(market.quantity_step, base.decimals);
            if (min !== null && amountUnits < min)
                errors.quantity = `The smallest amount is ${formatTrimmed(market.min_base_quantity)} ${base.symbol}.`;
            else if (step && amountUnits % step !== 0n)
                errors.quantity = `Amounts go in steps of ${formatTrimmed(market.quantity_step)} ${base.symbol}.`;
        }

        if (hasLimit) checkPrice("limit_price", form.limit_price, "price");
        if (isStop) checkPrice("trigger_price", form.trigger_price, "trigger price");
        if (!hasLimit && form.protection_price.trim())
            checkPrice("protection_price", form.protection_price, "protection price");

        const minValue = toUnits(market.min_quote_quantity, quote.decimals);
        if (valueUnits !== null && minValue !== null && valueUnits < minValue)
            errors.value = `The order's value must be at least ${formatTrimmed(market.min_quote_quantity)} ${quote.symbol}.`;

        if (form.time_in_force === "gtd") {
            if (!form.expire_time) errors.expire_time = "Choose when the order expires.";
            else if (new Date(form.expire_time).getTime() <= now)
                errors.expire_time = "Choose a time in the future.";
        }

        if (!baseAccounts.id) errors.base_account_id = `Choose a ${base.symbol} account.`;
        if (!quoteAccounts.id)
            errors.quote_account_id = `Choose a ${quote.symbol} account.`;
        return errors;
    }
    const errors = submittedAt !== null ? validate(submittedAt) : {};

    const queryClient = useQueryClient();
    const navigate = useNavigate();
    const placeOrder = useMutation({
        mutationFn: (payload: OrderPayload) => api.users.createOrder(userID, payload),
        onSuccess: (order) => {
            setIdempotencyKey(uuidv7());
            setSubmittedAt(null);
            // a fresh amount for the next order, so a second click can't repeat this one
            setForm((prev) => ({ ...prev, quantity: "" }));
            const { title, description } = placedToast(order, market);
            toast.success(title, {
                description,
                action: {
                    label: "View order",
                    onClick: () => navigate(`/orders/${order.order_id}`),
                },
            });
            onPlaced?.();
            // placing an order moves funds into its hold account (positions and transfers
            // change) and adds it to the user's and the market's orders
            return Promise.all([
                queryClient.invalidateQueries({ queryKey: ["users", userID, "orders"] }),
                queryClient.invalidateQueries({ queryKey: ["markets", market.symbol] }),
                queryClient.invalidateQueries({ queryKey: ["position"] }),
                queryClient.invalidateQueries({ queryKey: ["transfers"] }),
            ]);
        },
        onError: (error) => {
            // no answer from the backend: it may or may not have saved the order
            if (error.kind === "network" || error.status === 502 || error.status === 504)
                return;
            setIdempotencyKey(uuidv7());
        },
    });

    // One change handler for every field
    // New TypeScript: <K extends keyof TicketForm> ties `value` to the field's own type,
    // so change("side", "sideways") doesn't compile.
    function change<K extends keyof TicketForm>(field: K, value: TicketForm[K]) {
        // different form contents = a different order, which needs its own key
        setIdempotencyKey(uuidv7());
        if (placeOrder.isError) placeOrder.reset();
        setForm((prev) => ({ ...prev, [field]: value }));
    }

    const trading = market.status === "trading";
    const canTrade = !!userID && trading;

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const now = Date.now();
        setSubmittedAt(now);
        if (Object.keys(validate(now)).length > 0) return;

        const quantity = form.quantity.trim();
        placeOrder.mutate({
            idempotency_key: idempotencyKey,
            market: market.symbol,
            base_account_id: baseAccounts.id,
            quote_account_id: quoteAccounts.id,
            side: form.side,
            order_type: form.order_type,
            time_in_force: form.time_in_force,
            base_quantity: quoteSized ? null : quantity,
            quote_quantity: quoteSized ? quantity : null,
            limit_price: hasLimit ? form.limit_price.trim() : null,
            trigger_price: isStop ? form.trigger_price.trim() : null,
            trigger_direction: isStop ? form.trigger_direction : null,
            // optional, and only allowed on market and stop-market orders
            protection_price:
                !hasLimit && form.protection_price.trim()
                    ? form.protection_price.trim()
                    : null,
            // datetime-local gives local time without a timezone; toISOString() makes it UTC
            expire_time:
                form.time_in_force === "gtd"
                    ? new Date(form.expire_time).toISOString()
                    : null,
        });
    }

    const buying = form.side === "buy";
    // A step is only worth a hint when it's coarser than the decimals already allow
    // (a price step of 0.01 with 2 decimals goes without saying; 0.50 doesn't)
    const priceStepHint =
        toUnits(market.price_step, market.price_decimals) !== 1n
            ? `Steps of ${formatTrimmed(market.price_step)} ${quote.symbol}`
            : undefined;
    const quantityStepHint =
        toUnits(market.quantity_step, base.decimals) !== 1n
            ? `, in steps of ${formatTrimmed(market.quantity_step)}`
            : "";
    const lastPrice = ticker.data?.last_price;
    const show = (units: bigint | null, asset: { decimals: number; symbol: string }) =>
        units === null ? (
            <span className="text-muted-foreground">-</span>
        ) : (
            `${formatDecimal(fromUnits(units, asset.decimals))} ${asset.symbol}`
        );
    // "1,000.00 USD", shown under the paying account
    // "1,000.00 USD", shown under the paying account, with a way to add more
    const availableText =
        available !== null
            ? `${formatDecimal(fromUnits(available, heldAsset.decimals))} ${heldAsset.symbol}`
            : undefined;
    const availableNote = availableText && payingAccount && (
        <>
            {availableText} available.{" "}
            <DepositDialog
                account={payingAccount}
                asset={heldAsset}
                trigger={
                    <button
                        type="button"
                        className="text-primary underline-offset-2 hover:underline">
                        Deposit
                    </button>
                }
            />
        </>
    );

    return (
        <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-4">
            <h2 className="sr-only">Place an order</h2>
            {!userID && (
                <p className="rounded-lg bg-muted px-3 py-2">
                    Choose who you're trading as in the top bar to place orders.
                </p>
            )}
            {userID && !trading && (
                <p className="rounded-lg bg-muted px-3 py-2">
                    {market.status === "halted"
                        ? `Trading in ${market.symbol} is halted. New orders aren't accepted.`
                        : `${market.symbol} is in cancel-only mode. New orders aren't accepted.`}
                </p>
            )}

            {/* A disabled fieldset disables every control inside it: while an order is
                being placed, and when this user can't trade here */}
            <fieldset
                disabled={!canTrade || placeOrder.isPending}
                className="flex min-w-0 flex-col gap-4">
                <ToggleGroup
                    type="single"
                    variant="outline"
                    spacing={0}
                    value={form.side}
                    onValueChange={(side) => side && change("side", side as Side)}
                    aria-label="Side"
                    className="grid w-full grid-cols-2">
                    <ToggleGroupItem
                        value="buy"
                        className="h-9 data-[state=on]:border-gain data-[state=on]:bg-gain data-[state=on]:text-gain-foreground">
                        Buy
                    </ToggleGroupItem>
                    <ToggleGroupItem
                        value="sell"
                        className="h-9 data-[state=on]:border-loss data-[state=on]:bg-loss data-[state=on]:text-loss-foreground">
                        Sell
                    </ToggleGroupItem>
                </ToggleGroup>

                {/* 2:3, so "Good till cancelled" fits; minmax(0, ...) lets a longer
                    choice shorten with "..." instead of pushing out of the column */}
                <div className="grid grid-cols-[minmax(0,2fr)_minmax(0,3fr)] gap-3">
                    <div className="grid gap-1.5">
                        <Label htmlFor={`${id}-type`}>Type</Label>
                        <Select
                            value={form.order_type}
                            onValueChange={(type) =>
                                change("order_type", type as OrderType)
                            }>
                            <SelectTrigger id={`${id}-type`} size="sm" className="w-full">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {ORDER_TYPES.map((type) => {
                                    const supported =
                                        ENGINE_SUPPORTS.orderTypes.includes(type);
                                    return (
                                        <SelectItem
                                            key={type}
                                            value={type}
                                            disabled={!supported}>
                                            {ORDER_TYPE_LABELS[type]}
                                            {!supported && (
                                                <span className="text-xs text-muted-foreground">
                                                    not available yet
                                                </span>
                                            )}
                                        </SelectItem>
                                    );
                                })}
                            </SelectContent>
                        </Select>
                    </div>
                    <div className="grid gap-1.5">
                        <Label htmlFor={`${id}-tif`}>Time in force</Label>
                        <Select
                            value={form.time_in_force}
                            onValueChange={(tif) =>
                                change("time_in_force", tif as TimeInForce)
                            }>
                            <SelectTrigger id={`${id}-tif`} size="sm" className="w-full">
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {TIMES_IN_FORCE.map((tif) => {
                                    const supported =
                                        ENGINE_SUPPORTS.timeInForce.includes(tif);
                                    return (
                                        <SelectItem
                                            key={tif}
                                            value={tif}
                                            disabled={!supported}>
                                            {TIME_IN_FORCE_LABELS[tif]}
                                            {!supported && (
                                                <span className="text-xs text-muted-foreground">
                                                    not available yet
                                                </span>
                                            )}
                                        </SelectItem>
                                    );
                                })}
                            </SelectContent>
                        </Select>
                    </div>
                </div>

                {hasLimit && (
                    <DecimalField
                        id={`${id}-price`}
                        label="Price"
                        unit={quote.symbol}
                        value={form.limit_price}
                        onChange={(value) => change("limit_price", value)}
                        hint={priceStepHint}
                        error={errors.limit_price}
                        action={
                            lastPrice && (
                                <button
                                    type="button"
                                    className="text-xs text-primary underline-offset-2 hover:underline disabled:text-muted-foreground"
                                    onClick={() => change("limit_price", lastPrice)}>
                                    Use last price
                                </button>
                            )
                        }
                    />
                )}
                {isStop && (
                    <>
                        <DecimalField
                            id={`${id}-trigger`}
                            label="Trigger price"
                            unit={quote.symbol}
                            value={form.trigger_price}
                            onChange={(value) => change("trigger_price", value)}
                            error={errors.trigger_price}
                        />
                        <div className="grid gap-1.5">
                            <Label htmlFor={`${id}-direction`}>Triggers when the</Label>
                            <Select
                                value={form.trigger_direction}
                                onValueChange={(direction) =>
                                    change(
                                        "trigger_direction",
                                        direction as TriggerDirection,
                                    )
                                }>
                                <SelectTrigger id={`${id}-direction`} className="w-full">
                                    <SelectValue />
                                </SelectTrigger>
                                <SelectContent>
                                    {TRIGGER_DIRECTIONS.map((direction) => (
                                        <SelectItem key={direction} value={direction}>
                                            {TRIGGER_DIRECTION_LABELS[direction]}
                                        </SelectItem>
                                    ))}
                                </SelectContent>
                            </Select>
                        </div>
                    </>
                )}
                <DecimalField
                    id={`${id}-amount`}
                    label={quoteSized ? "Spend" : "Amount"}
                    unit={amountAsset.symbol}
                    value={form.quantity}
                    onChange={(value) => change("quantity", value)}
                    hint={
                        quoteSized
                            ? `At least ${formatTrimmed(market.min_quote_quantity)} ${quote.symbol}`
                            : `At least ${formatTrimmed(market.min_base_quantity)} ${base.symbol}${quantityStepHint}`
                    }
                    error={errors.quantity}
                />
                {!hasLimit && (
                    <DecimalField
                        id={`${id}-protection`}
                        label="Protection price (optional)"
                        unit={quote.symbol}
                        value={form.protection_price}
                        onChange={(value) => change("protection_price", value)}
                        hint="The worst price this order may fill at"
                        error={errors.protection_price}
                    />
                )}
                {form.time_in_force === "gtd" && (
                    <div className="grid gap-1.5">
                        <Label htmlFor={`${id}-expires`}>Expires at</Label>
                        <Input
                            id={`${id}-expires`}
                            type="datetime-local"
                            value={form.expire_time}
                            onChange={(event) =>
                                change("expire_time", event.target.value)
                            }
                            aria-invalid={!!errors.expire_time}
                            aria-describedby={
                                errors.expire_time ? `${id}-expires-note` : undefined
                            }
                        />
                        {errors.expire_time && (
                            <p
                                id={`${id}-expires-note`}
                                className="text-xs text-destructive">
                                {errors.expire_time}
                            </p>
                        )}
                    </div>
                )}

                {userID && (
                    <div className="grid gap-3">
                        <AccountField
                            id={`${id}-base-account`}
                            userID={userID}
                            asset={base.symbol}
                            accounts={baseAccounts.options}
                            isPending={accountsPending}
                            value={baseAccounts.id}
                            onChange={(accountID) => change("base_account_id", accountID)}
                            note={buying ? undefined : availableNote}
                            error={errors.base_account_id}
                        />
                        <AccountField
                            id={`${id}-quote-account`}
                            userID={userID}
                            asset={quote.symbol}
                            accounts={quoteAccounts.options}
                            isPending={accountsPending}
                            value={quoteAccounts.id}
                            onChange={(accountID) =>
                                change("quote_account_id", accountID)
                            }
                            note={buying ? availableNote : undefined}
                            error={errors.quote_account_id}
                        />
                        <AddAccountDialog
                            userID={userID}
                            trigger={
                                <button
                                    type="button"
                                    className="justify-self-start text-xs text-primary underline-offset-2 hover:underline">
                                    Add an account by ID
                                </button>
                            }
                        />
                    </div>
                )}

                {/* The summary, set like the foot of a ledger page: the amounts, a double
                    rule, then the total */}
                <dl className="grid gap-1.5 text-sm">
                    <SummaryRow label="Value">
                        {quoteSized || hasLimit ? (
                            show(valueUnits, quote)
                        ) : (
                            <span className="text-muted-foreground">
                                At the market price
                            </span>
                        )}
                    </SummaryRow>
                    <SummaryRow
                        label={`Fee, up to ${formatTrimmed((feeBps / 100).toFixed(2))}%`}>
                        {show(feeUnits, quote)}
                    </SummaryRow>
                    <div className="border-t-[3px] border-double border-foreground/70 pt-1.5">
                        <SummaryRow label="Held for this order">
                            <span className="font-semibold">
                                {show(heldUnits, heldAsset)}
                            </span>
                        </SummaryRow>
                    </div>
                </dl>
                {errors.value && (
                    <p className="text-xs text-destructive">{errors.value}</p>
                )}
                {tooMuch && !errors.value && (
                    <p className="text-xs text-destructive">
                        That's more than the {availableText} available.
                    </p>
                )}

                <Button
                    type="submit"
                    size="lg"
                    className={cn(
                        "w-full",
                        buying
                            ? "bg-gain text-gain-foreground hover:bg-gain/85"
                            : "bg-loss text-loss-foreground hover:bg-loss/85",
                    )}>
                    {placeOrder.isPending
                        ? `Placing ${form.side} order...`
                        : `Place ${form.side} order`}
                </Button>
            </fieldset>

            {placeOrder.isError && (
                <p role="alert" className="text-sm text-destructive">
                    {errorMessage(placeOrder.error, ORDER_ERRORS)}
                </p>
            )}
        </form>
    );
}
