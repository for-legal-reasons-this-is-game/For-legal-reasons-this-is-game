import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { CreateLedgerDialog } from "@/features/admin/CreateLedgerDialog";
import { LedgersTable } from "@/features/admin/LedgersTable";

const APP_NAME = "For legal reasons this is game";

export function AdminLedgersPage() {
    return (
        <>
            <title>{`Ledgers - ${APP_NAME}`}</title>
            <PageHeader
                title="Ledgers"
                description="The assets the exchange keeps accounts in. A ledger can't be deleted. Switched off, it takes no new accounts, deposits, orders or markets, and balances stay as they are."
                actions={<CreateLedgerDialog trigger={<Button>Create ledger</Button>} />}
            />
            <div className="max-w-3xl rounded-2xl border bg-card px-2">
                <LedgersTable />
            </div>
        </>
    );
}
