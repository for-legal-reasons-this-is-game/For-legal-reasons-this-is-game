import { useId, useState, type FormEvent, type ReactNode } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api, errorMessage } from "@/api";
import { userQueries } from "@/api/queries";
import { useActingUser } from "@/app/acting-user/context";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

// The user menu fits about this many characters; the backend has no limit
const MAX_NAME = 40;

// Creates a user. With nobody chosen yet, the app starts acting as the new user;
// otherwise the toast offers to switch.
export function CreateUserDialog({ trigger }: { trigger: ReactNode }) {
    const [open, setOpen] = useState(false);
    const [name, setName] = useState("");
    const [problem, setProblem] = useState<string | null>(null);
    const id = useId();
    const { userID, setUserID } = useActingUser();
    const users = useQuery({ ...userQueries.list(), enabled: open });

    const queryClient = useQueryClient();
    const createUser = useMutation({
        mutationFn: (name: string) => api.users.create({ name }),
        onSuccess: (user) => {
            // show the new user in the lists right away, then check with the backend.
            // exact: only the list; ["users", id, "orders", ...] start with "users" too
            queryClient.setQueryData(userQueries.list().queryKey, (list) =>
                list ? [...list, user] : list,
            );
            queryClient.invalidateQueries({
                queryKey: userQueries.list().queryKey,
                exact: true,
            });
            if (!userID) {
                setUserID(user.user_id);
                toast.success("User created", {
                    description: `You're acting as ${user.user_name} now.`,
                });
            } else {
                toast.success("User created", {
                    description: user.user_name,
                    action: {
                        label: `Act as ${user.user_name}`,
                        onClick: () => setUserID(user.user_id),
                    },
                });
            }
            handleOpenChange(false);
        },
    });

    function handleOpenChange(next: boolean) {
        setOpen(next);
        if (!next) {
            setName("");
            setProblem(null);
            createUser.reset();
        }
    }

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        event.stopPropagation();
        const trimmed = name.trim();
        if (!trimmed) return setProblem("Give the user a name.");
        // The backend allows two users with one name, but the user menu shows only names,
        // so they couldn't be told apart there.
        const taken = users.data?.some(
            (user) => user.user_name.toLowerCase() === trimmed.toLowerCase(),
        );
        if (taken) return setProblem(`There's already a user called ${trimmed}.`);
        createUser.mutate(trimmed);
    }

    const message =
        problem ?? (createUser.isError ? errorMessage(createUser.error) : null);

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogTrigger asChild>{trigger}</DialogTrigger>
            <DialogContent>
                <form onSubmit={handleSubmit} noValidate className="grid gap-4">
                    <DialogHeader>
                        <DialogTitle>Create a user</DialogTitle>
                        <DialogDescription>
                            Until there's a login, anyone can act as any user. Each user
                            has their own accounts and orders.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="grid gap-2">
                        <Label htmlFor={`${id}-name`}>Name</Label>
                        <Input
                            id={`${id}-name`}
                            value={name}
                            maxLength={MAX_NAME}
                            onChange={(event) => {
                                setProblem(null);
                                setName(event.target.value);
                            }}
                            placeholder="For example: alice"
                            autoComplete="off"
                            aria-invalid={!!message}
                            aria-describedby={message ? `${id}-problem` : undefined}
                        />
                    </div>
                    {message && (
                        <p id={`${id}-problem`} role="alert" className="text-destructive">
                            {message}
                        </p>
                    )}
                    <DialogFooter>
                        <Button type="submit" disabled={createUser.isPending}>
                            {createUser.isPending ? "Creating..." : "Create user"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}
