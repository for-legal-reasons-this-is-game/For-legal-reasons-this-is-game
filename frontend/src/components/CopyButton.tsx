import type { MouseEvent } from "react";
import { CopyIcon } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";

// A small button that copies `value` (usually an ID) and confirms it in a toast. `what`
// names it in the button's label and the toast, e.g. "account ID".
export function CopyButton({ value, what }: { value: string; what: string }) {
    async function copy(event: MouseEvent) {
        // inside a clickable row, copying shouldn't also open the row
        event.stopPropagation();
        try {
            await navigator.clipboard.writeText(value);
            toast.success(`Copied the ${what}`);
        } catch {
            toast.error(`Couldn't copy the ${what}. Select it on the page instead.`);
        }
    }

    return (
        <Button variant="ghost" size="icon-xs" onClick={copy} aria-label={`Copy ${what}`}>
            <CopyIcon />
        </Button>
    );
}
