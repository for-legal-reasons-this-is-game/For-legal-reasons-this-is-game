import { PostHog } from './posthog-core';
import type { CaptureMetricOptions } from './types';
import { Extension } from './extensions/types';
/**
 * The `posthog.metrics` extension: a statsd-style pre-aggregating client.
 *
 * ```ts
 * posthog.metrics.count('orders_created', 1)
 * posthog.metrics.gauge('active_connections', 42)
 * posthog.metrics.histogram('api_latency', 187, { unit: 'ms' })
 * ```
 *
 * Aggregation, series identity, and flush policy live in core's
 * `PostHogMetrics`; this wrapper adapts it to the browser request layer.
 */
export declare class PostHogMetrics implements Extension {
    private readonly _instance;
    private readonly _logger;
    private _core;
    private _resolvedFrom;
    constructor(_instance: PostHog);
    initialize(): void;
    private _getCore;
    /** Add to a counter — things that only go up. Value defaults to 1. */
    count(name: string, value?: number, options?: CaptureMetricOptions): void;
    /** Record the current value of something that goes up and down. */
    gauge(name: string, value: number, options?: CaptureMetricOptions): void;
    /** Record one observation of a distribution (latency, payload size). */
    histogram(name: string, value: number, options?: CaptureMetricOptions): void;
    /**
     * Sends the aggregated window now. With a transport, the window is
     * drained synchronously — bypassing the flush serializer, which could be
     * awaiting an in-flight send that will never finish during unload — and
     * the payload is handed to that transport in the same tick, so the
     * pagehide `sendBeacon` drain survives the page going away. A drained
     * window is not retried; the page is gone either way.
     */
    flush(transport?: 'XHR' | 'fetch' | 'sendBeacon'): Promise<void>;
    reset(): void;
    private _createHost;
    private _sendMetricsBatch;
    private _metricsUrl;
}
