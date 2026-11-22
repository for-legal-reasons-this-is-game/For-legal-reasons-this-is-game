import { useTheme } from "next-themes";
import { MonitorIcon, MoonIcon, SunIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

// next-themes stores the choice ("light", "dark" or "system") in localStorage and puts
// the "dark" class on <html>; the colours in index.css follow that class.
export function ThemeToggle() {
    const { theme, setTheme } = useTheme();

    return (
        <DropdownMenu>
            <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="icon" aria-label="Change theme">
                    <SunIcon className="dark:hidden" />
                    <MoonIcon className="hidden dark:block" />
                </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
                <DropdownMenuRadioGroup value={theme} onValueChange={setTheme}>
                    <DropdownMenuRadioItem value="light">
                        <SunIcon />
                        Light
                    </DropdownMenuRadioItem>
                    <DropdownMenuRadioItem value="dark">
                        <MoonIcon />
                        Dark
                    </DropdownMenuRadioItem>
                    <DropdownMenuRadioItem value="system">
                        <MonitorIcon />
                        Same as system
                    </DropdownMenuRadioItem>
                </DropdownMenuRadioGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}
