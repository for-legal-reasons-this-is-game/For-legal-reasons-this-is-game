import { Link, useLocation } from "react-router";
import { Button } from "@/components/ui/button";
import { PageHeader } from "@/components/PageHeader";

export function NotFoundPage() {
    const { pathname } = useLocation();

    return (
        <>
            <PageHeader
                title="Page not found"
                description={`There's no page at ${pathname}. It may have moved, or the link has a typo.`}
            />
            <Button asChild>
                <Link to="/markets">Go to markets</Link>
            </Button>
        </>
    );
}
