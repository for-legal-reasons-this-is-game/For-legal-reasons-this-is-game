import { Component, type ErrorInfo, type ReactNode } from "react";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";

type Props = {
    // when this changes (another page was opened), a caught error is cleared
    resetKey: string;
    children: ReactNode;
};
type State = { error: Error | null; resetKey: string };

// New React: an error boundary catches an error thrown while the components inside it
// render, and shows a message instead of leaving a blank page. React only offers it as a
// class component (there's no hook for it): getDerivedStateFromError turns the error into
// state, componentDidCatch logs it, and render() shows either the message or the page.
// It catches bugs in a page and a page's code failing to download (see the lazy pages in
// app/App.tsx); failed requests are shown by each page itself.
export class ErrorBoundary extends Component<Props, State> {
    state: State = { error: null, resetKey: this.props.resetKey };

    static getDerivedStateFromError(error: Error): Partial<State> {
        return { error };
    }

    // another page was opened: show it instead of the last page's error
    static getDerivedStateFromProps(props: Props, state: State): Partial<State> | null {
        return props.resetKey !== state.resetKey
            ? { error: null, resetKey: props.resetKey }
            : null;
    }

    componentDidCatch(error: Error, info: ErrorInfo) {
        console.error(error, info.componentStack);
    }

    render() {
        const { error } = this.state;
        if (!error) return this.props.children;
        return (
            <div role="alert" className="max-w-[70ch]">
                <h1 className="text-2xl font-bold font-stretch-expanded">
                    This page couldn't be shown
                </h1>
                <p className="mt-1 text-muted-foreground">
                    Reload to try again. If it fails again, the page has a bug, and the
                    message below helps whoever fixes it.
                </p>
                <p className="mt-3 rounded-lg border bg-card p-3 text-sm break-words">
                    {error.message}
                </p>
                <div className="mt-4 flex flex-wrap gap-2">
                    <Button onClick={() => window.location.reload()}>
                        Reload the page
                    </Button>
                    <Button variant="outline" asChild>
                        <Link to="/markets">Go to markets</Link>
                    </Button>
                </div>
            </div>
        );
    }
}
