import type { LogCaptureOptions } from '@posthog/types';
import type { ResolvedPostHogLogsConfig } from '@posthog/core';
/**
 * Resolves the public `logs` config into the shape core `PostHogLogs` consumes.
 *
 * `serviceNameDefault` is the console instance's `service.name` fallback
 * (`posthog-browser-logs`); the programmatic instance falls back to core's
 * `unknown_service`. `consoleCapture` disables the per-interval rate cap entirely
 * (even when the user set `maxLogsPerInterval`), so console capture is never
 * rate-dropped — the deep buffer is its only bound.
 */
export declare function resolveLogsConfig(config: LogCaptureOptions | undefined, opts?: {
    serviceNameDefault?: string;
    consoleCapture?: boolean;
}): ResolvedPostHogLogsConfig;
