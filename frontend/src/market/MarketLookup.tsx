import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";
import type { Market } from "../types/Market";
import { MarketTicker } from "./MarketTicker";
import { ListMarketOrders } from "./ListMarketOrders";
import { ListMarketTrades } from "./ListMarketTrades";

export function MarketInfo({ market }: { market: Market }) {
    return (
        <>
            <h4>Market Symbol: {market.symbol} </h4>
            <div>ID: {market.market_id} </div>
            <div>Status: {market.status} </div>
            <div>Maker Fee BPS: {market.maker_fee_bps} </div>
            <div>Taker Fee BPS: {market.taker_fee_bps} </div>
            <br />
            <div>Base Asset Name: {market.base_asset.name} </div>
            <div>Base Asset Symbol: {market.base_asset.symbol} </div>
            <div>Base Asset Enabled: {market.base_asset.enabled ? "yes" : "no"} </div>
            <div>Base Asset Decimals: {market.base_asset.decimals} </div>
            <br />
            <div>Min Base Quantity: {market.min_base_quantity} </div>
            <div>Min Quote Quantity: {market.min_quote_quantity} </div>
            <div>Price Decimals: {market.price_decimals} </div>
            <div>Price Step: {market.price_step} </div>
            <div>Quantity Step: {market.quantity_step} </div>
            <br />
            <div>Quote Asset Name: {market.quote_asset.name} </div>
            <div>Quote Asset Symbol: {market.quote_asset.symbol} </div>
            <div>Quote Asset Enabled: {market.quote_asset.enabled ? "yes" : "no"} </div>
            <div>Quote Asset Decimals: {market.quote_asset.decimals} </div>
            <br />
        </>
    );
}

export function MarketLookup() {
    const [symbol, setSymbol] = useState<string>("");
    const [submittedSymbol, setSubmittedSymbol] = useState("");
    const [validationError, setValidationError] = useState<string | null>(null);
    const [ticker, setTicker] = useState(false);
    const [marketOrders, setMarketOrders] = useState(false);
    const [marketTrades, setMarketTrades] = useState(false);

    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["markets", submittedSymbol],
        queryFn: () => api.markets.get(submittedSymbol),
        enabled: !!submittedSymbol,
    });
    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (!symbol.trim()) {
            setValidationError("Symbol field can't be empty");
            return;
        }
        setSubmittedSymbol(symbol.trim());
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setValidationError(null);
        setSymbol(event.target.value);
    }

    return (
        <>
            <h3>Market Lookup</h3>
            <form onSubmit={handleSubmit}>
                <input
                    onChange={handleChange}
                    value={symbol}
                    placeholder="Market Symbol"
                />
                <button type="submit">Get Market</button>
            </form>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, {
                        404: "A market with this symbol doesn't exist",
                    })}
                </div>
            )}
            {isFetching && <div>Loading...</div>}
            {isSuccess && (
                <>
                    <MarketInfo market={data} />
                    <button
                        style={{ background: ticker ? "green" : "grey" }}
                        onClick={() => setTicker(!ticker)}>
                        Ticker
                    </button>
                    <button
                        style={{ background: marketOrders ? "green" : "grey" }}
                        onClick={() => setMarketOrders(!marketOrders)}>
                        Orders
                    </button>
                    <button
                        style={{ background: marketTrades ? "green" : "grey" }}
                        onClick={() => setMarketTrades(!marketTrades)}>
                        Trades
                    </button>
                    {ticker && <MarketTicker symbol={data.symbol} />}
                    {marketOrders && <ListMarketOrders symbol={data.symbol} />}
                    {marketTrades && <ListMarketTrades symbol={data.symbol} />}
                </>
            )}
        </>
    );
}
