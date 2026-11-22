import DevApp from "@/dev/DevApp";
import { PageHeader } from "@/components/PageHeader";
import "@/dev/legacy.css";

// The old test UI, kept working until every part of it has a new page (phase 5).
export function DevPage() {
    return (
        <>
            <PageHeader
                title="Old test UI"
                description="The first version of the frontend: one form per backend route. It stays here until the new pages replace every part of it."
            />
            <div className="legacy-ui">
                <DevApp />
            </div>
        </>
    );
}
