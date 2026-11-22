import type { ReactNode } from "react";

// Temporary: stands in for a page until its phase of the redesign builds it.
// Delete this file once no page uses it.
export function PagePlaceholder({
    phase,
    children,
}: {
    phase: number;
    children: ReactNode;
}) {
    return (
        <div className="max-w-[70ch] rounded-2xl border border-dashed p-6">
            <p>{children}</p>
            <p className="mt-2 text-sm text-muted-foreground">
                Built in phase {phase} of the redesign (frontend/REDESIGN.md). Until then,
                the old test UI has these features.
            </p>
        </div>
    );
}
