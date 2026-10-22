import { type FormEvent, type ChangeEvent, useState } from "react";
import { type AccountPayload } from "../types/AccountPayload";
import { api, errorMessage } from "../api";
import { useMutation } from "@tanstack/react-query";

export function CreateAccount({ userID }: { userID: string }) {
    // TODO: put CreateAccount in each User in User's list
    // TODO: To make a parent User component that has Account control, maybe create
    //      ListAccounts endpoint, takes UserID, returns that user's accounts
    const [formData, setFormData] = useState<AccountPayload>({
        name: "",
        ledger_symbol: "",
    });
    const [validationError, setValidationError] = useState<string | null>(null);

    const { isPending, isError, isSuccess, data, error, mutate } = useMutation({
        mutationFn: (payload: AccountPayload) => api.users.createAccount(userID, payload),
    });

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();

        if (!formData.name.trim() || !formData.ledger_symbol.trim() || !userID) {
            setValidationError(
                "Empty field or no userID set. click ListUsers to set userID",
            );
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
            <h3>Create Account</h3>
            <form onSubmit={handleSubmit}>
                <input
                    name="name"
                    value={formData.name}
                    onChange={handleChange}
                    placeholder="Account Name"
                />
                <input
                    name="ledger_symbol"
                    value={formData.ledger_symbol}
                    onChange={handleChange}
                    placeholder="ledger_symbol"
                />
                <button disabled={isPending} type="submit">
                    Create Account
                </button>
            </form>

            {validationError && <div style={{ color: "red" }}>{validationError}</div>}
            {isError && <div style={{ color: "red" }}>{errorMessage(error)}</div>}
            {isSuccess && (
                <div>
                    <div>Created Account: </div>
                    <div>Acc ID: {data.account_id}</div>
                    <div>Acc Name: {data.account_name}</div>
                    <div>Ledger ID: {data.account_ledger_id}</div>
                    <div>Code Type: {data.account_code_type}</div>
                    <div>Status: {data.account_status}</div>
                    <div>User ID: {data.account_user_id}</div>
                </div>
            )}
        </div>
    );
}
