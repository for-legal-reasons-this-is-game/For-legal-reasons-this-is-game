import { useState } from "react";

import { CreateUser } from "./user/CreateUser";
import { ListUsers } from "./user/ListUsers";

import { CreateAccount } from "./account/CreateAccount";
import { AccountLookup } from "./account/AccountLookup";

import { CreateLedger } from "./ledger/CreateLedger";
import { ListLedgers } from "./ledger/ListLedgers";
import { LedgerLookup } from "./ledger/LedgerLookup";

import { MarketLookup } from "./market/MarketLookup";
import { ListMarkets } from "./market/ListMarkets";

function App() {
    const [userID, setUserID] = useState<string>("");
    const [createUser, setCreateUser] = useState(false);
    const [users, setUsers] = useState(false);
    const [createAccount, setCreateAccount] = useState(false);
    const [createLedger, setCreateLedger] = useState(false);
    const [ledgers, setLedgers] = useState(false);
    const [account, setAccount] = useState(false);
    const [ledger, setLedger] = useState(false);
    const [marketLookup, setMarketLookup] = useState(false);
    const [listMarkets, setListMarkets] = useState(false);

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
            <br />
            <button
                style={{ background: marketLookup ? "green" : "grey" }}
                onClick={() => setMarketLookup(!marketLookup)}>
                Market Lookup
            </button>
            <br />
            <button
                style={{ background: listMarkets ? "green" : "grey" }}
                onClick={() => setListMarkets(!listMarkets)}>
                List Markets
            </button>
            <br />
            {createUser && <CreateUser />}
            {users && <ListUsers setUserID={setUserID} />}
            {createAccount && <CreateAccount userID={userID} />}
            {account && <AccountLookup />}
            {createLedger && <CreateLedger />}
            {ledgers && <ListLedgers />}
            {ledger && <LedgerLookup />}
            {marketLookup && <MarketLookup />}
            {listMarkets && <ListMarkets />}
        </div>
    );
}

export default App;
