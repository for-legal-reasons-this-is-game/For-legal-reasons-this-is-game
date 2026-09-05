import { PostHog } from '../../posthog-core';
import { RemoteConfigResult } from '../../types';
import { ErrorTracking } from '@posthog/core';
export declare class ExceptionObserver {
    private _instance;
    private _rateLimiter;
    private _remoteEnabled;
    private _config;
    private _unwrapOnError;
    private _unwrapUnhandledRejection;
    private _unwrapConsoleError;
    constructor(instance: PostHog);
    private _requiredConfig;
    get isEnabled(): boolean;
    startIfEnabledOrStop(): void;
    private _loadScript;
    private _startCapturing;
    private _stopCapturing;
    onRemoteConfig(result: RemoteConfigResult): void;
    onConfigChange(): void;
    captureException(errorProperties: ErrorTracking.ErrorProperties): void;
}
