import { useState } from "react";
import { CreateUser } from "./CreateUser";
import { ListUsers } from "./ListUsers";
import { CreateAccount } from "./CreateAccount";
import { CreateLedger } from "./CreateLedger";
import { Ledgers } from "./Ledgers";
import { Account } from "./Account";
import { Position } from "./Position";
import { EnableLedger } from "./EnableLedger";
import { Ledger } from "./Ledger";

function App() {
    const [createUser, setCreateUser] = useState(false);
    const [users, setUsers] = useState(false);
    const [createAccount, setCreateAccount] = useState(false);
    const [createLedger, setCreateLedger] = useState(false);
    const [ledgers, setLedgers] = useState(false);
    const [account, setAccount] = useState(false);
    const [accountPosition, setAccountPosition] = useState(false);
    const [enableLedger, setSetLedgerEnabled] = useState(false);
    const [ledger, setLedger] = useState(false);

    return (
        <div>
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
                style={{ background: enableLedger ? "green" : "grey" }}
                onClick={() => setSetLedgerEnabled(!enableLedger)}>
                Enable Ledger
            </button>
            <br />
            <button
                style={{ background: ledger ? "green" : "grey" }}
                onClick={() => setLedger(!ledger)}>
                Get Ledger
            </button>
            <br />
            {createUser && <CreateUser />}
            {users && <ListUsers />}
            {createAccount && <CreateAccount />}
            {account && <Account />}
            {accountPosition && <Position />}
            {createLedger && <CreateLedger />}
            {ledgers && <Ledgers />}
            {enableLedger && <EnableLedger />}
            {ledger && <Ledger />}
        </div>
    );
}

export default App;
