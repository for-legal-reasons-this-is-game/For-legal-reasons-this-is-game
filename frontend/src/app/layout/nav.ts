import {
    BookOpenIcon,
    ChartCandlestickIcon,
    FlaskConicalIcon,
    ListOrderedIcon,
    StoreIcon,
    UsersIcon,
    WalletIcon,
    type LucideIcon,
} from "lucide-react";

// alsoActiveOn: other paths that belong to this item (an account page is part of
// the portfolio)
export type NavItem = {
    title: string;
    to: string;
    icon: LucideIcon;
    alsoActiveOn?: string[];
};
export type NavGroup = { title: string; items: NavItem[] };

// The sidebar's links, grouped by who uses them.
export const NAV_GROUPS: NavGroup[] = [
    {
        title: "Trading",
        items: [
            { title: "Markets", to: "/markets", icon: ChartCandlestickIcon },
            {
                title: "Portfolio",
                to: "/portfolio",
                icon: WalletIcon,
                alsoActiveOn: ["/accounts"],
            },
            { title: "Orders", to: "/orders", icon: ListOrderedIcon },
        ],
    },
    {
        title: "Admin",
        items: [
            { title: "Users", to: "/users", icon: UsersIcon },
            { title: "Ledgers", to: "/admin/ledgers", icon: BookOpenIcon },
            { title: "Market setup", to: "/admin/markets", icon: StoreIcon },
        ],
    },
    {
        // the old test UI, until every part of it has a replacement (phase 5)
        title: "Developer",
        items: [{ title: "Old test UI", to: "/dev", icon: FlaskConicalIcon }],
    },
];

// "/markets/BTC-USD" still counts as being on Markets, but "/admin/markets" doesn't
export function isNavItemActive(pathname: string, item: NavItem): boolean {
    return [item.to, ...(item.alsoActiveOn ?? [])].some(
        (path) => pathname === path || pathname.startsWith(`${path}/`),
    );
}
