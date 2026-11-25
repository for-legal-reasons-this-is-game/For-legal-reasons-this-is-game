import type { ReactNode } from "react";

// What a page shows when it has nothing to list yet: a dashed box saying what to do
// instead (an empty page is an invitation to act).
export function EmptyState({ children }: { children: ReactNode }) {
    return (
        <div className="max-w-[70ch] rounded-2xl border border-dashed p-6">
            {children}
        </div>
    );
}
