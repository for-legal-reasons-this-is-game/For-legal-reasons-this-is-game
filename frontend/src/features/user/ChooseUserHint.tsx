import { Link } from "react-router";

// What a page says while nobody is chosen in the top bar (there's no login yet). A first
// visit has no users at all, so it also links to creating one. `goal` finishes the
// sentence: "to see their orders".
export function ChooseUserHint({ goal }: { goal: string }) {
    return (
        <>
            Choose who you're acting as in the top bar {goal}, or{" "}
            <Link to="/users" className="text-primary underline-offset-4 hover:underline">
                create a user
            </Link>
            .
        </>
    );
}
