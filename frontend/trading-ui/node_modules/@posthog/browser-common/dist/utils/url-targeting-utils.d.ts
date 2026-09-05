import type { PostHogConfig } from '@posthog/types';
export type UrlTargetingInstance = {
    config: Pick<PostHogConfig, 'get_current_url'>;
};
/**
 * Applies the `get_current_url` config hook to an already-resolved URL. Returns `defaultUrl`
 * unchanged when no override is configured, or if the override throws or returns a
 * non-string/empty value.
 *
 * Use this when the caller already has its own URL source (e.g. web experiments read
 * `window.location` via a mockable indirection); otherwise prefer `getTargetingUrl`.
 */
export declare function applyUrlTargetingOverride(instance: UrlTargetingInstance | undefined, defaultUrl: string): string;
/**
 * Resolves the URL used for client-side URL targeting: session replay URL triggers, the
 * session replay URL blocklist, survey URL conditions, product tour URL conditions, and web
 * experiment URL conditions.
 *
 * Defaults to `window.location.href`, but honors the `get_current_url` config hook so apps
 * that rewrite their URL (e.g. Electron/desktop builds, or `$current_url` rewrites in
 * `before_send`) can make targeting match the logical URL instead of the raw browser URL.
 *
 * Returns `undefined` when there is no URL available (e.g. non-browser environments).
 *
 * Called on every rrweb event by the replay URL triggers/blocklist, so a configured
 * `get_current_url` override should stay cheap.
 */
export declare function getTargetingUrl(instance: UrlTargetingInstance | undefined): string | undefined;
