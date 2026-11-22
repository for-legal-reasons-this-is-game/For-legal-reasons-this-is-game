import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { ActingUserSwitcher } from "@/app/acting-user/ActingUserSwitcher";
import { ThemeToggle } from "./ThemeToggle";

export function TopBar() {
    return (
        <header className="sticky top-0 z-10 flex h-14 shrink-0 items-center gap-2 border-b bg-background px-4">
            <SidebarTrigger className="-ml-1" />
            {/* self-auto: shadcn's vertical separator stretches to full height by
                default, which put this short one at the top of the bar */}
            <Separator
                orientation="vertical"
                className="mr-1 data-vertical:h-4 data-vertical:self-auto"
            />
            {/* the sidebar shows the name on desktop; on mobile the sidebar is hidden */}
            <span className="truncate text-sm font-semibold md:hidden">
                For legal reasons this is game
            </span>
            <div className="ml-auto flex items-center gap-2">
                {/* the project's own joke, and true: balances are play money */}
                <Badge variant="outline" className="hidden sm:inline-flex">
                    Play money
                </Badge>
                <ActingUserSwitcher />
                <ThemeToggle />
            </div>
        </header>
    );
}
