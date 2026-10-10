import { useEffect, useState } from "react";
import { type User } from "./types/User";
import { api, errorMessage } from "./api";

function User({ user }: { user: User }) {
    return (
        <>
            <span>user: {user.user_name}</span>
            <br />
            <span>id: {user.user_id}</span>
            <br />
        </>
    );
}

export function ListUsers() {
    const [users, setUsers] = useState<User[]>([]);
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    // bumping this number re-runs the effect
    const [reloadKey, setReloadKey] = useState(0);

    useEffect(() => {
        // If the component unmounts (or refreshes again) before this request
        // finishes, ignore its result instead of setting stale state.
        let ignore = false;
        api.users
            .list()
            .then((users) => {
                if (ignore) return;
                setUsers(users);
                setError(null);
            })
            .catch((e) => !ignore && setError(errorMessage(e)))
            .finally(() => !ignore && setLoading(false));
        return () => {
            ignore = true;
        };
    }, [reloadKey]);

    function refresh() {
        setLoading(true);
        setReloadKey((k) => k + 1);
    }

    return (
        <>
            <h3>Users</h3>
            <button onClick={refresh}>refresh</button>
            {loading && <div>Loading...</div>}
            {error && <div style={{ color: "red" }}>{error}</div>}
            {users.map((user) => (
                <div key={user.user_id}>
                    <User user={user} />
                    <br />
                </div>
            ))}
        </>
    );
}
