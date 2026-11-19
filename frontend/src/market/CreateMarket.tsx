import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, errorMessage } from "../api";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { MarketInfo } from "./MarketLookup";
import type { MarketPayload } from "../types/MarketPayload";

// The fields stored as numbers; every other field is a string.
const NUMBER_FIELDS = ["price_decimals", "maker_fee_bps", "taker_fee_bps"];

export function CreateMarket() {
    const [formData, setFormData] = useState<MarketPayload>({
        symbol: "",
        base_ledger_symbol: "",
        quote_ledger_symbol: "",
        price_decimals: 2,
        min_base_quantity: "",
        min_quote_quantity: "",
        quantity_step: "",
        price_step: "",
        maker_fee_bps: 10,
        taker_fee_bps: 20,
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    // Same key as ListLedgers, so both share one cached ledgers list.
    const ledgers = useQuery({
        queryKey: ["ledgers"],
        queryFn: () => api.ledgers.list(),
    });

    const queryClient = useQueryClient();
    const { isPending, isError, isSuccess, data, error, mutate } = useMutation({
        mutationFn: (payload: MarketPayload) => api.markets.create(payload),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ["markets"] }),
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const { symbol, base_ledger_symbol, quote_ledger_symbol, price_decimals } =
            formData;

        if (
            !symbol.trim() ||
            !base_ledger_symbol ||
            !quote_ledger_symbol ||
            !formData.min_base_quantity.trim() ||
            !formData.min_quote_quantity.trim()
        ) {
            setValidationError(
                "Symbol, both ledgers and both minimum quantities are required",
            );
            return;
        }
        if (base_ledger_symbol === quote_ledger_symbol) {
            setValidationError("The base and quote ledgers must be different");
            return;
        }
        if (
            !Number.isInteger(price_decimals) ||
            price_decimals < 0 ||
            price_decimals > 18
        ) {
            setValidationError("Price decimals must be a whole number from 0 to 18");
            return;
        }
        for (const fee of [formData.maker_fee_bps, formData.taker_fee_bps]) {
            if (!Number.isInteger(fee) || fee < 0 || fee > 10000) {
                setValidationError(
                    "Fees are whole basis points from 0 to 10000 (1 bps = 0.01%)",
                );
                return;
            }
        }
        mutate({
            ...formData,
            symbol: symbol.trim(),
            min_base_quantity: formData.min_base_quantity.trim(),
            min_quote_quantity: formData.min_quote_quantity.trim(),
            // an empty step isn't sent: the backend then uses the smallest unit
            quantity_step: formData.quantity_step?.trim() || null,
            price_step: formData.price_step?.trim() || null,
        });
    }

    // One handler for the inputs and the dropdowns: `A | B` means "either type",
    // and both have the `name` and `value` used here.
    function handleChange(event: ChangeEvent<HTMLInputElement | HTMLSelectElement>) {
        const { name, value } = event.target;
        setValidationError(null);
        setFormData((prev) => ({
            ...prev,
            [name]: NUMBER_FIELDS.includes(name) ? Number(value) : value,
        }));
    }

    // the backend refuses disabled ledgers, so they aren't offered
    const enabledLedgers = ledgers.data?.filter((ledger) => ledger.enabled);

    return (
        <>
            <h3>Create Market</h3>
            {/* New: the quantity fields are text inputs with inputMode="decimal"
                rather than type="number". They keep the exact digits typed (a number
                input accepts "1e5" and its value goes through floating point), and
                phones still show a number keyboard. */}
            <form onSubmit={handleSubmit}>
                <div>
                    <input
                        name="symbol"
                        value={formData.symbol}
                        onChange={handleChange}
                        placeholder="Symbol, e.g. BTC-USD"
                    />
                </div>
                <div>
                    <label>
                        Base ledger{" "}
                        <select
                            name="base_ledger_symbol"
                            value={formData.base_ledger_symbol}
                            onChange={handleChange}>
                            <option value="">choose</option>
                            {enabledLedgers?.map((ledger) => (
                                <option key={ledger.ledger_id} value={ledger.symbol}>
                                    {ledger.symbol} ({ledger.name})
                                </option>
                            ))}
                        </select>
                    </label>{" "}
                    <label>
                        Quote ledger{" "}
                        <select
                            name="quote_ledger_symbol"
                            value={formData.quote_ledger_symbol}
                            onChange={handleChange}>
                            <option value="">choose</option>
                            {enabledLedgers?.map((ledger) => (
                                <option key={ledger.ledger_id} value={ledger.symbol}>
                                    {ledger.symbol} ({ledger.name})
                                </option>
                            ))}
                        </select>
                    </label>
                </div>
                <div>
                    <label>
                        Price decimals{" "}
                        <input
                            name="price_decimals"
                            type="number"
                            value={formData.price_decimals}
                            onChange={handleChange}
                        />
                    </label>
                </div>
                <div>
                    <input
                        name="min_base_quantity"
                        inputMode="decimal"
                        value={formData.min_base_quantity}
                        onChange={handleChange}
                        placeholder="Min base quantity"
                    />
                    <input
                        name="min_quote_quantity"
                        inputMode="decimal"
                        value={formData.min_quote_quantity}
                        onChange={handleChange}
                        placeholder="Min quote quantity"
                    />
                </div>
                <div>
                    <input
                        name="quantity_step"
                        inputMode="decimal"
                        value={formData.quantity_step ?? ""}
                        onChange={handleChange}
                        placeholder="Quantity step (optional)"
                    />
                    <input
                        name="price_step"
                        inputMode="decimal"
                        value={formData.price_step ?? ""}
                        onChange={handleChange}
                        placeholder="Price step (optional)"
                    />
                </div>
                <div>
                    <label>
                        Maker fee (bps){" "}
                        <input
                            name="maker_fee_bps"
                            type="number"
                            value={formData.maker_fee_bps}
                            onChange={handleChange}
                        />
                    </label>{" "}
                    <label>
                        Taker fee (bps){" "}
                        <input
                            name="taker_fee_bps"
                            type="number"
                            value={formData.taker_fee_bps}
                            onChange={handleChange}
                        />
                    </label>
                </div>
                <button disabled={isPending} type="submit">
                    Create Market
                </button>
            </form>

            {ledgers.isError && (
                <div style={{ color: "red" }}>
                    Couldn't load the ledgers: {errorMessage(ledgers.error)}
                </div>
            )}
            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, {
                        400: "Invalid market: the minimums and steps must fit the ledgers' decimals, and both ledgers must be enabled. The trading engine may also have refused it",
                        404: "One of the ledgers doesn't exist",
                        409: "A market with this symbol, or for these two ledgers, already exists",
                    })}
                </div>
            )}
            {isPending && <div>Creating market...</div>}
            {isSuccess && (
                <>
                    <div style={{ color: "green" }}>Created market {data.symbol}</div>
                    {/* A new market starts "halted" and becomes "trading" once the
                        engine accepts it. If the engine didn't answer in time, the
                        backend's relay keeps retrying in the background. */}
                    {data.status !== "trading" && (
                        <div>
                            The trading engine hasn't confirmed this market yet, so it's{" "}
                            {data.status}. The backend keeps retrying; look the market up
                            again in a moment.
                        </div>
                    )}
                    <MarketInfo market={data} />
                </>
            )}
        </>
    );
}
