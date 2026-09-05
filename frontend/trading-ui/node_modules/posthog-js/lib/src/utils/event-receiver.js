"use strict";
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
Object.defineProperty(exports, "__esModule", { value: true });
exports.EventReceiver = void 0;
var posthog_surveys_types_1 = require("../posthog-surveys-types");
var action_matcher_1 = require("../extensions/surveys/action-matcher");
var property_utils_1 = require("@posthog/browser-common/utils/property-utils");
var core_1 = require("@posthog/core");
/**
 * Abstract base class for receiving events and matching them to triggerable items.
 * Subclasses implement type-specific behavior for surveys and product tours.
 */
var EventReceiver = /** @class */ (function () {
    function EventReceiver(instance) {
        var _this = this;
        var _a, _b;
        /**
         * Items armed by an event or action but not yet shown live here, in memory only.
         * They are intentionally NOT persisted, so they do not survive a page reload: an
         * event trigger only displays an item in the session the event fired in. Once an
         * item is shown, surviving items are promoted into persistence (see `onEvent`) so
         * a reload re-reads and re-displays them until the user interacts — but that
         * persisted activation is scoped to the triggering session (see
         * `_getPersistedActivatedIds`), so it does not leak into a brand-new session where
         * the trigger never fired.
         */
        this._pendingActivatedItems = [];
        this._instance = instance;
        this._eventToItems = new Map();
        this._cancelEventToItems = new Map();
        this._actionToItems = new Map();
        // A persisted activation belongs to the session the item was shown in. When the session
        // rotates (idle timeout, max length, cross-tab adoption) the trigger did not fire in the
        // new session, so the activation is stale and must be dropped. We subscribe to rotations
        // here rather than relying only on reading the session id on the display path: that read
        // is read-only and so cannot observe an idle-expired session (and must not force a
        // rotation, since merely checking whether to show a survey should never keep a session
        // alive). The read-time check in `_getPersistedActivatedIds` remains as a complementary
        // backstop for a session that had already rotated in persistence before this page loaded.
        (_b = (_a = this._instance) === null || _a === void 0 ? void 0 : _a.onSessionId) === null || _b === void 0 ? void 0 : _b.call(_a, function (sessionId) { return _this._onSessionIdChanged(sessionId); });
    }
    EventReceiver.prototype._doesEventMatchFilter = function (eventConfig, eventPayload) {
        if (!eventConfig) {
            return false;
        }
        return (0, property_utils_1.matchPropertyFilters)(eventConfig.propertyFilters, eventPayload === null || eventPayload === void 0 ? void 0 : eventPayload.properties);
    };
    EventReceiver.prototype._buildEventToItemMap = function (items, conditionField) {
        var map = new Map();
        items.forEach(function (item) {
            var _a, _b, _c;
            (_c = (_b = (_a = item.conditions) === null || _a === void 0 ? void 0 : _a[conditionField]) === null || _b === void 0 ? void 0 : _b.values) === null || _c === void 0 ? void 0 : _c.forEach(function (event) {
                if (event === null || event === void 0 ? void 0 : event.name) {
                    var existing = map.get(event.name) || [];
                    existing.push(item.id);
                    map.set(event.name, existing);
                }
            });
        });
        return map;
    };
    /**
     * build a map of (Event1) => [Item1, Item2, Item3]
     * used for items that should be [activated|cancelled] by Event1
     */
    EventReceiver.prototype._getMatchingItems = function (eventName, eventPayload, conditionField) {
        var _this = this;
        var itemIdMap = conditionField === posthog_surveys_types_1.SurveyEventType.Activation ? this._eventToItems : this._cancelEventToItems;
        var itemIds = itemIdMap.get(eventName);
        var items = [];
        this._getItems(function (allItems) {
            items = allItems.filter(function (item) { return itemIds === null || itemIds === void 0 ? void 0 : itemIds.includes(item.id); });
        });
        return items.filter(function (item) {
            var _a, _b, _c;
            var eventConfig = (_c = (_b = (_a = item.conditions) === null || _a === void 0 ? void 0 : _a[conditionField]) === null || _b === void 0 ? void 0 : _b.values) === null || _c === void 0 ? void 0 : _c.find(function (e) { return e.name === eventName; });
            return _this._doesEventMatchFilter(eventConfig, eventPayload);
        });
    };
    EventReceiver.prototype.register = function (items) {
        var _a;
        if ((0, core_1.isUndefined)((_a = this._instance) === null || _a === void 0 ? void 0 : _a._addCaptureHook)) {
            return;
        }
        this._setupEventBasedItems(items);
        this._setupActionBasedItems(items);
    };
    EventReceiver.prototype._setupActionBasedItems = function (items) {
        var _this = this;
        var actionBasedItems = items.filter(function (item) { var _a, _b, _c, _d; return ((_a = item.conditions) === null || _a === void 0 ? void 0 : _a.actions) && ((_d = (_c = (_b = item.conditions) === null || _b === void 0 ? void 0 : _b.actions) === null || _c === void 0 ? void 0 : _c.values) === null || _d === void 0 ? void 0 : _d.length) > 0; });
        if (actionBasedItems.length === 0) {
            return;
        }
        if (this._actionMatcher == null) {
            this._actionMatcher = new action_matcher_1.ActionMatcher(this._instance);
            this._actionMatcher.init();
            // match any actions to its corresponding item.
            var matchActionToItem = function (actionName) {
                _this.onAction(actionName);
            };
            this._actionMatcher._addActionHook(matchActionToItem);
        }
        actionBasedItems.forEach(function (item) {
            var _a, _b, _c, _d, _e, _f, _g, _h, _j, _k;
            if (item.conditions &&
                ((_a = item.conditions) === null || _a === void 0 ? void 0 : _a.actions) &&
                ((_c = (_b = item.conditions) === null || _b === void 0 ? void 0 : _b.actions) === null || _c === void 0 ? void 0 : _c.values) &&
                ((_f = (_e = (_d = item.conditions) === null || _d === void 0 ? void 0 : _d.actions) === null || _e === void 0 ? void 0 : _e.values) === null || _f === void 0 ? void 0 : _f.length) > 0) {
                // register the known set of actions with
                // the action-matcher so it can match
                // events to actions
                (_g = _this._actionMatcher) === null || _g === void 0 ? void 0 : _g.register(item.conditions.actions.values);
                // maintain a mapping of (Action1) => [Item1, Item2, Item3]
                // where Items 1-3 are all activated by Action1
                (_k = (_j = (_h = item.conditions) === null || _h === void 0 ? void 0 : _h.actions) === null || _j === void 0 ? void 0 : _j.values) === null || _k === void 0 ? void 0 : _k.forEach(function (action) {
                    if (action && action.name) {
                        var knownItems = _this._actionToItems.get(action.name);
                        if (knownItems) {
                            knownItems.push(item.id);
                        }
                        _this._actionToItems.set(action.name, knownItems || [item.id]);
                    }
                });
            }
        });
    };
    EventReceiver.prototype._setupEventBasedItems = function (items) {
        var _this = this;
        var _a;
        var eventBasedItems = items.filter(function (item) { var _a, _b, _c, _d; return ((_a = item.conditions) === null || _a === void 0 ? void 0 : _a.events) && ((_d = (_c = (_b = item.conditions) === null || _b === void 0 ? void 0 : _b.events) === null || _c === void 0 ? void 0 : _c.values) === null || _d === void 0 ? void 0 : _d.length) > 0; });
        var itemsWithCancelEvents = items.filter(function (item) { var _a, _b, _c, _d; return ((_a = item.conditions) === null || _a === void 0 ? void 0 : _a.cancelEvents) && ((_d = (_c = (_b = item.conditions) === null || _b === void 0 ? void 0 : _b.cancelEvents) === null || _c === void 0 ? void 0 : _c.values) === null || _d === void 0 ? void 0 : _d.length) > 0; });
        if (eventBasedItems.length === 0 && itemsWithCancelEvents.length === 0) {
            return;
        }
        // match any events to its corresponding item.
        var matchEventToItem = function (eventName, eventPayload) {
            _this.onEvent(eventName, eventPayload);
        };
        (_a = this._instance) === null || _a === void 0 ? void 0 : _a._addCaptureHook(matchEventToItem);
        this._eventToItems = this._buildEventToItemMap(items, posthog_surveys_types_1.SurveyEventType.Activation);
        this._cancelEventToItems = this._buildEventToItemMap(items, posthog_surveys_types_1.SurveyEventType.Cancellation);
    };
    EventReceiver.prototype.onEvent = function (event, eventPayload) {
        var _this = this;
        var _a, _b;
        var logger = this._getLogger();
        // An item reacting to one of its own lifecycle events (shown / dismissed / sent).
        var itemId = ((_a = eventPayload === null || eventPayload === void 0 ? void 0 : eventPayload.properties) === null || _a === void 0 ? void 0 : _a.$survey_id) || ((_b = eventPayload === null || eventPayload === void 0 ? void 0 : eventPayload.properties) === null || _b === void 0 ? void 0 : _b.$product_tour_id);
        if (itemId && this.getActivatedIds().includes(itemId)) {
            var outcome = this._activationOutcome(event, itemId);
            if (outcome === 'consume') {
                logger.info('event consumed activated item, removing it', { event: event, itemId: itemId });
                this._deactivateItems([itemId]);
                return;
            }
            if (outcome === 'persist') {
                logger.info('shown item promoted to persisted activation', { event: event, itemId: itemId });
                this._persistActivation(itemId);
                return;
            }
            // 'ignore': no activation transition for this item on this event — fall through.
        }
        // check if this event should cancel any pending items
        if (this._cancelEventToItems.has(event)) {
            var itemsToCancel = this._getMatchingItems(event, eventPayload, posthog_surveys_types_1.SurveyEventType.Cancellation);
            if (itemsToCancel.length > 0) {
                logger.info('cancel event matched, cancelling items', {
                    event: event,
                    itemsToCancel: itemsToCancel.map(function (s) { return s.id; }),
                });
                this._deactivateItems(itemsToCancel.map(function (item) { return item.id; }));
                // cancel any pending timeout for these items
                itemsToCancel.forEach(function (item) { return _this._cancelPendingItem(item.id); });
            }
        }
        // if the event is not in the eventToItems map, nothing else to do
        if (!this._eventToItems.has(event)) {
            return;
        }
        logger.info('event name matched', {
            event: event,
            eventPayload: eventPayload,
            items: this._eventToItems.get(event),
        });
        var matchedItems = this._getMatchingItems(event, eventPayload, posthog_surveys_types_1.SurveyEventType.Activation);
        this._activateItems(matchedItems.map(function (item) { return item.id; }));
    };
    EventReceiver.prototype.onAction = function (actionName) {
        if (this._actionToItems.has(actionName)) {
            this._activateItems(this._actionToItems.get(actionName) || []);
        }
    };
    /** Arm items in memory only (not persisted) until they are shown. */
    EventReceiver.prototype._activateItems = function (itemIds) {
        if (itemIds.length === 0) {
            return;
        }
        this._pendingActivatedItems = __spreadArray([], __read(new Set(__spreadArray(__spreadArray([], __read(this._pendingActivatedItems), false), __read(itemIds), false))), false);
        this._getLogger().info('updating activated items', { activatedItems: this.getActivatedIds() });
    };
    /**
     * Move an in-memory activation into persistence so it survives a page reload within the
     * triggering session. The set is (re)stamped with the current session id; reading it back
     * via `_getPersistedActivatedIds` discards it once the session rolls over. Because we build
     * on top of the session-scoped read, a stale set left over from a previous session is
     * dropped here rather than accumulated.
     */
    EventReceiver.prototype._persistActivation = function (itemId) {
        this._pendingActivatedItems = this._pendingActivatedItems.filter(function (id) { return id !== itemId; });
        var persisted = this._getPersistedActivatedIds();
        if (!persisted.includes(itemId)) {
            this._setActivatedItems(__spreadArray(__spreadArray([], __read(persisted), false), [itemId], false));
            this._stampActivationSession();
        }
    };
    /** Drop items from both the in-memory and persisted activation sets. */
    EventReceiver.prototype._deactivateItems = function (itemIds) {
        var remove = new Set(itemIds);
        this._pendingActivatedItems = this._pendingActivatedItems.filter(function (id) { return !remove.has(id); });
        var persisted = this._getRawPersistedActivatedIds();
        var nextPersisted = persisted.filter(function (id) { return !remove.has(id); });
        if (nextPersisted.length !== persisted.length) {
            this._setActivatedItems(nextPersisted);
            if (nextPersisted.length === 0) {
                this._clearActivationSession();
            }
        }
    };
    /** The raw persisted set as stored, ignoring session scoping. */
    EventReceiver.prototype._getRawPersistedActivatedIds = function () {
        var _a, _b;
        var activatedKey = this._getActivatedKey();
        var existingActivatedItems = (_b = (_a = this._instance) === null || _a === void 0 ? void 0 : _a.persistence) === null || _b === void 0 ? void 0 : _b.props[activatedKey];
        return existingActivatedItems ? existingActivatedItems : [];
    };
    /**
     * The persisted activations that still belong to the current session. A persisted activation
     * is scoped to the session the item was shown in: an event/action trigger only earns a display
     * in the session it fired in, so once the session rolls over the activation is stale and must
     * not silently re-display the item in a brand-new session where the trigger never fired.
     */
    EventReceiver.prototype._getPersistedActivatedIds = function () {
        var _a, _b, _c, _d;
        var ids = this._getRawPersistedActivatedIds();
        if (ids.length === 0) {
            return [];
        }
        var stampedSessionId = (_b = (_a = this._instance) === null || _a === void 0 ? void 0 : _a.persistence) === null || _b === void 0 ? void 0 : _b.props[this._getActivatedSessionKey()];
        // Read-only: this catches a session that had already rotated in persistence before this
        // page loaded (the stamped id no longer matches the current one). It intentionally does
        // NOT force a rotation of an idle-expired session — that case is handled by the
        // `onSessionId` subscription in the constructor, which clears the activation when the
        // session actually rotates on the next real event.
        var currentSessionId = (_d = (_c = this._instance) === null || _c === void 0 ? void 0 : _c.get_session_id) === null || _d === void 0 ? void 0 : _d.call(_c);
        // No resolvable session (e.g. cookieless mode) → treat the activation as un-scopable and
        // do not carry it across a reload.
        if (!currentSessionId || stampedSessionId !== currentSessionId) {
            return [];
        }
        return ids;
    };
    /** Stamp the persisted activation set with the current session id. */
    EventReceiver.prototype._stampActivationSession = function () {
        var _a, _b;
        var currentSessionId = (_b = (_a = this._instance) === null || _a === void 0 ? void 0 : _a.get_session_id) === null || _b === void 0 ? void 0 : _b.call(_a);
        if (currentSessionId) {
            this._setActivatedSession(currentSessionId);
        }
    };
    /** Forget the session stamp once nothing is persisted under it. */
    EventReceiver.prototype._clearActivationSession = function () {
        this._clearActivatedSession();
    };
    /**
     * Drop a persisted activation once the session it was stamped under is no longer current.
     * Fired on session rotation (idle timeout, max length, cross-tab adoption) — the cases the
     * read-only session read on the display path cannot observe.
     */
    EventReceiver.prototype._onSessionIdChanged = function (sessionId) {
        var _a, _b;
        var stampedSessionId = (_b = (_a = this._instance) === null || _a === void 0 ? void 0 : _a.persistence) === null || _b === void 0 ? void 0 : _b.props[this._getActivatedSessionKey()];
        if (stampedSessionId && stampedSessionId !== sessionId) {
            if (this._getRawPersistedActivatedIds().length > 0) {
                this._setActivatedItems([]);
            }
            this._clearActivationSession();
        }
    };
    EventReceiver.prototype.getActivatedIds = function () {
        var _this = this;
        // The activated set is the union of in-memory (armed, not yet shown) and persisted
        // (shown and surviving) items. In-memory ones do not survive a reload by design.
        var all = __spreadArray([], __read(new Set(__spreadArray(__spreadArray([], __read(this._getPersistedActivatedIds()), false), __read(this._pendingActivatedItems), false))), false);
        return all.filter(function (itemId) { return !_this._isItemPermanentlyIneligible(itemId); });
    };
    /**
     * Clear all activations. Called on `posthog.reset()` so a logout or account switch
     * (without a full page reload) does not leave an event-armed item live for the next
     * user — the in-memory set would otherwise survive `persistence.clear()`.
     */
    EventReceiver.prototype.reset = function () {
        this._pendingActivatedItems = [];
        if (this._getRawPersistedActivatedIds().length > 0) {
            this._setActivatedItems([]);
        }
        this._clearActivationSession();
    };
    EventReceiver.prototype.getEventToItemsMap = function () {
        return this._eventToItems;
    };
    EventReceiver.prototype._getActionMatcher = function () {
        return this._actionMatcher;
    };
    return EventReceiver;
}());
exports.EventReceiver = EventReceiver;
//# sourceMappingURL=event-receiver.js.map