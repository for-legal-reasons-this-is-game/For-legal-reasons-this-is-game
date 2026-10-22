import { useState } from "react";
import { CreateUser } from "./user/CreateUser";
import { ListUsers } from "./user/ListUsers";
import { CreateAccount } from "./account/CreateAccount";
import { CreateLedger } from "./ledger/CreateLedger";
import { ListLedgers } from "./ledger/ListLedgers";
import { AccountLookup } from "./account/AccountLookup";
import { PositionLookup } from "./account/PositionLookup";
import { LedgerLookup } from "./ledger/LedgerLookup";
import { CloseAccount } from "./account/CloseAccount";

function App() {
    const [userID, setUserID] = useState<string>("");
    const [createUser, setCreateUser] = useState(false);
    const [users, setUsers] = useState(false);
    const [createAccount, setCreateAccount] = useState(false);
    const [createLedger, setCreateLedger] = useState(false);
    const [ledgers, setLedgers] = useState(false);
    const [account, setAccount] = useState(false);
    const [accountPosition, setAccountPosition] = useState(false);
    const [ledger, setLedger] = useState(false);
    const [closeAccount, setCloseAccount] = useState(false);

    return (
        <div>
            {userID ? <h3>Hello, user _{userID}_</h3> : ""}
            <button
                style={{ background: createUser ? "green" : "grey" }}
                onClick={() => setCreateUser(!createUser)}>
                Create User
            </button>
            <br />
            <button
                style={{ background: users ? "green" : "grey" }}
                onClick={() => setUsers(!users)}>
                List Users
            </button>
            <br />
            <br />
            <button
                style={{ background: createAccount ? "green" : "grey" }}
                onClick={() => setCreateAccount(!createAccount)}>
                Create Account
            </button>
            <br />
            <button
                style={{ background: account ? "green" : "grey" }}
                onClick={() => setAccount(!account)}>
                Fetch Account
            </button>
            <br />
            <button
                style={{ background: accountPosition ? "green" : "grey" }}
                onClick={() => setAccountPosition(!accountPosition)}>
                Fetch Position
            </button>
            <br />
            <button
                style={{ background: closeAccount ? "green" : "grey" }}
                onClick={() => setCloseAccount(!closeAccount)}>
                Close Account
            </button>
            <br />
            <br />
            <button
                style={{ background: createLedger ? "green" : "grey" }}
                onClick={() => setCreateLedger(!createLedger)}>
                Create Ledger
            </button>
            <br />
            <button
                style={{ background: ledgers ? "green" : "grey" }}
                onClick={() => setLedgers(!ledgers)}>
                List Ledgers
            </button>
            <br />
            <button
                style={{ background: ledger ? "green" : "grey" }}
                onClick={() => setLedger(!ledger)}>
                Get Ledger
            </button>
            <br />
            {createUser && <CreateUser />}
            {users && <ListUsers setUserID={setUserID} />}
            {createAccount && <CreateAccount userID={userID} />}
            {account && <AccountLookup />}
            {accountPosition && <PositionLookup />}
            {createLedger && <CreateLedger />}
            {ledgers && <ListLedgers />}
            {ledger && <LedgerLookup />}
            {closeAccount && <CloseAccount />}
        </div>
    );
}

export default App;
