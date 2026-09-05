import type { Extension } from './extensions/types';
import { PostHog } from './posthog-core';
import { DisplaySurveyOptions, Survey, SurveyCallback, SurveyRenderReason } from './posthog-surveys-types';
import { Properties, RemoteConfigResult } from './types';
import { SurveyEventReceiver } from './utils/survey-event-receiver';
export declare class PostHogSurveys implements Extension {
    private readonly _instance;
    private _isSurveysEnabled?;
    _surveyEventReceiver: SurveyEventReceiver | null;
    private _surveyManager;
    private _isInitializingSurveys;
    private _surveyCallbacks;
    private _getSurveysInFlightPromise;
    private _lastSurveyRefreshFailedAt;
    private get _config();
    constructor(_instance: PostHog);
    initialize(): void;
    onRemoteConfig(result: RemoteConfigResult): void;
    reset(): void;
    loadIfEnabled(): void;
    /** Helper to finalize survey initialization */
    private _completeSurveyInitialization;
    /** Helper to handle errors during survey loading */
    private _handleSurveyLoadError;
    /**
     * Register a callback that runs when surveys are initialized.
     * ### Usage:
     *
     *     posthog.onSurveysLoaded((surveys) => {
     *         // You can work with all surveys
     *         console.log('All available surveys:', surveys)
     *
     *         // Or get active matching surveys
     *         posthog.getActiveMatchingSurveys((activeMatchingSurveys) => {
     *             if (activeMatchingSurveys.length > 0) {
     *                 posthog.renderSurvey(activeMatchingSurveys[0].id, '#survey-container')
     *             }
     *         })
     *     })
     *
     * @param {Function} callback The callback function will be called when surveys are loaded or updated.
     *                           It receives the array of all surveys and a context object with error status.
     * @returns {Function} A function that can be called to unsubscribe the listener.
     */
    onSurveysLoaded(callback: SurveyCallback): () => void;
    getSurveys(callback: SurveyCallback, forceReload?: boolean): void;
    /**
     * Whether to kick off a background refresh of the cached definitions: the cache is stale, no
     * fetch is already in flight, and we're not backing off after a recent failure.
     */
    private _shouldBackgroundRefreshSurveys;
    /**
     * Whether the cached `$surveys` definitions have aged past their TTL. Returns false when no
     * timestamp is recorded (e.g. surveys injected directly in tests) so the cache stays valid.
     */
    private _isSurveyCacheStale;
    private _isSurveyRefreshBackingOff;
    /**
     * Marks a survey as seen for the current device, mirroring the local state the SDK records
     * when it shows or sends a survey itself.
     *
     * Use this when you display surveys through your own backend/integration (so the SDK never
     * captures the `survey shown`/`sent`/`dismissed` events) and still want PostHog's display
     * logic to honour the "already seen" and wait-period checks on subsequent page loads.
     *
     * Note: surveys configured to repeat (`schedule: 'always'` or event `repeatedActivation`)
     * intentionally bypass the seen check, so marking them as seen will not stop them showing.
     *
     * @param surveyId The ID of the survey to mark as seen.
     * @param options Optional settings. `iteration` is the survey's current iteration number, if any.
     */
    markSurveyAsSeen(surveyId: string, options?: {
        iteration?: number | null;
    }): void;
    /** Helper method to notify all registered callbacks */
    private _notifySurveyCallbacks;
    getActiveMatchingSurveys(callback: SurveyCallback, forceReload?: boolean): void;
    private _getSurveyById;
    private _checkSurveyEligibility;
    private _checkSurveyRenderability;
    canRenderSurvey(surveyId: string | Survey): SurveyRenderReason;
    canRenderSurveyAsync(surveyId: string, forceReload: boolean): Promise<SurveyRenderReason>;
    renderSurvey(surveyId: string | Survey, selector: string, properties?: Properties): void;
    displaySurvey(surveyId: string, options: DisplaySurveyOptions): void;
    cancelPendingSurvey(surveyId: string): void;
    handlePageUnload(): void;
}
