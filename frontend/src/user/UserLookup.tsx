import { useState, type FormEvent, type ChangeEvent } from "react";
import { api, errorMessage } from "../api";
import { useQuery } from "@tanstack/react-query";

export function UserLookup({ setUserID }: { setUserID: (id: string) => void }) {
    const [userID, setLookupUserID] = useState<string>("");
    const [submittedUserID, setSubmittedUserID] = useState("");
    const [validationError, setValidationError] = useState<string | null>(null);

    // ["users", id] shares the ["users"] prefix with the users list, so creating a
    // user (which invalidates ["users"]) refreshes this lookup too
    const { isFetching, isError, isSuccess, data, error } = useQuery({
        queryKey: ["users", submittedUserID],
        queryFn: () => api.users.get(submittedUserID),
        enabled: !!submittedUserID,
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (!userID.trim()) {
            setValidationError("UserID field can't be empty");
            return;
        }
        setSubmittedUserID(userID.trim());
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setValidationError(null);
        setLookupUserID(event.target.value);
    }

    return (
        <>
            <h3>User Lookup</h3>
            <form onSubmit={handleSubmit}>
                <input onChange={handleChange} value={userID} placeholder="UserID" />
                <button type="submit">Get User</button>
            </form>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && (
                <div style={{ color: "red" }}>
                    {errorMessage(error, {
                        400: "That's not a valid user ID",
                        404: "A user with this ID doesn't exist",
                    })}
                </div>
            )}
            {isFetching && <div>Loading...</div>}
            {isSuccess && (
                <>
                    <div>Name: {data.user_name} </div>
                    <div>ID: {data.user_id} </div>
                    <button onClick={() => setUserID(data.user_id)}>Select User</button>
                </>
            )}
        </>
    );
}
