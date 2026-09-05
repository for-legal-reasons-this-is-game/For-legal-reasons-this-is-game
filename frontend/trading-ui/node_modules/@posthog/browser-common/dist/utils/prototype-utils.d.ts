interface NativeImplementationsCache {
    MutationObserver: typeof MutationObserver;
}
type BrowserWindow = Window & typeof globalThis;
export declare function getNativeImplementation<T extends keyof NativeImplementationsCache>(name: T, assignableWindow: BrowserWindow): NativeImplementationsCache[T];
export declare function getNativeMutationObserverImplementation(assignableWindow: BrowserWindow): typeof MutationObserver;
export {};
