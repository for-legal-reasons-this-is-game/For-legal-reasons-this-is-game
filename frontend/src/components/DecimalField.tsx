import type { ReactNode } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

// A text input for a number (an amount, a price, a fee), with its unit inside the box on
// the right, a hint underneath, and the error in place of the hint when there is one.
// `inputMode`: "decimal" for amounts, "numeric" for whole numbers (phones show a keyboard
// without the decimal point).
export function DecimalField({
    id,
    label,
    unit,
    value,
    onChange,
    hint,
    error,
    action,
    placeholder,
    inputMode = "decimal",
}: {
    id: string;
    label: string;
    unit?: string;
    value: string;
    onChange: (value: string) => void;
    hint?: ReactNode;
    error?: string;
    action?: ReactNode;
    placeholder?: string;
    inputMode?: "decimal" | "numeric";
}) {
    const note = error ?? hint;
    return (
        <div className="grid gap-1.5">
            <div className="flex items-baseline justify-between gap-2">
                <Label htmlFor={id}>
                    {label}
                    {unit && <span className="sr-only"> in {unit}</span>}
                </Label>
                {action}
            </div>
            <div className="relative">
                {/* text, not type="number": number inputs round long decimals and
                    accept "1e3"; inputMode still opens the number keyboard */}
                <Input
                    id={id}
                    inputMode={inputMode}
                    autoComplete="off"
                    value={value}
                    placeholder={placeholder}
                    onChange={(event) => onChange(event.target.value)}
                    className={cn("text-right", unit && "pr-12")}
                    aria-invalid={!!error}
                    aria-describedby={note ? `${id}-note` : undefined}
                />
                {unit && (
                    <span
                        aria-hidden
                        className="pointer-events-none absolute inset-y-0 right-2.5 flex items-center text-muted-foreground">
                        {unit}
                    </span>
                )}
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
