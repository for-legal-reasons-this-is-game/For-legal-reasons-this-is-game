import { type FormEvent, type ChangeEvent, useState } from "react";
import { type User } from "./types/User";

export function CreateUser() {
    const [formData, setFormData] = useState({
        name: "",
        email: "",
        password: "",
        passwordConfirmation: "",
    });
    const [creationRes, setCreationRes] = useState({
        user_id: "",
        user_name: "",
    });

    async function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        try {
            const res = await fetch(`http://localhost:8000/api/v1/users`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(formData),
            });
            const user: User = await res.json();
            setCreationRes(user);
            console.log(`Created User:\n name:${user.user_name}\n id:${user.user_id}`);
        } catch (error) {
            console.log(`ERROR: ${error}`);
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
                    value={formData.password}
                    onChange={handleChange}
                    placeholder="Password"
                />
                <input
                    name="passwordConfirmation"
                    value={formData.passwordConfirmation}
                    onChange={handleChange}
                    placeholder="Password Confirmation"
                />
                <button type="submit">Create User</button>
            </form>
            {creationRes.user_name && (
                <div>
                    Created User: name: {creationRes.user_name}
                    id: {creationRes.user_id}
                </div>
            )}
        </div>
    );
}
