import { useParams } from "react-router";
import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function TradePage() {
    // New React Router: useParams() reads the ":symbol" part of the route's path
    const { symbol } = useParams();

    return (
        <>
            <PageHeader title={symbol} />
            <PagePlaceholder phase={2}>
                The trade page: live price, chart, order form, recent trades and your
                orders in this market.
            </PagePlaceholder>
        </>
    );
}
