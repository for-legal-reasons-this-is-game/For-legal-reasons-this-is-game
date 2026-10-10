import { type FormEvent, type ChangeEvent, useState } from "react";
import { type User } from "./types/User";
import { api, errorMessage } from "./api";

export function CreateUser() {
    const [formData, setFormData] = useState({
        name: "",
        email: "",
        password: "",
        passwordConfirmation: "",
    });
    const [user, setUser] = useState<User | null>(null);
    const [error, setError] = useState<string | null>(null);

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        setError(null);
        if (
            !formData.name ||
            !formData.email ||
            !formData.password ||
            !formData.passwordConfirmation ||
            formData.password != formData.passwordConfirmation
        ) {
            setError(errorMessage("Empty field or mismatching password fields"));
            return;
        }
        try {
            setUser(await api.users.create(formData));
        } catch (e) {
            setError(errorMessage(e));
        }
    }

    function handleChange(event: ChangeEvent<HTMLInputElement>) {
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
                <input
                    name="email"
                    value={formData.email}
                    onChange={handleChange}
                    placeholder="Email"
                />
                <input
                    name="password"
                    type="password"
                    value={formData.password}
                    onChange={handleChange}
                    placeholder="Password"
                />
                <input
                    name="passwordConfirmation"
                    type="password"
                    value={formData.passwordConfirmation}
                    onChange={handleChange}
                    placeholder="Password Confirmation"
                />
                <button type="submit">Create User</button>
            </form>
            {error && <div style={{ color: "red" }}>{error}</div>}
            {user && (
                <div>
                    <div>Created User...</div>
                    <div>name: {user.user_name}</div>
                    <div>id: {user.user_id}</div>
                </div>
            )}
        </div>
    );
}
