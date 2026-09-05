import { Survey } from '../posthog-surveys-types';
import { PostHog } from '../posthog-core';
import { ActivationOutcome, EventReceiver } from './event-receiver';
import { createLogger } from '@posthog/browser-common/utils/logger';
export declare class SurveyEventReceiver extends EventReceiver<Survey> {
    constructor(instance: PostHog);
    protected _getActivatedKey(): string;
    protected _getActivatedSessionKey(): string;
    protected _getShownEventName(): string;
    protected _getItems(callback: (items: Survey[]) => void): void;
    protected _cancelPendingItem(itemId: string): void;
    protected _getLogger(): ReturnType<typeof createLogger>;
    protected _setActivatedItems(eligibleItems: string[]): void;
    protected _setActivatedSession(sessionId: string): void;
    protected _clearActivatedSession(): void;
    protected _isItemPermanentlyIneligible(): boolean;
    protected _activationOutcome(event: string, itemId: string): ActivationOutcome;
    getSurveys(): string[];
    getEventToSurveys(): Map<string, string[]>;
}
