import { NavLink, useLocation } from "react-router";
import { NotebookTextIcon } from "lucide-react";
import {
    Sidebar,
    SidebarContent,
    SidebarGroup,
    SidebarGroupContent,
    SidebarGroupLabel,
    SidebarHeader,
    SidebarMenu,
    SidebarMenuButton,
    SidebarMenuItem,
    SidebarRail,
    useSidebar,
} from "@/components/ui/sidebar";
import { NAV_GROUPS, isNavItemActive } from "./nav";

// Navigation rail: full width on desktop, icons only when collapsed (Ctrl/Cmd+B or the
// button in the top bar), and a sheet that slides in on mobile.
export function AppSidebar() {
    const { pathname } = useLocation();
    const { setOpenMobile } = useSidebar();

    return (
        <Sidebar collapsible="icon">
            <SidebarHeader>
                <SidebarMenu>
                    <SidebarMenuItem>
                        <SidebarMenuButton size="lg" asChild>
                            <NavLink to="/markets" onClick={() => setOpenMobile(false)}>
                                {/* a 32 px box: collapsed to icons, this button is 32 px
                                    wide with no padding, so the mark stays centred */}
                                <span className="flex size-8 shrink-0 items-center justify-center">
                                    <NotebookTextIcon className="text-primary" />
                                </span>
                                {/* whitespace-normal! ("!" = important, to beat shadcn's more specific
                                    selector): wrap onto two lines instead of
                                    shadcn's truncation, so the full name shows */}
                                <span className="leading-tight font-semibold whitespace-normal! group-data-[collapsible=icon]:hidden">
                                    For legal reasons this is game
                                </span>
                            </NavLink>
                        </SidebarMenuButton>
                    </SidebarMenuItem>
                </SidebarMenu>
            </SidebarHeader>
            <SidebarContent>
                {/* a navigation landmark, so screen readers can jump to the page list */}
                <nav aria-label="Pages" className="flex flex-col gap-2">
                    {NAV_GROUPS.map((group) => (
                        <SidebarGroup key={group.title}>
                            <SidebarGroupLabel>{group.title}</SidebarGroupLabel>
                            <SidebarGroupContent>
                                <SidebarMenu>
                                    {group.items.map((item) => (
                                        <SidebarMenuItem key={item.to}>
                                            <SidebarMenuButton
                                                asChild
                                                isActive={isNavItemActive(pathname, item)}
                                                tooltip={item.title}>
                                                {/* on mobile the sidebar is a sheet: close
                                                    it after choosing a page */}
                                                <NavLink
                                                    to={item.to}
                                                    onClick={() => setOpenMobile(false)}>
                                                    <item.icon />
                                                    <span>{item.title}</span>
                                                </NavLink>
                                            </SidebarMenuButton>
                                        </SidebarMenuItem>
                                    ))}
                                </SidebarMenu>
                            </SidebarGroupContent>
                        </SidebarGroup>
                    ))}
                </nav>
            </SidebarContent>
            <SidebarRail />
        </Sidebar>
    );
}
