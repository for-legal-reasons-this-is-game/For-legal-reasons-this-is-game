import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function AdminLedgersPage() {
    return (
        <>
            <PageHeader title="Ledgers" />
            <PagePlaceholder phase={4}>
                The assets the exchange knows (USD, BTC, ...): create them and switch them
                on or off.
            </PagePlaceholder>
        </>
    );
}
