import type { ReactNode } from "react";
import { fromUnits, toUnits } from "@/lib/decimal";
import { formatDecimal } from "@/lib/format";
import type { Position } from "@/types/Position";
import { availableUnits } from "./balance";

function Line({ label, value }: { label: string; value: ReactNode }) {
    return (
        <div className="flex items-baseline justify-between gap-4">
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="text-right">{value}</dd>
        </div>
    );
}

// An account's balance laid out like a ledger page (a "T account"): money out on the left
// (debits), money in on the right (credits), each split into posted (done) and pending
// (reserved, not done yet), then a double rule and the balance. A user account is credited
// by deposits and trades it receives, and debited by holds for orders, fees and trades it
// pays for.
export function LedgerSummary({ position }: { position: Position }) {
    const units = availableUnits(position);
    const pendingOut = toUnits(position.debits_pending, position.decimals);
    const balance = `${formatDecimal(position.net_posted)} ${position.symbol}`;

    return (
        <section
            aria-labelledby="ledger-heading"
            className="rounded-2xl border bg-card p-4">
            <h2 id="ledger-heading" className="font-semibold">
                Balance
            </h2>
            <div className="mt-3 grid grid-cols-2">
                <div className="pr-4">
                    <h3 className="text-sm font-medium">
                        Out{" "}
                        <span className="font-normal text-muted-foreground">
                            (debits)
                        </span>
                    </h3>
                    <dl className="mt-2 grid gap-1 text-sm">
                        <Line
                            label="Posted"
                            value={formatDecimal(position.debits_posted)}
                        />
                        <Line
                            label="Pending"
                            value={formatDecimal(position.debits_pending)}
                        />
                    </dl>
                </div>
                {/* the rule down the middle of a T account */}
                <div className="border-l pl-4">
                    <h3 className="text-sm font-medium">
                        In{" "}
                        <span className="font-normal text-muted-foreground">
                            (credits)
                        </span>
                    </h3>
                    <dl className="mt-2 grid gap-1 text-sm">
                        <Line
                            label="Posted"
                            value={formatDecimal(position.credits_posted)}
                        />
                        <Line
                            label="Pending"
                            value={formatDecimal(position.credits_pending)}
                        />
                    </dl>
                </div>
            </div>
            <dl className="mt-3 grid gap-1 border-t-[3px] border-double border-foreground/70 pt-2">
                <Line
                    label="Balance"
                    value={
                        <span className="font-semibold whitespace-nowrap">{balance}</span>
                    }
                />
                {/* the same as the balance unless the account is closing */}
                {units !== null && pendingOut !== null && pendingOut > 0n && (
                    <Line
                        label="Available"
                        value={`${formatDecimal(fromUnits(units, position.decimals))} ${position.symbol}`}
                    />
                )}
            </dl>
            <p className="mt-2 text-xs text-muted-foreground">
                The balance is what came in minus what went out, counting posted transfers
                only.
            </p>
        </section>
    );
}
