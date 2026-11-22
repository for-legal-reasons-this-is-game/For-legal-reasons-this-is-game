import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function MarketsPage() {
    return (
        <>
            <PageHeader title="Markets" />
            <PagePlaceholder phase={2}>
                Every market with its last price, 24-hour change, high, low and volume.
                Choose one to trade it.
            </PagePlaceholder>
        </>
    );
}
