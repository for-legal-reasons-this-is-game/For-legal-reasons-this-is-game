import { PostHogConfig, Properties } from './types';
/**
 * PostHog Persistence Object
 * @constructor
 */
export declare class PostHogPersistence {
    private _config;
    props: Properties;
    private _storage;
    private _campaign_params_saved;
    private readonly _name;
    _disabled: boolean | undefined;
    private _secure;
    private _expire_days;
    private _default_expiry;
    private _cross_subdomain;
    private _slotState;
    private _splitStorageEligible;
    private _splitStorage;
    private readonly _ownsSplitStorage;
    private _pendingSaveTimer;
    /**
     * @param {PostHogConfig} config initial PostHog configuration
     * @param {boolean=} isDisabled should persistence be disabled (e.g. because of consent management)
     */
    constructor(config: PostHogConfig, isDisabled?: boolean, ownsSplitStorage?: boolean);
    private _saveDebounceMs;
    /**
     * Returns whether persistence is disabled. Only available in SDKs > 1.257.1. Do not use on extensions, otherwise
     * it'll break backwards compatibility for any version before 1.257.1.
     */
    isDisabled?(): boolean;
    private _buildStorage;
    private _groupEntryName;
    private _resolveSplitStorage;
    /**
     * Check if the feature flag cache is stale based on the configured TTL.
     * @param ttl Optional TTL override (uses config value if not provided)
     * @internal
     */
    _isFeatureFlagCacheStale(ttl?: number): boolean;
    properties(): Properties;
    load(): void;
    private _loadGroupEntries;
    private _mainCarriesGroupKey;
    private _groupEntryIsStale;
    /**
     * Refresh a single key from on-disk storage into `this.props` without
     * touching the rest. Used by `SessionIdManager` on the cross-tab idle
     * path so we can pick up a sibling tab's SESSION_ID write without
     * either:
     *  - flushing our own (potentially stale) whole-props blob to storage
     *    via `flush()`, which would clobber the sibling's write, or
     *  - replacing all of `props` via `load()`, which would discard any
     *    in-memory writes that haven't yet been debounced to storage.
     */
    refreshKey(prop: string): void;
    /**
     * NOTE: Saving frequently causes issues with Recordings and Consent Management Platform (CMP) tools which
     * observe cookie changes, and modify their UI, often causing infinite loops.
     * As such callers of this should ideally check that the data has changed beforehand
     */
    save(): void;
    /**
     * Force any pending debounced save to land in storage immediately.
     * No-op when there is no pending timer — crucially, this means the
     * `beforeunload` / `pagehide` listeners installed in the constructor
     * cannot accidentally resurrect a storage entry that `remove()` or
     * `clear()` just deleted. Without this guard, the listener would
     * call `_writeNow()` and write the in-memory `props` (now `{}`) back
     * to storage, breaking `posthog.reset()` / opt-out flows.
     */
    flush(): void;
    private _writeNow;
    private _writeNowSplit;
    private _partitionProps;
    private _entryFingerprint;
    private _writeEntry;
    remove({ keepGroupEntries }?: {
        keepGroupEntries?: boolean;
    }): void;
    clear(): void;
    /**
     * @param {Object} props
     * @param {*=} default_value
     * @param {number=} days
     */
    register_once(props: Properties, default_value: any, days?: number): boolean;
    /**
     * @param {Object} props
     * @param {number=} days
     */
    register(props: Properties, days?: number): boolean;
    unregister(prop: string): void;
    update_campaign_params(): void;
    update_search_keyword(): void;
    update_referrer_info(): void;
    set_initial_person_info(): void;
    get_initial_props(): Properties;
    safe_merge(props: Properties): Properties;
    update_config(config: PostHogConfig, oldConfig: PostHogConfig, isDisabled?: boolean): void;
    set_disabled(disabled: boolean): void;
    set_cross_subdomain(cross_subdomain: boolean): void;
    set_secure(secure: boolean): void;
    set_event_timer(event_name: string, timestamp: number): void;
    remove_event_timer(event_name: string): number;
    get_property(prop: string): any;
    set_property(prop: string, to: any): void;
    private _setProp;
    private _deleteProp;
    private _markGroupDirty;
    private _slotWriteState;
}
