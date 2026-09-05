import { type Logger } from '@posthog/core';
import type { Client } from './client';
import type { Disposable } from './disposable';
import type { Extension } from './extension';
/** Shared setup and lifecycle registry for browser extension hosts. */
export declare class ExtensionRuntime implements Disposable {
    private readonly _logger;
    private readonly _client;
    private readonly _extensions;
    private _disposed;
    constructor(_logger: Logger, _client: Client);
    /** Reserves an extension name and sets it up with the host client adapter. */
    add(extension: Extension): Promise<void>;
    /** Releases every registered extension once in reverse registration order without waiting for pending setup. */
    dispose(): void;
    private _disposeExtension;
}
