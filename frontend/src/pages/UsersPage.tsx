import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { SearchIcon } from "lucide-react";
import { userQueries } from "@/api/queries";
import { EmptyState } from "@/components/EmptyState";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { CreateUserDialog } from "@/features/user/CreateUserDialog";
import { UsersTable } from "@/features/user/UsersTable";

const APP_NAME = "For legal reasons this is game";

export function UsersPage() {
    const users = useQuery(userQueries.list());
    const [search, setSearch] = useState("");

    return (
        <>
            <title>{`Users - ${APP_NAME}`}</title>
            <PageHeader
                title="Users"
                description="Everyone on the exchange. Until there's a login, the app acts as one of them: choose who here or in the top bar."
                actions={<CreateUserDialog trigger={<Button>Create user</Button>} />}
            />
            {users.isSuccess && users.data.length === 0 ? (
                <EmptyState>
                    <p>No users yet. Create the first one, and the app acts as them.</p>
                    <CreateUserDialog
                        trigger={<Button className="mt-4">Create user</Button>}
                    />
                </EmptyState>
            ) : (
                <div className="max-w-4xl rounded-2xl border bg-card p-2">
                    <div className="relative p-2">
                        <SearchIcon
                            aria-hidden
                            className="pointer-events-none absolute top-1/2 left-4.5 size-4 -translate-y-1/2 text-muted-foreground"
                        />
                        <Input
                            type="search"
                            value={search}
                            onChange={(event) => setSearch(event.target.value)}
                            placeholder="Search by name or ID"
                            aria-label="Search users"
                            className="max-w-sm pl-8"
                        />
                    </div>
                    <UsersTable search={search} />
                </div>
            )}
        </>
    );
}
