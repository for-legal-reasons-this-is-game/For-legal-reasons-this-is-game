import { useParams } from "react-router";
import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function AccountPage() {
    // New React Router: useParams() reads the ":accountId" part of the route's path
    const { accountId } = useParams();

    return (
        <>
            <PageHeader title={`Account ${accountId}`} />
            <PagePlaceholder phase={3}>
                This account's balance as a ledger (debits and credits), its transfers,
                deposits, and closing or reopening it.
            </PagePlaceholder>
        </>
    );
}
