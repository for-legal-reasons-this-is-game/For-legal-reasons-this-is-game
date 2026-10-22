import { type FormEvent, type ChangeEvent, useState } from "react";
import { api, errorMessage } from "../api";
import type { UserPayload } from "../types/UserPayload";
import { useMutation, useQueryClient } from "@tanstack/react-query";

export function CreateUser() {
    const [formData, setFormData] = useState<UserPayload>({
        name: "",
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    const queryClient = useQueryClient();
    const { isPending, isError, isSuccess, data, error, mutate } = useMutation({
        mutationFn: (payload: UserPayload) => api.users.create(payload),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ["users"] }),
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (!formData.name.trim()) {
            setValidationError("Name field can't be empty");
            return;
        }
        mutate({
            ...formData,
            name: formData.name.trim(),
        });
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
        setValidationError(null);
        const { name, value } = event.target;
        setFormData((prev) => ({
            ...prev,
            [name]: value,
        }));
    }

    return (
        <div>
            <h3>Create User</h3>
            <form onSubmit={handleSubmit}>
                <input
                    name="name"
                    value={formData.name}
                    onChange={handleChange}
                    placeholder="User Name"
                />
                <button disabled={isPending} type="submit">
                    Create User
                </button>
            </form>
            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isPending && <div>Loading...</div>}
            {isSuccess && (
                <div>
                    <div>Created User...</div>
                    <div>name: {data.user_name}</div>
                    <div>id: {data.user_id}</div>
                </div>
            )}
        </div>
    );
}
