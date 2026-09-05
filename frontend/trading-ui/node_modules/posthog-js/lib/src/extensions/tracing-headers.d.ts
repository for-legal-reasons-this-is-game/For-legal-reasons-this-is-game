import { PostHog } from '../posthog-core';
import type { Extension } from './types';
export declare class TracingHeaders implements Extension {
    private readonly _instance;
    private _restoreXHRPatch;
    private _restoreFetchPatch;
    private _hostnamesForPatch;
    constructor(_instance: PostHog);
    initialize(): void;
    private _loadScript;
    private _getConfiguredHostnames;
    private _syncHostnamesForPatch;
    private _stopCapturing;
    startIfEnabledOrStop(): void;
    private _startCapturing;
}
