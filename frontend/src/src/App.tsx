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
            <button onClick={() => setCreateUser(!createUser)}>Create User</button>
            <br />
            <button onClick={() => setUsers(!users)}>List Users</button>
            <br />
            <br />
            <button onClick={() => setCreateAccount(!createAccount)}>
                Create Account
            </button>
            <br />
            <button onClick={() => setAccount(!account)}>Fetch Account</button>
            <br />
            <button onClick={() => setAccountPosition(!accountPosition)}>
                Fetch Position
            </button>
            <br />
            <br />
            <button onClick={() => setCreateLedger(!createLedger)}>Create Ledger</button>
            <br />
            <button onClick={() => setLedgers(!ledgers)}>List Ledgers</button>
            <br />
            <button onClick={() => setSetLedgerEnabled(!enableLedger)}>
                Enable Ledger
            </button>
            <br />
            <button onClick={() => setLedger(!ledger)}>Get Ledger</button>
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
