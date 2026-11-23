import { useState } from "react";
import { cn } from "@/lib/utils";

// The last price, where only the digits that changed flash in Gain (up) or Loss (down)
// for 600 ms (the animate-flash-* utilities in index.css). The design's one bold element.
export function FlashingPrice({
    raw,
    text,
    className,
}: {
    raw: string; // the decimal string, e.g. "65450.00", used to tell up from down
    text: string; // what's shown, e.g. "65,450.00"
    className?: string;
}) {
    // New React: to react to a changed prop, keep what was shown before in state and update
    // it *during render*. React then re-renders right away, before anything is painted.
    // (An effect would paint the stale version first.) `version` counts the changes.
    const [shown, setShown] = useState({
        raw,
        text,
        previousRaw: raw,
        previousText: text,
        version: 0,
    });
    if (shown.raw !== raw || shown.text !== text) {
        setShown({
            raw,
            text,
            previousRaw: shown.raw,
            previousText: shown.text,
            version: shown.version + 1,
        });
    }

    // Number() is fine here: it only decides the colour of the flash, it isn't shown
    const delta = Number(shown.raw) - Number(shown.previousRaw);
    const flash = delta > 0 ? "animate-flash-up" : delta < 0 ? "animate-flash-down" : "";

    // Digits are compared from the right, so "9,999.00" -> "10,000.00" lines up.
    // A changed digit gets a new key, which makes React create a new <span> for it, and
    // a new span plays its CSS animation. Unchanged digits keep their key and stay still.
    const chars = [...shown.text];
    const previous = [...shown.previousText];
    return (
        <span className={className} aria-label={shown.text}>
            {chars.map((char, index) => {
                const fromRight = chars.length - index;
                const changed =
                    shown.version > 0 && previous[previous.length - fromRight] !== char;
                return (
                    <span
                        key={changed ? `${fromRight}-${shown.version}` : `${fromRight}`}
                        aria-hidden
                        className={cn(changed && flash)}>
                        {char}
                    </span>
                );
            })}
        </span>
    );
}
