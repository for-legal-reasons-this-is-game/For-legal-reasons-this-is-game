import { PageHeader } from "@/components/PageHeader";
import { MarketsTable } from "@/features/market/MarketsTable";

export function MarketsPage() {
    return (
        <>
            {/* New React 19: a <title> anywhere in a component is moved into <head>, so
                each page can name the browser tab */}
            <title>Markets - For legal reasons this is game</title>
            <PageHeader
                title="Markets"
                description="Prices update every few seconds. Choose a market to see its chart and trade it."
            />
            <MarketsTable />
        </>
    );
}
