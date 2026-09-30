import { useEffect, useState } from "react";
import { type User } from "./types/User";

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
    const [refresh, setRefresh] = useState(false);

    useEffect(() => {
        async function fetchUsers() {
            try {
                const res = await fetch("http://localhost:8000/api/v1/users");
                const resJson = await res.json();
                setUsers(resJson);
                console.log(`received users:\n ${resJson}`);
                setRefresh(false);
            } catch (error) {
                console.log(`ERROR: ${error}`);
            }
        }
        fetchUsers();
    }, [refresh]);

    return (
        <>
            <h3>Users</h3>
            <button onClick={() => setRefresh(true)}>refresh</button>
            {users.map((user) => (
                <div key={user.user_id}>
                    <User user={user} />
                    <br />
                </div>
            ))}
        </>
    );
}
