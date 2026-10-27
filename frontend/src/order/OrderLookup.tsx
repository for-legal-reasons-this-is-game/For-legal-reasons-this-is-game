import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";
import { ListOrderTrades } from "./ListOrderTrades";
import type { Order } from "../types/Order";

// The *_price/quantity fields are null when they don't apply to the order type
// (e.g. a market order has no limit_price), so those lines only show when set.
export function OrderInfo({ order }: { order: Order }) {
    return (
        <>
            <h4>Order {order.order_id} </h4>
            <div>Market: {order.market} </div>
            <div>Side: {order.side} </div>
            <div>Type: {order.order_type} </div>
            <div>Time In Force: {order.time_in_force} </div>
            <div>Submission: {order.submission} </div>
            {/* status stays null until the engine has accepted the order */}
            <div>Status: {order.status ?? "-"} </div>
            {order.limit_price !== null && <div>Limit Price: {order.limit_price} </div>}
            {order.trigger_price !== null && (
                <div>
                    Trigger Price: {order.trigger_price} ({order.trigger_direction})
                </div>
            )}
            {order.protection_price !== null && (
                <div>Protection Price: {order.protection_price} </div>
            )}
            {order.base_quantity !== null && (
                <div>Base Quantity: {order.base_quantity} </div>
            )}
            {order.quote_quantity !== null && (
                <div>Quote Quantity: {order.quote_quantity} </div>
            )}
            <div>Filled Base Quantity: {order.filled_base_quantity} </div>
            <div>Filled Quote Quantity: {order.filled_quote_quantity} </div>
            <div>Reserved Amount: {order.reserved_amount} </div>
            <div>Maker Fee BPS: {order.maker_fee_bps} </div>
            <div>Taker Fee BPS: {order.taker_fee_bps} </div>
            {order.expire_time !== null && <div>Expires: {order.expire_time} </div>}
            {order.reject_reason !== null && (
                <div style={{ color: "red" }}>
                    Rejected: {order.reject_reason} {order.reject_detail}
                </div>
            )}
            {order.cancel_reason !== null && (
                <div>Cancel Reason: {order.cancel_reason} </div>
            )}
            <div>User ID: {order.user_id} </div>
            <div>Base Account ID: {order.base_account_id} </div>
            <div>Quote Account ID: {order.quote_account_id} </div>
            <div>Created: {order.created_at} </div>
            <div>Updated: {order.updated_at} </div>
        </>
    );
}

export function OrderLookup() {
    const [orderID, setOrderID] = useState<string>("");
    const [submittedOrderID, setSubmittedOrderID] = useState("");
    const [validationError, setValidationError] = useState<string | null>(null);
    const [orderTrades, setOrderTrades] = useState(false);

    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["orders", submittedOrderID],
        queryFn: () => api.orders.get(submittedOrderID),
        enabled: !!submittedOrderID,
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (!orderID.trim()) {
            setValidationError("OrderID field can't be empty");
            return;
        }
        setSubmittedOrderID(orderID.trim());
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setValidationError(null);
        setOrderID(event.target.value);
    }

    return (
        <>
            <h3>Order Lookup</h3>
            <form onSubmit={handleSubmit}>
                <input onChange={handleChange} value={orderID} placeholder="OrderID" />
                <button type="submit">Get Order</button>
            </form>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, {
                        400: "That's not a valid order ID",
                        404: "An order with this ID doesn't exist",
                    })}
                </div>
            )}
            {isFetching && <div>Loading...</div>}
            {isSuccess && (
                <>
                    <OrderInfo order={data} />
                    <button
                        style={{ background: orderTrades ? "green" : "grey" }}
                        onClick={() => setOrderTrades(!orderTrades)}>
                        Order Trades
                    </button>
                    {orderTrades && <ListOrderTrades orderID={data.order_id} />}
                </>
            )}
        </>
    );
}
