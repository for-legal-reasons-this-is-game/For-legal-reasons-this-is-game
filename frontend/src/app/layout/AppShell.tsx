import { Suspense, type MouseEvent } from "react";
import { Outlet, useLocation } from "react-router";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Skeleton } from "@/components/ui/skeleton";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { AppSidebar } from "./AppSidebar";
import { TopBar } from "./TopBar";

// What shows while a page's code downloads (the pages load lazily, see app/App.tsx):
// the shape of a page header and a panel.
function PageSkeleton() {
    return (
        <div role="status" className="grid gap-3">
            <span className="sr-only">Loading the page</span>
            <Skeleton className="h-8 w-64 max-w-full" />
            <Skeleton className="h-4 w-96 max-w-full" />
            <Skeleton className="mt-3 h-64 rounded-2xl" />
        </div>
    );
}

// Moves keyboard focus to the page itself, past the sidebar and the top bar. Done in code
// rather than with href="#content" alone, which would leave "#content" in the address.
function skipToContent(event: MouseEvent<HTMLAnchorElement>) {
    event.preventDefault();
    document.getElementById("content")?.focus();
}

// The frame around every page. New React Router: <Outlet /> is where the matched child
// route's page renders (see the routes in app/App.tsx).
export function AppShell() {
    const { pathname } = useLocation();

    return (
        <SidebarProvider>
            {/* The first stop for the Tab key: hidden until it has focus, then it lets
                keyboard users skip the sidebar links on every page */}
            <a
                href="#content"
                onClick={skipToContent}
                className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:rounded-lg focus:border focus:bg-background focus:px-3 focus:py-2 focus:text-sm focus:font-medium focus:shadow-md">
                Skip to content
            </a>
            <AppSidebar />
            {/* SidebarInset is the page's <main> element */}
            <SidebarInset>
                <TopBar />
                {/* tabIndex -1: the skip link can focus it, the Tab key doesn't stop here */}
                <div
                    id="content"
                    tabIndex={-1}
                    className="flex-1 p-4 outline-none md:p-6">
                    <ErrorBoundary resetKey={pathname}>
                        <Suspense fallback={<PageSkeleton />}>
                            <Outlet />
                        </Suspense>
                    </ErrorBoundary>
                </div>
            </SidebarInset>
        </SidebarProvider>
    );
}
