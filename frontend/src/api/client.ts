// Thin wrapper around fetch: one place for the base URL, JSON headers,
// status checking and error shaping. Components never call fetch directly.
//
// Every failure in request() is thrown as an ApiError, never as a raw
// TypeError/SyntaxError. That's what makes the TanStack Query declaration in
// react-query.d.ts (`defaultError: ApiError`) true, so `error.status` is safe.

// '' in dev: the Vite proxy forwards /api to the backend (see vite.config.ts).
// Set VITE_API_BASE at build time if the API lives on another origin.
const API_BASE: string = import.meta.env.VITE_API_BASE ?? "";

// network:          no response at all (backend/proxy unreachable, offline, CORS)
// http:             the server answered with a non-2xx status
// invalid-response: a 2xx whose body isn't the JSON we expected
export type ApiErrorKind = "network" | "http" | "invalid-response";

export class ApiError extends Error {
    kind: ApiErrorKind;
    // HTTP status; 0 when there was no response (kind "network").
    status: number;
    // Text the server sent, e.g. axum's "Failed to deserialize the JSON body..."
    // on a 422. Empty when there was none, or when it was an HTML error page.
    body: string;

    constructor(kind: ApiErrorKind, status: number, body = "", cause?: unknown) {
        super(body ? `${status}: ${body}` : `${kind} error (${status})`, { cause });
        this.name = "ApiError";
        this.kind = kind;
        this.status = status;
        this.body = body;
    }
}

type Method = "GET" | "POST" | "PATCH" | "PUT" | "DELETE";

async function request<T>(method: Method, path: string, body?: unknown): Promise<T> {
    let res: Response;
    let text: string;
    try {
        res = await fetch(`${API_BASE}/api/v1${path}`, {
            method,
            headers:
                body !== undefined ? { "Content-Type": "application/json" } : undefined,
            body: body !== undefined ? JSON.stringify(body) : undefined,
        });
        text = await res.text();
    } catch (e) {
        throw new ApiError("network", 0, "", e);
    }

    if (!res.ok) {
        const isHtml = res.headers.get("Content-Type")?.includes("text/html");
        throw new ApiError("http", res.status, isHtml ? "" : text);
    }

    if (!text) return undefined as T;
    try {
        return JSON.parse(text) as T;
    } catch (e) {
        throw new ApiError("invalid-response", res.status, "", e);
    }
}

export const http = {
    get: <T>(path: string) => request<T>("GET", path),
    post: <T>(path: string, body?: unknown) => request<T>("POST", path, body),
    patch: <T>(path: string, body?: unknown) => request<T>("PATCH", path, body),
    put: <T>(path: string, body?: unknown) => request<T>("PUT", path, body),
    delete: <T>(path: string) => request<T>("DELETE", path),
};

// Per-status wording for one screen, e.g. { 404: "No ledger with this symbol" }.
export type ErrorOverrides = Partial<Record<number, string>>;

// Turn any error into a message you can show the user. Pass `overrides` for
// statuses that need wording specific to the screen.
export function errorMessage(error: unknown, overrides: ErrorOverrides = {}): string {
    if (!(error instanceof ApiError)) {
        // Not from request(): a bug in our own code. Show something rather than nothing.
        return error instanceof Error ? error.message : String(error);
    }

    switch (error.kind) {
        case "network":
            return "Can't reach the server. Is the backend running?";
        case "invalid-response":
            return "The server sent an unexpected response.";
        case "http": {
            const override = overrides[error.status];
            if (override) return override;
            if (error.status >= 502 && error.status <= 504)
                return "The server is unavailable. Try again shortly.";
            if (error.body) return error.body;
            if (error.status === 404) return "Not found";
            if (error.status === 400) return "Bad request";
            return `Server error (${error.status})`;
        }
    }
}
