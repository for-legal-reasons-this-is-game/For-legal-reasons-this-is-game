/** A resource handle with idempotent, best-effort cleanup. */
export interface Disposable {
    /** Release resources owned by this object. */
    dispose(): void;
}
/** Invokes teardown at most once without awaiting Promise results. */
export declare function createDisposable(dispose: () => void): Disposable;
