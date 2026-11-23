import { useSyncExternalStore } from "react";

// There's no GET /users/{id}/accounts yet, so this browser remembers the accounts it has
// seen for each user: accounts opened here and accounts added by ID (decision 4 in
// frontend/REDESIGN.md). use-user-accounts.ts adds the ones found in the user's orders.
// Stored in localStorage as a list of IDs per user, e.g. "known-accounts:<user id>".

const PREFIX = "known-accounts:";
const NONE: string[] = [];

// The lists in memory, so the app keeps working when storage is blocked (it just won't
// remember them after a reload)
const lists = new Map<string, string[]>();
const listeners = new Set<() => void>();

function readStorage(userID: string): string[] {
    try {
        const parsed: unknown = JSON.parse(localStorage.getItem(PREFIX + userID) ?? "[]");
        return Array.isArray(parsed)
            ? parsed.filter((id): id is string => typeof id === "string")
            : [];
    } catch {
        // blocked storage, or text that isn't JSON: start with an empty list
        return [];
    }
}

function getList(userID: string): string[] {
    let list = lists.get(userID);
    if (!list) {
        list = readStorage(userID);
        lists.set(userID, list);
    }
    return list;
}

function notify() {
    listeners.forEach((listener) => listener());
}

export function rememberAccount(userID: string, accountID: string) {
    const list = getList(userID);
    if (list.includes(accountID)) return;
    // a new array, never a changed one: React sees the change because the value differs
    const next = [...list, accountID];
    lists.set(userID, next);
    try {
        localStorage.setItem(PREFIX + userID, JSON.stringify(next));
    } catch {
        // kept in memory for this visit
    }
    notify();
}

// another tab changed a list: read it again
function onStorage(event: StorageEvent) {
    if (event.key?.startsWith(PREFIX)) {
        lists.delete(event.key.slice(PREFIX.length));
        notify();
    }
}

function subscribe(listener: () => void) {
    listeners.add(listener);
    window.addEventListener("storage", onStorage);
    return () => {
        listeners.delete(listener);
        window.removeEventListener("storage", onStorage);
    };
}

// The account IDs remembered for a user ([] when no user is picked).
// New React: useSyncExternalStore reads a value that lives outside React (here: this
// module's lists) and re-renders the component when subscribe's listener is called.
// getSnapshot must return the same array until the list really changes, which is why
// rememberAccount replaces the array instead of pushing into it.
export function useKnownAccountIDs(userID: string): string[] {
    return useSyncExternalStore(subscribe, () => (userID ? getList(userID) : NONE));
}
