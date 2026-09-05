import { PostHog } from '../../../posthog-core';
export declare class FlushedSizeTracker {
    private readonly _getProperty;
    private readonly _setProperty;
    constructor(posthog: PostHog);
    trackSize(sessionId: string, size: number): void;
    currentTrackedSize(sessionId: string): number;
}
