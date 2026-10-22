import { type User } from "../types/User";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";

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
    const { isFetching, isError, data, error, refetch } = useQuery({
        queryKey: ["users"],
        queryFn: () => api.users.list(),
    });

    return (
        <>
            <h3>Users</h3>
            <button onClick={() => refetch()}>Refresh</button>
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isFetching && <div>Loading...</div>}
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
