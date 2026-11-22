import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function PortfolioPage() {
    return (
        <>
            <PageHeader title="Portfolio" />
            <PagePlaceholder phase={3}>
                Your accounts and balances, with what is on hold for open orders. Open
                accounts and deposit from here.
            </PagePlaceholder>
        </>
    );
}
