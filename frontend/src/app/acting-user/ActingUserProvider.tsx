import { useState, type ReactNode } from "react";
import { ActingUserContext } from "./context";

const STORAGE_KEY = "acting-user-id";

// localStorage can throw (private windows, blocked storage), so reads and writes are
// wrapped: the app still works, it just won't remember the user.
function readStoredUserID(): string {
    try {
        return localStorage.getItem(STORAGE_KEY) ?? "";
    } catch {
        return "";
    }
}

export function ActingUserProvider({ children }: { children: ReactNode }) {
    // New React: useState(readStoredUserID) passes the function itself, which React calls
    // only on the first render. useState(readStoredUserID()) would read storage on every
    // render and throw the result away.
    const [userID, setUserIDState] = useState(readStoredUserID);

    function setUserID(id: string) {
        setUserIDState(id);
        try {
            localStorage.setItem(STORAGE_KEY, id);
        } catch {
            // not remembered, but still used for this visit
        }
    }

    return (
        <ActingUserContext.Provider value={{ userID, setUserID }}>
            {children}
        </ActingUserContext.Provider>
    );
}
