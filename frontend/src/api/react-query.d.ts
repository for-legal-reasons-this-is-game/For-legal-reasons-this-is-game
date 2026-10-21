// Tells TanStack Query that `error` from useQuery/useMutation is an ApiError
// (instead of the default `Error`), so components can read `error.status`.
//
// This is only true because client.ts converts every failure into an ApiError.
// Keep it true: query and mutation functions must only throw errors that come
// from `api.*` calls, never `throw new Error(...)` of their own. Do input
// validation in the component, before calling mutate()/enabling the query.
import type { ApiError } from "./client";

declare module "@tanstack/react-query" {
    interface Register {
        defaultError: ApiError;
    }
}
