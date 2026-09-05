import type { Extension } from './extensions/types';
import { PostHog } from './posthog-core';
import { CaptureResult, RemoteConfigResult, SiteApp, SiteAppGlobals, SiteAppLoader } from './types';
export declare class SiteApps implements Extension {
    private _instance;
    apps: Record<string, SiteApp>;
    private _stopBuffering?;
    private _bufferedInvocations;
    private _siteAppElementPatchCount;
    private _restoreSiteAppElementPatches?;
    constructor(_instance: PostHog);
    get isEnabled(): boolean;
    private _eventCollector;
    get siteAppLoaders(): SiteAppLoader[] | undefined;
    initialize(): void;
    globalsForEvent(event: CaptureResult): SiteAppGlobals;
    private _prepareElementForSiteApp;
    private _patchSiteAppElementInsertionMethods;
    private _releaseSiteAppElementPatches;
    private _runWithPreparedSiteAppElements;
    setupSiteApp(loader: SiteAppLoader): void;
    private _setupSiteApps;
    private _onCapturedEvent;
    onRemoteConfig(result: RemoteConfigResult): void;
}
