import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { AccountStatus } from "@/types/AccountStatus";

// "processing": saved, and being created in the ledger (TigerBeetle); usually a moment
const LABELS: Record<AccountStatus, string> = {
    active: "Active",
    processing: "Being opened",
};

// Muted, like the other status badges: a green dot when the account can be used.
export function AccountStatusBadge({ status }: { status: AccountStatus }) {
    return (
        <Badge variant="outline" className="gap-1.5 font-normal">
            <span
                aria-hidden
                className={cn(
                    "size-1.5 rounded-full",
                    status === "active" ? "bg-gain" : "border border-muted-foreground",
                )}
            />
            {LABELS[status]}
        </Badge>
    );
}
