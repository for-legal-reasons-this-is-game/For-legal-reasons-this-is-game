import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function OrdersPage() {
    return (
        <>
            <PageHeader title="Your orders" />
            <PagePlaceholder phase={4}>
                Every order you placed, filtered by market and status.
            </PagePlaceholder>
        </>
    );
}
