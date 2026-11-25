import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { CreateMarketDialog } from "@/features/admin/CreateMarketDialog";
import { MarketSetupTable } from "@/features/admin/MarketSetupTable";

const APP_NAME = "For legal reasons this is game";

export function AdminMarketsPage() {
    return (
        <>
            <title>{`Market setup - ${APP_NAME}`}</title>
            <PageHeader
                title="Market setup"
                description="Each market trades one ledger's asset for another's, with its own order rules and fees. A new market opens once the trading engine confirms it."
                actions={<CreateMarketDialog trigger={<Button>Create market</Button>} />}
            />
            <div className="rounded-2xl border bg-card px-2">
                <MarketSetupTable />
            </div>
        </>
    );
}
