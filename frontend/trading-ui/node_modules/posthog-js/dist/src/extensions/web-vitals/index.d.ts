import { PostHog } from '../../posthog-core';
import { RemoteConfigResult, SupportedWebVitalsMetrics } from '../../types';
export declare const DEFAULT_FLUSH_TO_CAPTURE_TIMEOUT_MILLISECONDS = 5000;
export declare const FIFTEEN_MINUTES_IN_MILLIS: number;
export declare class WebVitalsAutocapture {
    private readonly _instance;
    private _enabledServerSide;
    private _initialized;
    private _buffer;
    private _delayedFlushTimer;
    constructor(_instance: PostHog);
    private get _perfConfig();
    get allowedMetrics(): SupportedWebVitalsMetrics[];
    get flushToCaptureTimeoutMs(): number;
    get useAttribution(): boolean;
    get _maxAllowedValue(): number;
    get isEnabled(): boolean;
    startIfEnabled(): void;
    onRemoteConfig(result: RemoteConfigResult): void;
    private _loadScript;
    private _currentURL;
    private _flushToCapture;
    private _addToBuffer;
    private _startCapturing;
}
