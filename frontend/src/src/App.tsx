import { useState } from "react";
import { CreateUser } from "./CreateUser";
import { ListUsers } from "./ListUsers";
import { CreateAccount } from "./CreateAccount";
import { Ledgers } from "./Ledgers";
import { Account } from "./Account";

function App() {
    const [showCreateUser, setShowCreateUser] = useState(false);
    const [showUsers, setShowUsers] = useState(false);
    const [showCreateAccount, setShowCreateAccount] = useState(false);
    const [showLedgers, setShowLedgers] = useState(false);
    const [showAccount, setShowAccount] = useState(false);
    // const [showUser, setShowUser] = useState(false);
    // const [showAccountPosition, setAccountPosition] = useState(false);

    return (
        <div>
            <button onClick={() => setShowCreateUser(!showCreateUser)}>
                Create User
            </button>
            <br />
            <button onClick={() => setShowUsers(!showUsers)}>List Users</button>
            <br />
            <button onClick={() => setShowCreateAccount(!showCreateAccount)}>
                Create Account
            </button>
            <br />
            <button onClick={() => setShowLedgers(!showLedgers)}>List Ledgers</button>
            <br />
            <button onClick={() => setShowAccount(!showAccount)}>Fetch Account</button>
            {showCreateUser && <CreateUser />}
            {showUsers && <ListUsers />}
            {showCreateAccount && <CreateAccount />}
            {showLedgers && <Ledgers />}
            {showAccount && <Account />}
        </div>
    );
}

export default App;
