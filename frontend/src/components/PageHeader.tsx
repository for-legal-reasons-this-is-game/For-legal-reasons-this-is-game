import type { ReactNode } from "react";

// Title row of a page: the title in Archivo's expanded width (the plan's type rules),
// an optional one-line description, and optional actions on the right.
export function PageHeader({
    title,
    description,
    actions,
}: {
    title: ReactNode;
    description?: ReactNode;
    actions?: ReactNode;
}) {
    return (
        <div className="mb-6 flex flex-wrap items-end justify-between gap-4">
            <div className="min-w-0">
                <h1 className="text-2xl font-bold font-stretch-expanded">{title}</h1>
                {description && (
                    // under 80 characters per line, for readability
                    <p className="mt-1 max-w-[70ch] text-muted-foreground">
                        {description}
                    </p>
                )}
            </div>
            {actions && <div className="flex items-center gap-2">{actions}</div>}
        </div>
    );
}
