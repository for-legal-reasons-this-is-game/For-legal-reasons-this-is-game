import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, errorMessage } from "../api";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { uuidv7 } from "uuidv7";
import { OrderInfo } from "./OrderLookup";
import type { OrderPayload } from "../types/OrderPayload";
import type { OrderType } from "../types/OrderType";
import type { Side } from "../types/Side";
import type { TimeInForce } from "../types/TimeInForce";
import type { TriggerDirection } from "../types/TriggerDirection";

// What the form holds while the user types. OrderPayload's quantity and price fields
// depend on the order type (each is either a value or null), so the payload is only
// built on submit, from these fields.
type OrderForm = {
    market: string;
    base_account_id: string;
    quote_account_id: string;
    side: Side;
    order_type: OrderType;
    time_in_force: TimeInForce;
    quantity: string;
    limit_price: string;
    trigger_price: string;
    trigger_direction: TriggerDirection;
    protection_price: string;
    expire_time: string;
};

const EMPTY_FORM: OrderForm = {
    market: "",
    base_account_id: "",
    quote_account_id: "",
    side: "buy",
    order_type: "limit",
    time_in_force: "gtc",
    quantity: "",
    limit_price: "",
    trigger_price: "",
    trigger_direction: "last_at_or_above",
    protection_price: "",
    expire_time: "",
};

const SIDES: Side[] = ["buy", "sell"];
const ORDER_TYPES: OrderType[] = ["limit", "market", "stop_limit", "stop_market"];
const TIMES_IN_FORCE: TimeInForce[] = ["gtc", "gtd", "ioc", "fok"];
const TRIGGER_DIRECTIONS: TriggerDirection[] = ["last_at_or_above", "last_at_or_below"];

export function CreateOrder({ userID }: { userID: string }) {
    const [formData, setFormData] = useState<OrderForm>(EMPTY_FORM);
    const [validationError, setValidationError] = useState<string | null>(null);

    // The backend uses the idempotency key to recognise a repeated request: the same
    // key with the same order returns the order already placed instead of a second one.
    // So the key changes when the form changes (a different order) and after the
    // backend answers (that order is done, even if rejected: a rejected order is saved
    // under its key). It's kept only when a request got no answer, so that clicking
    // again can't place the order twice.
    // New React: useState(() => uuidv7()) passes a function, which React calls only on
    // the first render. useState(uuidv7()) would also work, but would generate (and
    // throw away) a new key on every render.
    const [idempotencyKey, setIdempotencyKey] = useState(() => uuidv7());

    // Same key as ListMarkets: one shared, cached markets list.
    const markets = useQuery({
        queryKey: ["markets"],
        queryFn: () => api.markets.list(),
    });

    const queryClient = useQueryClient();
    const { isPending, isError, isSuccess, data, error, mutate } = useMutation({
        mutationFn: (payload: OrderPayload) => api.users.createOrder(userID, payload),
        onSuccess: (_order, payload) => {
            setIdempotencyKey(uuidv7());
            // placing an order moves funds into its hold account (positions and
            // transfers change) and adds it to the user's and the market's orders
            return Promise.all([
                queryClient.invalidateQueries({ queryKey: ["users", userID, "orders"] }),
                queryClient.invalidateQueries({ queryKey: ["markets", payload.market] }),
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

    // These are recalculated on every render from the form, so they need no state.
    const market = markets.data?.find((m) => m.symbol === formData.market);
    const base = market?.base_asset.symbol ?? "base";
    const quote = market?.quote_asset.symbol ?? "quote";
    const hasLimit =
        formData.order_type === "limit" || formData.order_type === "stop_limit";
    const isStop =
        formData.order_type === "stop_limit" || formData.order_type === "stop_market";
    // market and stop-market buys say how much quote to spend; every other order says
    // how much base to buy or sell (the backend's validate_shape)
    const quoteSized = formData.side === "buy" && !hasLimit;

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (
            !formData.market ||
            !formData.base_account_id.trim() ||
            !formData.quote_account_id.trim() ||
            !formData.quantity.trim()
        ) {
            setValidationError(
                "The market, both account IDs and the quantity are required",
            );
            return;
        }
        if (hasLimit && !formData.limit_price.trim()) {
            setValidationError("Limit and stop-limit orders need a limit price");
            return;
        }
        if (isStop && !formData.trigger_price.trim()) {
            setValidationError("Stop orders need a trigger price");
            return;
        }
        if (formData.time_in_force === "gtd" && !formData.expire_time) {
            setValidationError("Good-till-date orders need an expiry time");
            return;
        }

        mutate({
            idempotency_key: idempotencyKey,
            market: formData.market,
            base_account_id: formData.base_account_id.trim(),
            quote_account_id: formData.quote_account_id.trim(),
            side: formData.side,
            order_type: formData.order_type,
            time_in_force: formData.time_in_force,
            base_quantity: quoteSized ? null : formData.quantity.trim(),
            quote_quantity: quoteSized ? formData.quantity.trim() : null,
            limit_price: hasLimit ? formData.limit_price.trim() : null,
            trigger_price: isStop ? formData.trigger_price.trim() : null,
            trigger_direction: isStop ? formData.trigger_direction : null,
            // optional, and only allowed on market and stop-market orders
            protection_price:
                !hasLimit && formData.protection_price.trim()
                    ? formData.protection_price.trim()
                    : null,
            // datetime-local gives local time without a timezone ("2026-11-20T15:30");
            // toISOString() converts it to UTC ("2026-11-20T14:30:00.000Z")
            expire_time:
                formData.time_in_force === "gtd"
                    ? new Date(formData.expire_time).toISOString()
                    : null,
        });
    }

    // One handler for the inputs and the dropdowns (see CreateMarket)
    function handleChange(event: ChangeEvent<HTMLInputElement | HTMLSelectElement>) {
        const { name, value } = event.target;
        setValidationError(null);
        // different form contents = a different order, which needs its own key
        setIdempotencyKey(uuidv7());
        setFormData((prev) => ({ ...prev, [name]: value }));
    }

    if (!userID) {
        return (
            <div style={{ color: "red" }}>
                No user selected. Click "Select User" in List Users or User Lookup first
            </div>
        );
    }

    return (
        <>
            <h3>Create Order for user __{userID}__</h3>
            {/* Amounts and prices are text inputs with inputMode="decimal", for the
                exact digits (explained in CreateMarket). */}
            <form onSubmit={handleSubmit}>
                <div>
                    <label>
                        Market{" "}
                        <select
                            name="market"
                            value={formData.market}
                            onChange={handleChange}>
                            <option value="">choose a market</option>
                            {markets.data?.map((m) => (
                                <option key={m.market_id} value={m.symbol}>
                                    {m.symbol} ({m.status})
                                </option>
                            ))}
                        </select>
                    </label>
                </div>
                {market && (
                    <div>
                        Rules: quantity at least {market.min_base_quantity} {base} in
                        steps of {market.quantity_step}, order value at least{" "}
                        {market.min_quote_quantity} {quote}, prices in steps of{" "}
                        {market.price_step}
                    </div>
                )}
                {/* There's no endpoint yet that lists a user's accounts, so the account
                    IDs have to be typed in (they're shown when an account is created). */}
                <div>
                    <input
                        name="base_account_id"
                        value={formData.base_account_id}
                        onChange={handleChange}
                        placeholder={`Base account ID (${base})`}
                    />
                    <input
                        name="quote_account_id"
                        value={formData.quote_account_id}
                        onChange={handleChange}
                        placeholder={`Quote account ID (${quote})`}
                    />
                </div>
                <div>
                    <select name="side" value={formData.side} onChange={handleChange}>
                        {SIDES.map((side) => (
                            <option key={side} value={side}>
                                {side}
                            </option>
                        ))}
                    </select>
                    <select
                        name="order_type"
                        value={formData.order_type}
                        onChange={handleChange}>
                        {ORDER_TYPES.map((type) => (
                            <option key={type} value={type}>
                                {type}
                            </option>
                        ))}
                    </select>
                    <select
                        name="time_in_force"
                        value={formData.time_in_force}
                        onChange={handleChange}>
                        {TIMES_IN_FORCE.map((tif) => (
                            <option key={tif} value={tif}>
                                {tif}
                            </option>
                        ))}
                    </select>
                </div>
                <div>
                    <input
                        name="quantity"
                        inputMode="decimal"
                        value={formData.quantity}
                        onChange={handleChange}
                        placeholder={
                            quoteSized
                                ? `Amount to spend (${quote})`
                                : `Quantity (${base})`
                        }
                    />
                </div>
                {hasLimit && (
                    <div>
                        <input
                            name="limit_price"
                            inputMode="decimal"
                            value={formData.limit_price}
                            onChange={handleChange}
                            placeholder={`Limit price (${quote} per ${base})`}
                        />
                    </div>
                )}
                {isStop && (
                    <div>
                        <input
                            name="trigger_price"
                            inputMode="decimal"
                            value={formData.trigger_price}
                            onChange={handleChange}
                            placeholder={`Trigger price (${quote} per ${base})`}
                        />
                        <select
                            name="trigger_direction"
                            value={formData.trigger_direction}
                            onChange={handleChange}>
                            {TRIGGER_DIRECTIONS.map((direction) => (
                                <option key={direction} value={direction}>
                                    {direction}
                                </option>
                            ))}
                        </select>
                    </div>
                )}
                {!hasLimit && (
                    <div>
                        <input
                            name="protection_price"
                            inputMode="decimal"
                            value={formData.protection_price}
                            onChange={handleChange}
                            placeholder="Protection price (optional)"
                        />
                    </div>
                )}
                {formData.time_in_force === "gtd" && (
                    <div>
                        <label>
                            Expires at{" "}
                            <input
                                name="expire_time"
                                type="datetime-local"
                                value={formData.expire_time}
                                onChange={handleChange}
                            />
                        </label>
                    </div>
                )}
                <button disabled={isPending} type="submit">
                    Place Order
                </button>
            </form>

            {markets.isError && (
                <div style={{ color: "red" }}>
                    Couldn't load the markets: {errorMessage(markets.error)}
                </div>
            )}
            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {/* The backend answers 400 both for an invalid order and when the engine
                doesn't support the order type yet (UNSUPPORTED), with no body to tell
                them apart. In November 2026 the engine only accepts GTC limit orders. */}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, {
                        400: "The order isn't valid for this market (check the quantity's minimum and step, the price step, and the fields this order type needs), or the trading engine doesn't support this order type yet",
                        403: "Both accounts must be user accounts that belong to this user",
                        404: "The user, the market or one of the accounts doesn't exist",
                        409: "Order rejected: not enough funds, an account isn't active, or the market isn't trading",
                        503: "The trading engine rejected the order",
                    })}
                </div>
            )}
            {isPending && <div>Placing order...</div>}
            {isSuccess && (
                <>
                    <div style={{ color: "green" }}>
                        {data.submission === "accepted"
                            ? "Order placed"
                            : "Order saved: waiting for the trading engine to confirm it"}
                    </div>
                    {/* The status and fills shown here come from the engine's answer.
                        The stored order gets them from the event consumer, so an Order
                        Lookup straight away can still show no status for a moment. */}
                    <OrderInfo order={data} />
                </>
            )}
        </>
    );
}
