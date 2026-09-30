import { CreateUser } from "./CreateUser";
import { ListUsers } from "./ListUsers";
import { CreateAccount } from "./CreateAccount";
import { Ledgers } from "./Ledgers";

function App() {
    return (
        <div>
            <CreateUser />
            <ListUsers />
            <CreateAccount />
            <Ledgers />
        </div>
    );
}

export default App;
