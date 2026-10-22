import { type User } from "../types/User";
import { api, useFetch } from "../api";

function User({ user, setUserID }: { user: User; setUserID: (id: string) => void }) {
    return (
        <>
            <span>user: {user.user_name}</span>
            <br />
            <span>id: {user.user_id}</span>
            <button onClick={() => setUserID(user.user_id)}>Select User</button>
            <br />
        </>
    );
}

export function ListUsers({ setUserID }: { setUserID: (id: string) => void }) {
    const { data, error, loading, refresh } = useFetch(() => api.users.list());

    return (
        <>
            <h3>Users</h3>
            <button onClick={refresh}>refresh</button>
            {loading && <div>Loading...</div>}
            {error && <div style={{ color: "red" }}>{error}</div>}
            {data?.map((user) => {
                return (
                    <div key={user.user_id}>
                        <User user={user} setUserID={setUserID} />
                        <br />
                    </div>
                );
            })}
        </>
    );
}
