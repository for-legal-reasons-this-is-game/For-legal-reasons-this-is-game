import { useQuery } from "@tanstack/react-query";
import { CheckIcon } from "lucide-react";
import { toast } from "sonner";
import { errorMessage } from "@/api";
import { userQueries } from "@/api/queries";
import { useActingUser } from "@/app/acting-user/context";
import { CopyButton } from "@/components/CopyButton";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from "@/components/ui/table";
import type { User } from "@/types/User";

// on phones the ID moves under the name
const ID_COLUMN = "hidden sm:table-cell";

function matches(user: User, search: string): boolean {
    const text = search.trim().toLowerCase();
    return (
        !text ||
        user.user_name.toLowerCase().includes(text) ||
        user.user_id.toLowerCase().includes(text)
    );
}

function UserRow({ user }: { user: User }) {
    const { userID, setUserID } = useActingUser();
    const current = user.user_id === userID;

    return (
        <TableRow>
            <TableCell className="max-w-0 sm:max-w-none">
                <div className="font-semibold">{user.user_name}</div>
                <div className="flex items-center gap-1 text-xs text-muted-foreground sm:hidden">
                    <span className="truncate">{user.user_id}</span>
                    <CopyButton value={user.user_id} what="user ID" />
                </div>
            </TableCell>
            <TableCell className={ID_COLUMN}>
                <span className="inline-flex items-center gap-1 text-muted-foreground">
                    {user.user_id}
                    <CopyButton value={user.user_id} what="user ID" />
                </span>
            </TableCell>
            <TableCell className="text-right">
                {current ? (
                    <span className="inline-flex items-center gap-1 text-sm text-muted-foreground">
                        <CheckIcon className="size-4" />
                        Acting as
                    </span>
                ) : (
                    <Button
                        variant="outline"
                        size="xs"
                        onClick={() => {
                            setUserID(user.user_id);
                            toast.success(`Acting as ${user.user_name}`);
                        }}>
                        Act as
                    </Button>
                )}
            </TableCell>
        </TableRow>
    );
}

// Everyone, by name, filtered by `search` (part of a name or an ID). The app acts as the
// user chosen here or in the top bar.
export function UsersTable({ search }: { search: string }) {
    const users = useQuery(userQueries.list());

    if (users.isError) {
        return (
            <p role="alert" className="p-2 text-destructive">
                Couldn't load the users: {errorMessage(users.error)}
            </p>
        );
    }

    const shown = (users.data ?? [])
        .filter((user) => matches(user, search))
        .sort(
            (a, b) =>
                a.user_name.localeCompare(b.user_name) ||
                a.user_id.localeCompare(b.user_id),
        );

    if (users.isSuccess && shown.length === 0) {
        return (
            <p className="p-2 text-muted-foreground">
                No user's name or ID contains “{search.trim()}”.
            </p>
        );
    }

    return (
        <Table className="font-stretch-semi-condensed">
            <TableHeader>
                <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead className={ID_COLUMN}>ID</TableHead>
                    <TableHead>
                        <span className="sr-only">Act as</span>
                    </TableHead>
                </TableRow>
            </TableHeader>
            <TableBody>
                {users.isPending
                    ? [0, 1, 2].map((row) => (
                          <TableRow key={row}>
                              {["", ID_COLUMN, ""].map((className, cell) => (
                                  <TableCell key={cell} className={className}>
                                      <Skeleton className="h-4 w-full" />
                                  </TableCell>
                              ))}
                          </TableRow>
                      ))
                    : shown.map((user) => <UserRow key={user.user_id} user={user} />)}
            </TableBody>
        </Table>
    );
}
