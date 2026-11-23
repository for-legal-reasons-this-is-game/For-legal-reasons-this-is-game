import { useState, type FormEvent, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { errorMessage } from "@/api";
import { accountQueries } from "@/api/queries";
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
import { rememberAccount } from "./known-accounts";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

// Remembers an account opened somewhere else (another browser, the old /dev UI), after
// checking that it exists and belongs to the user. `trigger` is the button that opens it.
export function AddAccountDialog({
    userID,
    trigger,
}: {
    userID: string;
    trigger: ReactNode;
}) {
    const [open, setOpen] = useState(false);
    const [accountID, setAccountID] = useState("");
    // a problem found in our own checks, as opposed to an error from the request
    const [problem, setProblem] = useState<string | null>(null);

    const queryClient = useQueryClient();
    // New TanStack: fetchQuery fetches through the cache (and fills it), so the account
    // doesn't have to be fetched again when the order form lists it
    const lookup = useMutation({
        mutationFn: (id: string) => queryClient.fetchQuery(accountQueries.detail(id)),
        onSuccess: (account) => {
            if (account.account_user_id !== userID) {
                setProblem(
                    "This account belongs to another user. To trade with it, switch user in the top bar.",
                );
            } else if (account.account_code_type !== 1) {
                setProblem(
                    "This is one of the exchange's own accounts, not a trading account.",
                );
            } else {
                rememberAccount(userID, account.account_id);
                toast.success("Account added", { description: account.account_name });
                setOpen(false);
            }
        },
    });

    function handleOpenChange(next: boolean) {
        setOpen(next);
        // start empty the next time it opens
        if (!next) {
            setAccountID("");
            setProblem(null);
            lookup.reset();
        }
    }

    function handleSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        // New React: the dialog is drawn in a portal (at the end of <body>), but its
        // events still bubble up through the components it's written inside. Without
        // this, its submit would also reach the order form it sits in.
        event.stopPropagation();
        const id = accountID.trim();
        if (!UUID.test(id)) {
            setProblem(
                "That isn't an account ID. It has 36 characters, like 0195f3a2-7c1d-7e4b-9a2f-3c4d5e6f7a8b.",
            );
            return;
        }
        lookup.mutate(id);
    }

    const message =
        problem ??
        (lookup.isError
            ? errorMessage(lookup.error, { 404: "There's no account with this ID." })
            : null);

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogTrigger asChild>{trigger}</DialogTrigger>
            <DialogContent>
                <form onSubmit={handleSubmit} className="grid gap-4">
                    <DialogHeader>
                        <DialogTitle>Add an account by ID</DialogTitle>
                        <DialogDescription>
                            Accounts opened in this browser are listed already. To trade
                            with one opened somewhere else, paste its ID.
                        </DialogDescription>
                    </DialogHeader>
                    <div className="grid gap-2">
                        <Label htmlFor="add-account-id">Account ID</Label>
                        <Input
                            id="add-account-id"
                            value={accountID}
                            onChange={(event) => {
                                setAccountID(event.target.value);
                                setProblem(null);
                            }}
                            autoComplete="off"
                            spellCheck={false}
                            aria-invalid={!!message}
                            aria-describedby={message ? "add-account-problem" : undefined}
                        />
                        {message && (
                            <p
                                id="add-account-problem"
                                role="alert"
                                className="text-destructive">
                                {message}
                            </p>
                        )}
                    </div>
                    <DialogFooter>
                        <Button type="submit" disabled={lookup.isPending}>
                            {lookup.isPending ? "Checking..." : "Add account"}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}
