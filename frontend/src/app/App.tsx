import { BrowserRouter, Navigate, Route, Routes } from "react-router";
import { AppShell } from "./layout/AppShell";
import { MarketsPage } from "@/pages/MarketsPage";
import { TradePage } from "@/pages/TradePage";
import { PortfolioPage } from "@/pages/PortfolioPage";
import { AccountPage } from "@/pages/AccountPage";
import { OrdersPage } from "@/pages/OrdersPage";
import { OrderPage } from "@/pages/OrderPage";
import { UsersPage } from "@/pages/UsersPage";
import { AdminLedgersPage } from "@/pages/AdminLedgersPage";
import { AdminMarketsPage } from "@/pages/AdminMarketsPage";
import { DevPage } from "@/pages/DevPage";
import { NotFoundPage } from "@/pages/NotFoundPage";

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
                    <Route path="dev" element={<DevPage />} />
                    {/* "*" matches any path no other route matched */}
                    <Route path="*" element={<NotFoundPage />} />
                </Route>
            </Routes>
        </BrowserRouter>
    );
}
