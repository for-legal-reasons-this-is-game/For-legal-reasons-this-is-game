// Thin wrapper around fetch: one place for the base URL, JSON headers,
// status checking and error shaping. Components never call fetch directly.

// '' in dev: the Vite proxy forwards /api to the backend (see vite.config.ts).
// Set VITE_API_BASE at build time if the API lives on another origin.
const API_BASE: string = import.meta.env.VITE_API_BASE ?? "";

// Thrown for any non-2xx response. `body` is whatever text the server sent,
// e.g. axum's "Failed to deserialize the JSON body..." on a 422.
export class ApiError extends Error {
    status: number;
    body: string;

    constructor(status: number, body: string) {
        super(body ? `${status}: ${body}` : `HTTP ${status}`);
        this.name = "ApiError";
        this.status = status;
        this.body = body;
    }
}

type Method = "GET" | "POST" | "PATCH" | "PUT" | "DELETE";

async function request<T>(method: Method, path: string, body?: unknown): Promise<T> {
    const res = await fetch(`${API_BASE}/api/v1${path}`, {
        method,
        headers: body !== undefined ? { "Content-Type": "application/json" } : undefined,
        body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    // Read as text first: our backend sends empty bodies on errors (and could on 204),
    // and res.json() throws on an empty string.
    const text = await res.text();

    if (!res.ok) {
        throw new ApiError(res.status, text);
    }
    return (text ? JSON.parse(text) : undefined) as T;
}

export const http = {
    get: <T>(path: string) => request<T>("GET", path),
    post: <T>(path: string, body?: unknown) => request<T>("POST", path, body),
    patch: <T>(path: string, body?: unknown) => request<T>("PATCH", path, body),
    put: <T>(path: string, body?: unknown) => request<T>("PUT", path, body),
    delete: <T>(path: string) => request<T>("DELETE", path),
};

// Turn anything caught in a try/catch into a message you can show the user.
export function errorMessage(error: unknown): string {
    if (error instanceof ApiError) {
        if (error.body) return error.body;
        if (error.status === 404) return "Not found";
        if (error.status === 400) return "Bad request";
        return `Server error (${error.status})`;
    }
    if (error instanceof TypeError)
        return "Can't reach the server. Is the backend running?";
    return String(error);
}
