import { SurveyActionType, SurveyEventWithFilters } from '../posthog-surveys-types';
import { ActionMatcher } from '../extensions/surveys/action-matcher';
import { PostHog } from '../posthog-core';
import { CaptureResult } from '../types';
import { createLogger } from '@posthog/browser-common/utils/logger';
/**
 * Interface for items that can be triggered by events/actions.
 * Both Survey and ProductTour implement this interface.
 */
export interface EventTriggerable {
    id: string;
    conditions?: {
        events?: {
            repeatedActivation?: boolean;
            values: SurveyEventWithFilters[];
        } | null;
        cancelEvents?: {
            values: SurveyEventWithFilters[];
        } | null;
        actions?: {
            values: SurveyActionType[];
        } | null;
    } | null;
}
/**
 * What a captured lifecycle event (shown / dismissed / sent) does to an already-activated item:
 * - `consume`: it's done — drop it from both the in-memory and persisted sets.
 * - `persist`: it was shown and should survive a reload — move it from memory into persistence.
 * - `ignore`: no transition for this item on this event.
 */
export type ActivationOutcome = 'consume' | 'persist' | 'ignore';
/**
 * Abstract base class for receiving events and matching them to triggerable items.
 * Subclasses implement type-specific behavior for surveys and product tours.
 */
export declare abstract class EventReceiver<T extends EventTriggerable> {
    protected _eventToItems: Map<string, string[]>;
    protected _cancelEventToItems: Map<string, string[]>;
    protected readonly _actionToItems: Map<string, string[]>;
    protected _actionMatcher?: ActionMatcher | null;
    protected readonly _instance?: PostHog;
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
    private _pendingActivatedItems;
    constructor(instance: PostHog);
    protected abstract _getActivatedKey(): string;
    /** Persistence key under which the session id of the persisted activation set is stamped. */
    protected abstract _getActivatedSessionKey(): string;
    protected abstract _getShownEventName(): string;
    protected abstract _getItems(callback: (items: T[]) => void): void;
    protected abstract _cancelPendingItem(itemId: string): void;
    protected abstract _getLogger(): ReturnType<typeof createLogger>;
    protected abstract _setActivatedItems(eligibleItems: string[]): void;
    /** Persist the session id the current activation set belongs to. */
    protected abstract _setActivatedSession(sessionId: string): void;
    /** Forget the persisted session stamp. */
    protected abstract _clearActivatedSession(): void;
    /** Check if item is permanently ineligible (e.g. completed/dismissed). Skip adding to activated list. */
    protected abstract _isItemPermanentlyIneligible(itemId?: string): boolean;
    /**
     * Decide what a captured lifecycle `event` does to an already-activated `itemId`. Most items are
     * consumed when shown (so they only reappear when their trigger fires again). Surveys keep
     * non-repeatable ones activated — promoting them to session-scoped persistence on shown — until the
     * user dismisses or answers them, so an event-triggered survey survives a reload within the
     * triggering session (but not a brand-new session) until it's actually interacted with.
     */
    protected abstract _activationOutcome(event: string, itemId: string): ActivationOutcome;
    private _doesEventMatchFilter;
    private _buildEventToItemMap;
    /**
     * build a map of (Event1) => [Item1, Item2, Item3]
     * used for items that should be [activated|cancelled] by Event1
     */
    private _getMatchingItems;
    register(items: T[]): void;
    private _setupActionBasedItems;
    private _setupEventBasedItems;
    onEvent(event: string, eventPayload?: CaptureResult): void;
    onAction(actionName: string): void;
    /** Arm items in memory only (not persisted) until they are shown. */
    private _activateItems;
    /**
     * Move an in-memory activation into persistence so it survives a page reload within the
     * triggering session. The set is (re)stamped with the current session id; reading it back
     * via `_getPersistedActivatedIds` discards it once the session rolls over. Because we build
     * on top of the session-scoped read, a stale set left over from a previous session is
     * dropped here rather than accumulated.
     */
    private _persistActivation;
    /** Drop items from both the in-memory and persisted activation sets. */
    private _deactivateItems;
    /** The raw persisted set as stored, ignoring session scoping. */
    private _getRawPersistedActivatedIds;
    /**
     * The persisted activations that still belong to the current session. A persisted activation
     * is scoped to the session the item was shown in: an event/action trigger only earns a display
     * in the session it fired in, so once the session rolls over the activation is stale and must
     * not silently re-display the item in a brand-new session where the trigger never fired.
     */
    private _getPersistedActivatedIds;
    /** Stamp the persisted activation set with the current session id. */
    private _stampActivationSession;
    /** Forget the session stamp once nothing is persisted under it. */
    private _clearActivationSession;
    /**
     * Drop a persisted activation once the session it was stamped under is no longer current.
     * Fired on session rotation (idle timeout, max length, cross-tab adoption) — the cases the
     * read-only session read on the display path cannot observe.
     */
    private _onSessionIdChanged;
    getActivatedIds(): string[];
    /**
     * Clear all activations. Called on `posthog.reset()` so a logout or account switch
     * (without a full page reload) does not leave an event-armed item live for the next
     * user — the in-memory set would otherwise survive `persistence.clear()`.
     */
    reset(): void;
    getEventToItemsMap(): Map<string, string[]>;
    _getActionMatcher(): ActionMatcher | null | undefined;
}
