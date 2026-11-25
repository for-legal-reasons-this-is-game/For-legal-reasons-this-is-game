import { lazy } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router";
import { AppShell } from "./layout/AppShell";
import { MarketsPage } from "@/pages/MarketsPage";
import { NotFoundPage } from "@/pages/NotFoundPage";

// New React: lazy() loads a component's code the first time it renders. Each page below
// becomes its own file, which the browser downloads only when the page is first visited,
// so the app starts with less to download (the trade page alone brings the chart
// library). While a page's file loads, the <Suspense> in AppShell shows a placeholder.
// lazy() needs a default export, so `then` picks the page's named export out of the module.
// The markets page, where the app opens, and the tiny not-found page load with the app.
const TradePage = lazy(() =>
    import("@/pages/TradePage").then((module) => ({ default: module.TradePage })),
);
const PortfolioPage = lazy(() =>
    import("@/pages/PortfolioPage").then((module) => ({ default: module.PortfolioPage })),
);
const AccountPage = lazy(() =>
    import("@/pages/AccountPage").then((module) => ({ default: module.AccountPage })),
);
const OrdersPage = lazy(() =>
    import("@/pages/OrdersPage").then((module) => ({ default: module.OrdersPage })),
);
const OrderPage = lazy(() =>
    import("@/pages/OrderPage").then((module) => ({ default: module.OrderPage })),
);
const UsersPage = lazy(() =>
    import("@/pages/UsersPage").then((module) => ({ default: module.UsersPage })),
);
const AdminLedgersPage = lazy(() =>
    import("@/pages/AdminLedgersPage").then((module) => ({
        default: module.AdminLedgersPage,
    })),
);
const AdminMarketsPage = lazy(() =>
    import("@/pages/AdminMarketsPage").then((module) => ({
        default: module.AdminMarketsPage,
    })),
);

// New React Router: each <Route> pairs a URL path with the page to show. The routes
// inside the AppShell route render in its <Outlet />, so every page gets the sidebar
// and top bar. The page list is in frontend/REDESIGN.md ("Pages and routes").
export function App() {
    return (
        <BrowserRouter>
            <Routes>
                <Route element={<AppShell />}>
                    <Route index element={<Navigate to="/markets" replace />} />
                    <Route path="markets" element={<MarketsPage />} />
                    <Route path="markets/:symbol" element={<TradePage />} />
                    <Route path="portfolio" element={<PortfolioPage />} />
                    <Route path="accounts/:accountId" element={<AccountPage />} />
                    <Route path="orders" element={<OrdersPage />} />
                    <Route path="orders/:orderId" element={<OrderPage />} />
                    <Route path="users" element={<UsersPage />} />
                    <Route path="admin/ledgers" element={<AdminLedgersPage />} />
                    <Route path="admin/markets" element={<AdminMarketsPage />} />
                    {/* "*" matches any path no other route matched */}
                    <Route path="*" element={<NotFoundPage />} />
                </Route>
            </Routes>
        </BrowserRouter>
    );
}
