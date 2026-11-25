import { useActingUser } from "@/app/acting-user/context";
import { EmptyState } from "@/components/EmptyState";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { AccountsTable } from "@/features/account/AccountsTable";
import { AddAccountDialog } from "@/features/account/AddAccountDialog";
import { OpenAccountDialog } from "@/features/account/OpenAccountDialog";
import { ChooseUserHint } from "@/features/user/ChooseUserHint";

const APP_NAME = "For legal reasons this is game";

export function PortfolioPage() {
    const { userID } = useActingUser();

    return (
        <>
            <title>{`Portfolio - ${APP_NAME}`}</title>
            <PageHeader
                title="Portfolio"
                description="Your accounts and what each can spend. Every account holds one asset; deposit play money to trade with it."
                actions={
                    userID && (
                        <>
                            <AddAccountDialog
                                userID={userID}
                                trigger={<Button variant="outline">Add by ID</Button>}
                            />
                            <OpenAccountDialog
                                userID={userID}
                                trigger={<Button>Open account</Button>}
                            />
                        </>
                    )
                }
            />
            {userID ? (
                <AccountsTable
                    userID={userID}
                    empty={
                        <EmptyState>
                            <p>
                                No accounts yet. Open one for each asset you want to
                                trade, for example one in BTC and one in USD for BTC-USD.
                            </p>
                            <OpenAccountDialog
                                userID={userID}
                                trigger={<Button className="mt-4">Open account</Button>}
                            />
                        </EmptyState>
                    }
                />
            ) : (
                <EmptyState>
                    <ChooseUserHint goal="to see their accounts" />
                </EmptyState>
            )}
        </>
    );
}
