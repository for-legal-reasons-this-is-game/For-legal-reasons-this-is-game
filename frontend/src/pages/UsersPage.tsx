import { PageHeader } from "@/components/PageHeader";
import { PagePlaceholder } from "./PagePlaceholder";

export function UsersPage() {
    return (
        <>
            <PageHeader title="Users" />
            <PagePlaceholder phase={4}>
                Create users and choose which one the app acts as, until there's a login.
            </PagePlaceholder>
        </>
    );
}
