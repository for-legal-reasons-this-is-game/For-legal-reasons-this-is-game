import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function AdminMarketsPage() {
    return (
        <>
            <PageHeader title="Market setup" />
            <PagePlaceholder phase={4}>
                Create markets from two ledgers and see which ones the trading engine has
                accepted.
            </PagePlaceholder>
        </>
    );
}
