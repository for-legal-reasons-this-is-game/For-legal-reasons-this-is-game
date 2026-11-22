import { Outlet } from "react-router";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { AppSidebar } from "./AppSidebar";
import { TopBar } from "./TopBar";

// The frame around every page. New React Router: <Outlet /> is where the matched child
// route's page renders (see the routes in app/App.tsx).
export function AppShell() {
    return (
        <SidebarProvider>
            <AppSidebar />
            {/* SidebarInset is the page's <main> element */}
            <SidebarInset>
                <TopBar />
                <div className="flex-1 p-4 md:p-6">
                    <Outlet />
                </div>
            </SidebarInset>
        </SidebarProvider>
    );
}
