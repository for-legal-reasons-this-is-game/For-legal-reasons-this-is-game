import { createContext, useContext } from "react";

// There's no login yet (Keycloak is planned), so the app "acts as" a user picked in the
// top bar. Login will replace this context later; components only use useActingUser().
export type ActingUser = {
    // "" when no user is picked yet
    userID: string;
    setUserID: (id: string) => void;
};

// New React: createContext makes a value that any component below its Provider can
// read with useContext, without passing it down as a prop through every component.
export const ActingUserContext = createContext<ActingUser | null>(null);

export function useActingUser(): ActingUser {
    const value = useContext(ActingUserContext);
    if (!value) throw new Error("useActingUser must be used inside <ActingUserProvider>");
    return value;
}
