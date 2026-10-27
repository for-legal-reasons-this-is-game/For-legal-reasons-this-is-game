import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

// New TanStack: defaultOptions applies to every useQuery in the app.
// By default a failed query is retried 3 times (after ~1s, 2s and 4s), so a
// "not found" took ~7 seconds to appear. A 4xx answer won't change on a retry,
// so only network errors and 5xx (kind "network"/"invalid-response", or
// status >= 500) are retried. `error` is an ApiError (see api/react-query.d.ts).
const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            retry: (failureCount, error) =>
                failureCount < 3 && !(error.kind === "http" && error.status < 500),
        },
    },
});

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <QueryClientProvider client={queryClient}>
            <App />
        </QueryClientProvider>
    </StrictMode>,
);
