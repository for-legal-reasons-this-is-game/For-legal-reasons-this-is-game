import { useParams } from "react-router";
import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function OrderPage() {
    // New React Router: useParams() reads the ":orderId" part of the route's path
    const { orderId } = useParams();

    return (
        <>
            <PageHeader title={`Order ${orderId}`} />
            <PagePlaceholder phase={4}>
                This order's details and the trades that filled it.
            </PagePlaceholder>
        </>
    );
}
