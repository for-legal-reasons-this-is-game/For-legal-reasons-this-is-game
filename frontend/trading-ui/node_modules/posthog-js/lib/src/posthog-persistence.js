"use strict";
/* eslint camelcase: "off" */
var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
var __read = (this && this.__read) || function (o, n) {
    var m = typeof Symbol === "function" && o[Symbol.iterator];
    if (!m) return o;
    var i = m.call(o), r, ar = [], e;
    try {
        while ((n === void 0 || n-- > 0) && !(r = i.next()).done) ar.push(r.value);
    }
    catch (error) { e = { error: error }; }
    finally {
        try {
            if (r && !r.done && (m = i["return"])) m.call(i);
        }
        finally { if (e) throw e.error; }
    }
    return ar;
};
var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PostHogPersistence = void 0;
var general_utils_1 = require("@posthog/browser-common/utils/general-utils");
var storage_1 = require("./storage");
var globals_1 = require("@posthog/browser-common/utils/globals");
var constants_1 = require("./constants");
var persistence_key_policy_1 = require("./persistence-key-policy");
// The "freshness" key each split group stamps when its server payload is
// (re)loaded. A group entry carrying an older timestamp than the main blob is a
// stale orphan (a gate-off / older-SDK tab wrote a fresher payload back to main)
// and must not win on load. Groups without an entry here have no freshness
// signal, so the group entry wins by default (the migrated-forward home).
var VOLATILE_FINGERPRINT_PLACEHOLDER = '__volatile__';
// Both freshness keys are volatile (their on-disk value lags the last content
// change). That is safe: when only the stamp moved, both the group entry and the
// main blob hold identical content, so whichever side wins still produces the
// same values. A mixed-fleet race at most causes one extra /flags cycle.
var GROUP_FRESHNESS_KEY = {
    flags: constants_1.PERSISTENCE_FEATURE_FLAG_EVALUATED_AT,
    surveys: constants_1.SURVEYS_LOADED_AT,
};
var core_1 = require("@posthog/core");
var event_utils_1 = require("@posthog/browser-common/utils/event-utils");
var logger_1 = require("@posthog/browser-common/utils/logger");
var core_2 = require("@posthog/core");
var CASE_INSENSITIVE_PERSISTENCE_TYPES = [
    'cookie',
    'localstorage',
    'localstorage+cookie',
    'sessionstorage',
    'memory',
];
var parseName = function (config) {
    var token = '';
    if (config['token']) {
        token = config['token'].replace(/\+/g, 'PL').replace(/\//g, 'SL').replace(/=/g, 'EQ');
    }
    if (config['persistence_name']) {
        return 'ph_' + config['persistence_name'];
    }
    else {
        return 'ph_' + token + '_posthog';
    }
};
// Fingerprint slot for the main persistence entry. Group entries (`flags`)
// use their group name as the slot. See `_writeEntry`.
var MAIN_STORAGE_SLOT = 'main';
var isArrayContentsEqual = function (arr1, arr2) {
    if (arr1.length !== arr2.length) {
        return false;
    }
    var sortedArr1 = __spreadArray([], __read(arr1), false).sort();
    var sortedArr2 = __spreadArray([], __read(arr2), false).sort();
    return sortedArr1.every(function (item, index) { return item === sortedArr2[index]; });
};
/**
 * PostHog Persistence Object
 * @constructor
 */
var PostHogPersistence = /** @class */ (function () {
    /**
     * @param {PostHogConfig} config initial PostHog configuration
     * @param {boolean=} isDisabled should persistence be disabled (e.g. because of consent management)
     */
    function PostHogPersistence(config, isDisabled, ownsSplitStorage) {
        if (ownsSplitStorage === void 0) { ownsSplitStorage = true; }
        var _this = this;
        // Per-storage-entry write bookkeeping, keyed by slot (`main` plus each group
        // name): no-op-rejection fingerprint, per-group dirty flag, and on-disk
        // materialization. Reset wholesale on remove()/clear() so a save after
        // remove always lands. See `SlotWriteState`.
        this._slotState = {};
        // Whether the resolved storage backend can host the split (localStorage /
        // localStorage+cookie). Set by `_buildStorage`.
        this._splitStorageEligible = false;
        // Whether flag config is stored in their own entries this session:
        // backend-eligible AND `split_storage` enabled.
        // Re-resolved on every `update_config` (backend rebuild or a runtime flag flip).
        this._splitStorage = false;
        this._config = config;
        this._ownsSplitStorage = ownsSplitStorage;
        this.props = {};
        this._campaign_params_saved = false;
        this._name = parseName(config);
        this._storage = this._buildStorage(config);
        this._splitStorage = this._resolveSplitStorage(config);
        this.load();
        if (config.debug) {
            logger_1.logger.info('Persistence loaded', config['persistence'], __assign({}, this.props));
        }
        this.update_config(config, config, isDisabled);
        this.save();
        // Install unload flush listeners unconditionally. They are a no-op
        // when no debounced write is pending (see `flush()`), so it is safe
        // to install even when `persistence_save_debounce_ms` is 0 at
        // construction. Crucially this also handles `posthog.set_config({
        // persistence_save_debounce_ms: 250 })` enabling debounce later —
        // we'd otherwise miss the listener install and lose pending writes
        // on close.
        if (globals_1.window) {
            var flush = function () { return _this.flush(); };
            (0, general_utils_1.addEventListener)(globals_1.window, 'beforeunload', flush, { capture: false });
            (0, general_utils_1.addEventListener)(globals_1.window, 'pagehide', flush, { capture: false });
        }
    }
    PostHogPersistence.prototype._saveDebounceMs = function () {
        var _a;
        var value = (_a = this._config) === null || _a === void 0 ? void 0 : _a.persistence_save_debounce_ms;
        return (0, core_1.isNumber)(value) && value > 0 ? value : 0;
    };
    /**
     * Returns whether persistence is disabled. Only available in SDKs > 1.257.1. Do not use on extensions, otherwise
     * it'll break backwards compatibility for any version before 1.257.1.
     */
    PostHogPersistence.prototype.isDisabled = function () {
        return !!this._disabled;
    };
    PostHogPersistence.prototype._buildStorage = function (config) {
        if (CASE_INSENSITIVE_PERSISTENCE_TYPES.indexOf(config['persistence'].toLowerCase()) === -1) {
            logger_1.logger.critical('Unknown persistence type ' + config['persistence'] + '; falling back to localStorage+cookie');
            config['persistence'] = 'localStorage+cookie';
        }
        // Create this before hand to avoid creating it multiple times
        // Creating it inside each individual condition below is too complicated and will break backwards compatibility
        // so create it once for this specific config and use it if necessary
        var localPlusCookieStore = (0, storage_1.createLocalPlusCookieStore)(config['cookie_persisted_properties'] || [], config['__preview_cookie_wins_on_conflict'] || false);
        var store;
        // The flag split is only meaningful on a localStorage-backed
        // store: it is the one that broadcasts large cross-tab `storage` events.
        // cookie can't hold the cluster, memory/sessionStorage don't broadcast.
        var splitEligible = false;
        // We handle storage type in a case-insensitive way for backwards compatibility
        var storage_type = config['persistence'].toLowerCase();
        if (storage_type === 'localstorage' && storage_1.localStore._is_supported()) {
            store = storage_1.localStore;
            splitEligible = true;
        }
        else if (storage_type === 'localstorage+cookie' && localPlusCookieStore._is_supported()) {
            store = localPlusCookieStore;
            splitEligible = true;
        }
        else if (storage_type === 'sessionstorage' && storage_1.sessionStore._is_supported()) {
            store = storage_1.sessionStore;
        }
        else if (storage_type === 'memory') {
            store = storage_1.memoryStore;
        }
        else if (storage_type === 'cookie') {
            store = storage_1.cookieStore;
        }
        else if (localPlusCookieStore._is_supported()) {
            // selected storage type wasn't supported, fallback to 'localstorage+cookie' if possible
            store = localPlusCookieStore;
            splitEligible = true;
        }
        else {
            store = storage_1.cookieStore;
        }
        this._splitStorageEligible = splitEligible;
        return store;
    };
    PostHogPersistence.prototype._groupEntryName = function (group) {
        return "".concat(this._name, "__").concat(group);
    };
    // The split is on only when the resolved backend can host it (localStorage /
    // localStorage+cookie, set by `_buildStorage` into `_splitStorageEligible`)
    // AND the config opts in. Resolved here so the constructor and the runtime
    // `update_config` toggle can never disagree about whether the split is active.
    PostHogPersistence.prototype._resolveSplitStorage = function (config) {
        return this._splitStorageEligible && !!config['split_storage'];
    };
    /**
     * Check if the feature flag cache is stale based on the configured TTL.
     * @param ttl Optional TTL override (uses config value if not provided)
     * @internal
     */
    PostHogPersistence.prototype._isFeatureFlagCacheStale = function (ttl) {
        var effectiveTtl = ttl !== null && ttl !== void 0 ? ttl : this._config.feature_flag_cache_ttl_ms;
        if (!effectiveTtl || effectiveTtl <= 0) {
            return false;
        }
        var evaluatedAt = this.props[constants_1.PERSISTENCE_FEATURE_FLAG_EVALUATED_AT];
        // If evaluatedAt is missing or not a numeric timestamp, consider cache stale.
        // This handles SDK upgrades where old cached flags lack evaluatedAt.
        if (!evaluatedAt || typeof evaluatedAt !== 'number') {
            return true;
        }
        return Date.now() - evaluatedAt > effectiveTtl;
    };
    PostHogPersistence.prototype.properties = function () {
        var _this = this;
        var p = {};
        (0, general_utils_1.each)(this.props, function (v, k) {
            var _a, _b;
            var policy = (0, persistence_key_policy_1.getPersistenceKeyPolicy)(k);
            if ((policy === null || policy === void 0 ? void 0 : policy.exposure) === 'derived') {
                var shouldSkip = k === constants_1.ENABLED_FEATURE_FLAGS ? function () { return _this._isFeatureFlagCacheStale(); } : function () { return false; };
                if ((_a = policy.shouldSkipFromEventProperties) === null || _a === void 0 ? void 0 : _a.call(policy, v, shouldSkip)) {
                    return;
                }
                if (policy.transformToEventProperties) {
                    (0, general_utils_1.extend)(p, policy.transformToEventProperties(v));
                }
            }
            else if (!policy || policy.exposure === 'event') {
                if ((_b = policy === null || policy === void 0 ? void 0 : policy.shouldSkipFromEventProperties) === null || _b === void 0 ? void 0 : _b.call(policy, v, function () { return false; })) {
                    return;
                }
                // Unknown keys are treated as user-defined super properties and remain event-visible.
                p[k] = v;
            }
        });
        return p;
    };
    PostHogPersistence.prototype.load = function () {
        if (this._disabled) {
            return;
        }
        var entry = this._storage._parse(this._name);
        if (entry) {
            this.props = (0, general_utils_1.extend)({}, entry);
        }
        if (this._splitStorage) {
            this._loadGroupEntries();
        }
    };
    // Merge each group entry over `props`, which already holds the main blob.
    // On a first upgrade the main blob may still carry the old flag
    // values; a present group entry wins, so we resolve
    // `props[key] = group[key] ?? main[key]` in a single pass (the "check the
    // old key once" migration). The first `save()` then strips the keys from
    // the main blob. Group entries are localStorage-only (read via `localStore`
    // directly, never the cookie or the localPlusCookie re-write-on-parse path).
    PostHogPersistence.prototype._loadGroupEntries = function () {
        var e_1, _a;
        try {
            for (var PERSISTENCE_STORAGE_GROUPS_1 = __values(persistence_key_policy_1.PERSISTENCE_STORAGE_GROUPS), PERSISTENCE_STORAGE_GROUPS_1_1 = PERSISTENCE_STORAGE_GROUPS_1.next(); !PERSISTENCE_STORAGE_GROUPS_1_1.done; PERSISTENCE_STORAGE_GROUPS_1_1 = PERSISTENCE_STORAGE_GROUPS_1.next()) {
                var group = PERSISTENCE_STORAGE_GROUPS_1_1.value;
                // `localStore._parse` returns `{}` (not null) for a missing key, so
                // gate on real content: an empty/absent entry is not "persisted" and
                // must not be tracked, or `_writeNowSplit` would re-create it as `{}`.
                var groupEntry = storage_1.localStore._parse(this._groupEntryName(group));
                if (groupEntry && !(0, core_2.isEmptyObject)(groupEntry)) {
                    var state = this._slotWriteState(group);
                    state.persisted = true;
                    // Seed the no-op fingerprint with the snapshot we just read, so the
                    // first frequent main-blob save at startup (before fresh flags
                    // return from the network) recognises an unchanged flag entry and
                    // neither re-serializes nor re-broadcasts it to every open tab.
                    // Only safe when the main blob carries no key for this group: a
                    // leftover (partial migration, or a stale tab that wrote a flag key
                    // back to main) makes the partitioned payload differ from what is on
                    // disk, so the entry must still be written. Leaving it unseeded then
                    // lets the first save's fingerprint check write the merged payload
                    // through — completing the migration / healing the orphan. The
                    // `_writeEntry` group fast-path skips on `!dirty && fingerprint set`,
                    // which would otherwise short-circuit that write before the
                    // fingerprint is even compared.
                    if (!this._mainCarriesGroupKey(group)) {
                        state.fingerprint = this._entryFingerprint(groupEntry, group);
                    }
                    // The group entry is normally the migrated-forward home and wins
                    // over the main blob. The exception: a group that stamps a
                    // freshness timestamp (flags: $feature_flag_evaluated_at) lets us
                    // detect when a gate-off / older-SDK tab wrote a fresher payload
                    // back into the main blob — then we keep the main blob and let the
                    // next save heal the group entry. With no timestamp on either side
                    // (migration leftover) the group wins.
                    if (!this._groupEntryIsStale(group, groupEntry)) {
                        (0, general_utils_1.extend)(this.props, groupEntry);
                    }
                }
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (PERSISTENCE_STORAGE_GROUPS_1_1 && !PERSISTENCE_STORAGE_GROUPS_1_1.done && (_a = PERSISTENCE_STORAGE_GROUPS_1.return)) _a.call(PERSISTENCE_STORAGE_GROUPS_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
    };
    // True when the already-loaded main blob still holds a key belonging to this
    // group — a migration leftover the next save must fold into the group entry.
    // Checked before the group entry is merged in, so it sees only the main blob's
    // own keys (sibling groups carry a different storageGroup and never match).
    PostHogPersistence.prototype._mainCarriesGroupKey = function (group) {
        return Object.keys(this.props).some(function (key) { var _a; return ((_a = (0, persistence_key_policy_1.getPersistenceKeyPolicy)(key)) === null || _a === void 0 ? void 0 : _a.storageGroup) === group; });
    };
    PostHogPersistence.prototype._groupEntryIsStale = function (group, groupEntry) {
        var freshnessKey = GROUP_FRESHNESS_KEY[group];
        if (!freshnessKey) {
            return false;
        }
        var groupLoadedAt = groupEntry[freshnessKey];
        var mainLoadedAt = this.props[freshnessKey];
        return (0, core_1.isNumber)(groupLoadedAt) && (0, core_1.isNumber)(mainLoadedAt) && mainLoadedAt > groupLoadedAt;
    };
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
    PostHogPersistence.prototype.refreshKey = function (prop) {
        var _a;
        if (this._disabled) {
            return;
        }
        var group = this._splitStorage ? (_a = (0, persistence_key_policy_1.getPersistenceKeyPolicy)(prop)) === null || _a === void 0 ? void 0 : _a.storageGroup : undefined;
        var entry = group ? storage_1.localStore._parse(this._groupEntryName(group)) : this._storage._parse(this._name);
        if (entry && prop in entry) {
            this._setProp(prop, entry[prop]);
            return;
        }
        // A grouped key that has not migrated yet still lives in the main blob;
        // check there once before concluding a sibling removed it.
        if (group) {
            var mainEntry = this._storage._parse(this._name);
            if (mainEntry && prop in mainEntry) {
                this._setProp(prop, mainEntry[prop]);
                return;
            }
        }
        this._deleteProp(prop);
    };
    /**
     * NOTE: Saving frequently causes issues with Recordings and Consent Management Platform (CMP) tools which
     * observe cookie changes, and modify their UI, often causing infinite loops.
     * As such callers of this should ideally check that the data has changed beforehand
     */
    PostHogPersistence.prototype.save = function () {
        var _this = this;
        if (this._disabled) {
            return;
        }
        var debounce = this._saveDebounceMs();
        if (debounce <= 0) {
            this._writeNow();
            return;
        }
        // Coalesce: if a flush is already scheduled, the latest `props`
        // will be picked up when the timer fires. Otherwise schedule one.
        if (!(0, core_1.isUndefined)(this._pendingSaveTimer)) {
            return;
        }
        this._pendingSaveTimer = setTimeout(function () {
            _this._pendingSaveTimer = undefined;
            _this._writeNow();
        }, debounce);
    };
    /**
     * Force any pending debounced save to land in storage immediately.
     * No-op when there is no pending timer — crucially, this means the
     * `beforeunload` / `pagehide` listeners installed in the constructor
     * cannot accidentally resurrect a storage entry that `remove()` or
     * `clear()` just deleted. Without this guard, the listener would
     * call `_writeNow()` and write the in-memory `props` (now `{}`) back
     * to storage, breaking `posthog.reset()` / opt-out flows.
     */
    PostHogPersistence.prototype.flush = function () {
        if ((0, core_1.isUndefined)(this._pendingSaveTimer)) {
            return;
        }
        clearTimeout(this._pendingSaveTimer);
        this._pendingSaveTimer = undefined;
        this._writeNow();
    };
    PostHogPersistence.prototype._writeNow = function () {
        if (this._disabled) {
            return;
        }
        if (this._splitStorage) {
            this._writeNowSplit();
            return;
        }
        this._writeEntry(this._storage, this._name, this.props, MAIN_STORAGE_SLOT);
    };
    // Partition `props` by storage group and write each entry independently:
    // the main blob without the grouped keys (stripping them completes the
    // migration), plus one entry per group holding only its keys. Per-entry
    // fingerprints mean a main-blob change does not rewrite the rarely-changing
    // flag entries, and vice-versa — which is the whole bandwidth win.
    // Group entries go to `localStore` directly so they never hit the 4 KB
    // cookie or the localPlusCookie re-write-on-parse path. INVARIANT: keep group
    // entries on `localStore` — `_entryFingerprint` omits the cookie options from
    // group fingerprints precisely because cookies can never carry a group entry;
    // routing one to a cookie store would make a cookie-option change silently
    // skip a needed rewrite.
    PostHogPersistence.prototype._writeNowSplit = function () {
        var e_2, _a;
        var _b;
        var _c = this._partitionProps(), main = _c.main, groups = _c.groups;
        this._writeEntry(this._storage, this._name, main, MAIN_STORAGE_SLOT);
        try {
            for (var PERSISTENCE_STORAGE_GROUPS_2 = __values(persistence_key_policy_1.PERSISTENCE_STORAGE_GROUPS), PERSISTENCE_STORAGE_GROUPS_2_1 = PERSISTENCE_STORAGE_GROUPS_2.next(); !PERSISTENCE_STORAGE_GROUPS_2_1.done; PERSISTENCE_STORAGE_GROUPS_2_1 = PERSISTENCE_STORAGE_GROUPS_2.next()) {
                var group = PERSISTENCE_STORAGE_GROUPS_2_1.value;
                var groupProps = groups[group];
                // Don't materialize an entry just to hold `{}`: skip a group that is
                // empty and has never been persisted. Once a group has held content
                // we keep writing it (even when empty) so a later clear actually lands.
                if ((0, core_2.isEmptyObject)(groupProps) && !((_b = this._slotState[group]) === null || _b === void 0 ? void 0 : _b.persisted)) {
                    continue;
                }
                // `_writeEntry` marks the slot `persisted` (on `_slotState`) only
                // after a confirmed-successful `_set`, so a failed (e.g. quota) write
                // does not falsely mark the group as materialized on disk.
                this._writeEntry(storage_1.localStore, this._groupEntryName(group), groupProps, group);
            }
        }
        catch (e_2_1) { e_2 = { error: e_2_1 }; }
        finally {
            try {
                if (PERSISTENCE_STORAGE_GROUPS_2_1 && !PERSISTENCE_STORAGE_GROUPS_2_1.done && (_a = PERSISTENCE_STORAGE_GROUPS_2.return)) _a.call(PERSISTENCE_STORAGE_GROUPS_2);
            }
            finally { if (e_2) throw e_2.error; }
        }
    };
    PostHogPersistence.prototype._partitionProps = function () {
        var main = {};
        var groups = { flags: {}, surveys: {} };
        (0, general_utils_1.each)(this.props, function (value, key) {
            var _a;
            var group = (_a = (0, persistence_key_policy_1.getPersistenceKeyPolicy)(key)) === null || _a === void 0 ? void 0 : _a.storageGroup;
            if (group) {
                groups[group][key] = value;
            }
            else {
                main[key] = value;
            }
        });
        return { main: main, groups: groups };
    };
    // The no-op-rejection snapshot for an entry. The main entry can live in a
    // cookie, so its fingerprint also covers the cookie options (expire_days,
    // cross_subdomain, secure): a `set_config({ cookie_expiration })` must force
    // a rewrite even when props are unchanged, otherwise the cookie keeps its old
    // `Expires` header until some other prop changes. Group entries are
    // localStorage-only — cookie options never reach them, so excluding those
    // keeps a group fingerprint a pure function of its payload. That lets `load()`
    // seed it before the cookie options are even resolved, and keeps it stable
    // across the cookie-option setters that run during construction, so an
    // unchanged flag entry is neither re-serialized nor re-broadcast.
    PostHogPersistence.prototype._entryFingerprint = function (props, slot) {
        if (slot === MAIN_STORAGE_SLOT) {
            return JSON.stringify(props) + '|' + this._expire_days + '|' + this._cross_subdomain + '|' + this._secure;
        }
        // Volatile keys count by presence only: a write triggered by a real
        // content change records a fingerprint that stays valid while the
        // volatile values keep moving between writes, but adding or deleting a
        // volatile key still changes the fingerprint so the entry writes through.
        var stable = {};
        (0, general_utils_1.each)(props, function (value, key) {
            var _a;
            stable[key] = ((_a = (0, persistence_key_policy_1.getPersistenceKeyPolicy)(key)) === null || _a === void 0 ? void 0 : _a.volatile) ? VOLATILE_FINGERPRINT_PLACEHOLDER : value;
        });
        return JSON.stringify(stable);
    };
    // No-op rejection: skip the write when nothing that affects this entry has
    // changed since the last successful write. Callers spam `save()` after every
    // property change, and many of those changes leave the storage payload
    // unchanged. Writing identical bytes to localStorage still fires a cross-tab
    // `storage` event where Chrome allocates the payload buffer in mojo IPC even
    // though no listener reacts.
    //
    // JSON.stringify can throw on BigInt / circular refs. We let the
    // underlying storage layer keep its existing try/catch behaviour
    // (log and drop) by falling through on serialization errors.
    PostHogPersistence.prototype._writeEntry = function (storage, name, props, slot) {
        var state = this._slotWriteState(slot);
        // Fast path for group slots (localStorage-only): when nothing in the
        // group changed since its last successful write, skip the JSON.stringify
        // of the large flag payload entirely. The main slot is excluded —
        // it is small, changes on nearly every write, and carries cookie options
        // in its fingerprint, so it always serializes.
        if (slot !== MAIN_STORAGE_SLOT && !state.dirty && !(0, core_1.isUndefined)(state.fingerprint)) {
            return;
        }
        var fingerprint;
        try {
            fingerprint = this._entryFingerprint(props, slot);
            if (fingerprint === state.fingerprint) {
                state.dirty = false;
                return;
            }
        }
        catch (_a) {
            // serialization failed (BigInt / circular ref); fall through to
            // storage._set, which handles the error itself, but don't cache an
            // un-fingerprinted write.
            fingerprint = undefined;
        }
        // Record the fingerprint (and clear the dirty flag, mark persisted) only
        // after a confirmed-successful durable write: localStorage / sessionStorage
        // swallow quota errors, so caching ahead of a failed write would skip
        // every future retry and silently lose the entry.
        if (storage._set(name, props, this._expire_days, this._cross_subdomain, this._secure, this._config.debug)) {
            state.dirty = false;
            if (slot !== MAIN_STORAGE_SLOT) {
                // The group entry has now actually landed on disk — only here is
                // it correct to record it as persisted (gates the empty-entry
                // skip in `_writeNowSplit`).
                state.persisted = true;
            }
            if (!(0, core_1.isUndefined)(fingerprint)) {
                state.fingerprint = fingerprint;
            }
        }
        else if (this._config.debug) {
            // The durable write did not land (e.g. localStorage quota). The slot
            // stays dirty / un-fingerprinted so the next save retries it; surface
            // it under debug so a repeated failure on a group entry — which would
            // otherwise silently strand the flag cache — is visible.
            logger_1.logger.warn("failed to persist storage entry \"".concat(name, "\"; will retry on next save"));
        }
    };
    // `keepGroupEntries` is set by the cookie-option setters (set_secure /
    // set_cross_subdomain). A cookie-scope change has to clear the cookie-backed
    // main entry, but the group entries are localStorage-only and entirely
    // scope-independent, so deleting and rewriting them would be the exact
    // per-page-load flag-blob churn the split exists to remove (these setters fire
    // once each on every construction, transitioning the in-memory option from
    // undefined to its configured value). Opt-out / reset (set_disabled / clear)
    // pass nothing and wipe everything.
    //
    // INVARIANT for `keepGroupEntries: true`: the caller must not also mutate
    // `props`. We keep the on-disk group entries AND their retained fingerprint;
    // that is only safe while `props` still matches what is on disk. The cookie
    // setters satisfy this (they touch only `_secure` / `_cross_subdomain`). A
    // future caller that clears or rewrites `props` while keeping the entries
    // would leave the retained fingerprint describing stale on-disk content and
    // skip the corrective write.
    PostHogPersistence.prototype.remove = function (_a) {
        var e_3, _b;
        var _c = _a === void 0 ? {} : _a, _d = _c.keepGroupEntries, keepGroupEntries = _d === void 0 ? false : _d;
        // Cancel any pending debounced write — the storage entry is going
        // away so there is nothing useful to flush.
        if (!(0, core_1.isUndefined)(this._pendingSaveTimer)) {
            clearTimeout(this._pendingSaveTimer);
            this._pendingSaveTimer = undefined;
        }
        // remove both domain and subdomain cookies
        this._storage._remove(this._name, false);
        this._storage._remove(this._name, true);
        // Wipe the group entries too — even when the split is currently off — so
        // a default flip-flop or version downgrade cannot strand an orphaned
        // flag entry that would leak across users on reset()/opt-out.
        // Only the owning instance does this: the sessionStorage sibling
        // posthog-core spins up shares this instance's storage name, so it must
        // not delete the localStorage owner's entries. localStorage-only.
        if (!keepGroupEntries && this._ownsSplitStorage) {
            try {
                for (var PERSISTENCE_STORAGE_GROUPS_3 = __values(persistence_key_policy_1.PERSISTENCE_STORAGE_GROUPS), PERSISTENCE_STORAGE_GROUPS_3_1 = PERSISTENCE_STORAGE_GROUPS_3.next(); !PERSISTENCE_STORAGE_GROUPS_3_1.done; PERSISTENCE_STORAGE_GROUPS_3_1 = PERSISTENCE_STORAGE_GROUPS_3.next()) {
                    var group = PERSISTENCE_STORAGE_GROUPS_3_1.value;
                    storage_1.localStore._remove(this._groupEntryName(group));
                }
            }
            catch (e_3_1) { e_3 = { error: e_3_1 }; }
            finally {
                try {
                    if (PERSISTENCE_STORAGE_GROUPS_3_1 && !PERSISTENCE_STORAGE_GROUPS_3_1.done && (_b = PERSISTENCE_STORAGE_GROUPS_3.return)) _b.call(PERSISTENCE_STORAGE_GROUPS_3);
                }
                finally { if (e_3) throw e_3.error; }
            }
        }
        // The main entry is gone, so its bookkeeping must reset for the next
        // save to write through. When the group entries are kept, so is their
        // fingerprint/persisted state — that is what lets the following save
        // recognise them as unchanged and skip the rewrite.
        if (keepGroupEntries) {
            delete this._slotState[MAIN_STORAGE_SLOT];
        }
        else {
            this._slotState = {};
        }
    };
    // removes the storage entry and deletes all loaded data
    // forced name for tests
    PostHogPersistence.prototype.clear = function () {
        this.remove();
        this.props = {};
    };
    /**
     * @param {Object} props
     * @param {*=} default_value
     * @param {number=} days
     */
    PostHogPersistence.prototype.register_once = function (props, default_value, days) {
        var _this = this;
        if ((0, core_2.isObject)(props)) {
            if ((0, core_1.isUndefined)(default_value)) {
                default_value = 'None';
            }
            this._expire_days = (0, core_1.isUndefined)(days) ? this._default_expiry : days;
            var hasChanges_1 = false;
            (0, general_utils_1.each)(props, function (val, prop) {
                if (!_this.props.hasOwnProperty(prop) || _this.props[prop] === default_value) {
                    _this._setProp(prop, val);
                    hasChanges_1 = true;
                }
            });
            if (hasChanges_1) {
                this.save();
                return true;
            }
        }
        return false;
    };
    /**
     * @param {Object} props
     * @param {number=} days
     */
    PostHogPersistence.prototype.register = function (props, days) {
        var _this = this;
        if ((0, core_2.isObject)(props)) {
            this._expire_days = (0, core_1.isUndefined)(days) ? this._default_expiry : days;
            var hasChanges_2 = false;
            (0, general_utils_1.each)(props, function (val, prop) {
                if (props.hasOwnProperty(prop) && _this.props[prop] !== val) {
                    _this._setProp(prop, val);
                    hasChanges_2 = true;
                }
            });
            if (hasChanges_2) {
                this.save();
                return true;
            }
        }
        return false;
    };
    PostHogPersistence.prototype.unregister = function (prop) {
        if (prop in this.props) {
            this._deleteProp(prop);
            this.save();
        }
    };
    PostHogPersistence.prototype.update_campaign_params = function () {
        if (!this._campaign_params_saved) {
            var campaignParams = (0, event_utils_1.getCampaignParams)(this._config.custom_campaign_params, this._config.mask_personal_data_properties, this._config.custom_personal_data_properties);
            // only save campaign params if there were any
            if (!(0, core_2.isEmptyObject)((0, general_utils_1.stripEmptyProperties)(campaignParams))) {
                this.register(campaignParams);
            }
            this._campaign_params_saved = true;
        }
    };
    PostHogPersistence.prototype.update_search_keyword = function () {
        this.register((0, event_utils_1.getSearchInfo)());
    };
    PostHogPersistence.prototype.update_referrer_info = function () {
        this.register_once((0, event_utils_1.getReferrerInfo)(), undefined);
    };
    PostHogPersistence.prototype.set_initial_person_info = function () {
        var _a;
        if (this.props[constants_1.INITIAL_CAMPAIGN_PARAMS] || this.props[constants_1.INITIAL_REFERRER_INFO]) {
            // the user has initial properties stored the previous way, don't save them again
            return;
        }
        this.register_once((_a = {},
            _a[constants_1.INITIAL_PERSON_INFO] = (0, event_utils_1.getPersonInfo)(this._config.mask_personal_data_properties, this._config.custom_personal_data_properties, this._config.disable_capture_url_hashes),
            _a), undefined);
    };
    PostHogPersistence.prototype.get_initial_props = function () {
        var _this = this;
        var p = {};
        // this section isn't written to anymore, but we should keep reading from it for backwards compatibility
        // for a while
        (0, general_utils_1.each)([constants_1.INITIAL_REFERRER_INFO, constants_1.INITIAL_CAMPAIGN_PARAMS], function (key) {
            var initialReferrerInfo = _this.props[key];
            if (initialReferrerInfo) {
                (0, general_utils_1.each)(initialReferrerInfo, function (v, k) {
                    p['$initial_' + (0, core_2.stripLeadingDollar)(k)] = v;
                });
            }
        });
        var initialPersonInfo = this.props[constants_1.INITIAL_PERSON_INFO];
        if (initialPersonInfo) {
            var initialPersonProps = (0, event_utils_1.getInitialPersonPropsFromInfo)(initialPersonInfo, this._config.disable_capture_url_hashes);
            (0, general_utils_1.extend)(p, initialPersonProps);
        }
        return p;
    };
    // safely fills the passed in object with stored properties,
    // does not override any properties defined in both
    // returns the passed in object
    PostHogPersistence.prototype.safe_merge = function (props) {
        (0, general_utils_1.each)(this.props, function (val, prop) {
            if (!(prop in props)) {
                props[prop] = val;
            }
        });
        return props;
    };
    PostHogPersistence.prototype.update_config = function (config, oldConfig, isDisabled) {
        this._default_expiry = this._expire_days = config['cookie_expiration'];
        this.set_disabled(config['disable_persistence'] || !!isDisabled);
        this.set_cross_subdomain(config['cross_subdomain_cookie']);
        this.set_secure(config['secure_cookie']);
        var persistenceChanged = config.persistence !== oldConfig.persistence ||
            !isArrayContentsEqual(config.cookie_persisted_properties || [], oldConfig.cookie_persisted_properties || []);
        // `_buildStorage` re-resolves both the backend and `_splitStorageEligible`,
        // so on a persistence change build the new store first, then derive the
        // split flag from the fresh eligibility. The new backend may no longer be
        // split-eligible (e.g. localStorage -> memory).
        var newStore = persistenceChanged ? this._buildStorage(config) : this._storage;
        var wantSplit = this._resolveSplitStorage(config);
        // Migrate when the backend changed or the split routing flipped at
        // runtime, e.g. set_config({ split_storage: true })
        // without touching persistence. Either way we clear the old layout and
        // re-save so subsequent reads/writes land in the right entries.
        if (persistenceChanged || wantSplit !== this._splitStorage) {
            var props = this.props;
            this.clear();
            this._storage = newStore;
            this._splitStorage = wantSplit;
            this.props = props;
            this.save();
        }
    };
    PostHogPersistence.prototype.set_disabled = function (disabled) {
        this._disabled = disabled;
        if (this._disabled) {
            this.remove();
        }
        else {
            this.save();
        }
    };
    PostHogPersistence.prototype.set_cross_subdomain = function (cross_subdomain) {
        if (cross_subdomain !== this._cross_subdomain) {
            this._cross_subdomain = cross_subdomain;
            this.remove({ keepGroupEntries: true });
            this.save();
        }
    };
    PostHogPersistence.prototype.set_secure = function (secure) {
        if (secure !== this._secure) {
            this._secure = secure;
            this.remove({ keepGroupEntries: true });
            this.save();
        }
    };
    PostHogPersistence.prototype.set_event_timer = function (event_name, timestamp) {
        var timers = this.props[constants_1.EVENT_TIMERS_KEY] || {};
        timers[event_name] = timestamp;
        this._setProp(constants_1.EVENT_TIMERS_KEY, timers);
        this.save();
    };
    PostHogPersistence.prototype.remove_event_timer = function (event_name) {
        var timers = this.props[constants_1.EVENT_TIMERS_KEY] || {};
        var timestamp = timers[event_name];
        if (!(0, core_1.isUndefined)(timestamp)) {
            delete timers[event_name];
            this._setProp(constants_1.EVENT_TIMERS_KEY, timers);
            this.save();
        }
        return timestamp;
    };
    PostHogPersistence.prototype.get_property = function (prop) {
        return this.props[prop];
    };
    PostHogPersistence.prototype.set_property = function (prop, to) {
        this._setProp(prop, to);
        this.save();
    };
    PostHogPersistence.prototype._setProp = function (prop, to) {
        var _a;
        this.props[prop] = to;
        // A volatile value change never dirties its group — it changes on every
        // remote load and would otherwise force a rewrite of the large entry per
        // load. Deletions still dirty (see _deleteProp): presence is part of the
        // fingerprint, the moving value is not.
        if (!((_a = (0, persistence_key_policy_1.getPersistenceKeyPolicy)(prop)) === null || _a === void 0 ? void 0 : _a.volatile)) {
            this._markGroupDirty(prop);
        }
    };
    PostHogPersistence.prototype._deleteProp = function (prop) {
        delete this.props[prop];
        this._markGroupDirty(prop);
    };
    // Mark the prop's storage group dirty so its entry is re-serialized on the
    // next write. Props with no group live in the main blob, which always writes.
    PostHogPersistence.prototype._markGroupDirty = function (prop) {
        var _a;
        var group = (_a = (0, persistence_key_policy_1.getPersistenceKeyPolicy)(prop)) === null || _a === void 0 ? void 0 : _a.storageGroup;
        if (group) {
            this._slotWriteState(group).dirty = true;
        }
    };
    // The write-bookkeeping record for a slot, created on first access.
    PostHogPersistence.prototype._slotWriteState = function (slot) {
        return this._slotState[slot] || (this._slotState[slot] = {});
    };
    return PostHogPersistence;
}());
exports.PostHogPersistence = PostHogPersistence;
//# sourceMappingURL=posthog-persistence.js.map