import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { ChevronsUpDownIcon, UserRoundIcon } from "lucide-react";
import { errorMessage } from "@/api";
import { userQueries } from "@/api/queries";
import { Button } from "@/components/ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useActingUser } from "./context";

// Top-bar menu to choose which user the app acts as (until there's a login).
export function ActingUserSwitcher() {
    const { userID, setUserID } = useActingUser();
    const users = useQuery(userQueries.list());

    const current = users.data?.find((user) => user.user_id === userID);
    // a remembered user can be gone after the database is reset
    const label = current ? current.user_name : userID ? "Unknown user" : "Choose a user";

    return (
        <DropdownMenu>
            <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm" className="max-w-44">
                    <UserRoundIcon />
                    <span className="truncate">{label}</span>
                    <ChevronsUpDownIcon className="text-muted-foreground" />
                </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="max-h-80 w-56 overflow-y-auto">
                <DropdownMenuLabel>Act as</DropdownMenuLabel>
                {users.isPending && (
                    <DropdownMenuItem disabled>Loading users...</DropdownMenuItem>
                )}
                {users.isError && (
                    <DropdownMenuItem disabled>
                        {errorMessage(users.error)}
                    </DropdownMenuItem>
                )}
                {users.isSuccess && users.data.length === 0 && (
                    <DropdownMenuItem disabled>No users yet</DropdownMenuItem>
                )}
                {users.isSuccess && (
                    <DropdownMenuRadioGroup value={userID} onValueChange={setUserID}>
                        {users.data.map((user) => (
                            <DropdownMenuRadioItem
                                key={user.user_id}
                                value={user.user_id}>
                                {user.user_name}
                            </DropdownMenuRadioItem>
                        ))}
                    </DropdownMenuRadioGroup>
                )}
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild>
                    {/* the users page creates users too: the way in on a first visit */}
                    <Link to="/users">
                        {users.data?.length === 0 ? "Create a user" : "Manage users"}
                    </Link>
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}
