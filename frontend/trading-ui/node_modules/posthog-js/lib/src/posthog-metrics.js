"use strict";
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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PostHogMetrics = void 0;
var config_1 = __importDefault(require("./config"));
var core_1 = require("@posthog/core");
var logger_1 = require("@posthog/browser-common/utils/logger");
var METRICS_ENDPOINT = '/i/v1/metrics';
// Safety backstop for a `_send_request` that never calls back — same policy
// as logs: above the request layer's own 60s timeout so a real request
// always settles via its callback first.
var METRICS_SEND_TIMEOUT_MS = 90000;
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
var PostHogMetrics = /** @class */ (function () {
    function PostHogMetrics(_instance) {
        this._instance = _instance;
        this._logger = (0, logger_1.createLogger)('[metrics]');
    }
    // Nothing to set up eagerly — the aggregator builds lazily on the first
    // capture so it sees post-init config. Present so the class satisfies the
    // weakly-typed `Extension` contract.
    PostHogMetrics.prototype.initialize = function () { };
    // The extension is constructed before `init` applies config, so build the
    // core lazily and rebuild when `config.metrics` is swapped (e.g. via
    // `set_config`). A rebuild drops the old core's pending window — acceptable
    // for a config swap, which is rare and usually happens at startup.
    PostHogMetrics.prototype._getCore = function () {
        var _a, _b, _c;
        var metricsConfig = (_b = (_a = this._instance) === null || _a === void 0 ? void 0 : _a.config) === null || _b === void 0 ? void 0 : _b.metrics;
        if (!this._core || this._resolvedFrom !== metricsConfig) {
            (_c = this._core) === null || _c === void 0 ? void 0 : _c.reset();
            this._resolvedFrom = metricsConfig;
            this._core = new core_1.PostHogMetrics(this._createHost(), (0, core_1.resolveMetricsConfig)(metricsConfig), this._logger);
        }
        return this._core;
    };
    /** Add to a counter — things that only go up. Value defaults to 1. */
    PostHogMetrics.prototype.count = function (name, value, options) {
        if (value === void 0) { value = 1; }
        this._getCore().count(name, value, options);
    };
    /** Record the current value of something that goes up and down. */
    PostHogMetrics.prototype.gauge = function (name, value, options) {
        this._getCore().gauge(name, value, options);
    };
    /** Record one observation of a distribution (latency, payload size). */
    PostHogMetrics.prototype.histogram = function (name, value, options) {
        this._getCore().histogram(name, value, options);
    };
    /**
     * Sends the aggregated window now. With a transport, the window is
     * drained synchronously — bypassing the flush serializer, which could be
     * awaiting an in-flight send that will never finish during unload — and
     * the payload is handed to that transport in the same tick, so the
     * pagehide `sendBeacon` drain survives the page going away. A drained
     * window is not retried; the page is gone either way.
     */
    PostHogMetrics.prototype.flush = function (transport) {
        var _this = this;
        if (!this._core) {
            return Promise.resolve();
        }
        if (transport) {
            var payload = this._core.drainWindow();
            if (payload) {
                void this._sendMetricsBatch(payload, transport);
            }
            return Promise.resolve();
        }
        return this._core.flush().catch(function (err) { return _this._logger.error('PostHog metrics flush failed:', err); });
    };
    PostHogMetrics.prototype.reset = function () {
        var _a;
        (_a = this._core) === null || _a === void 0 ? void 0 : _a.reset();
    };
    // Host adapter for core's `PostHogMetrics`; structurally checked against
    // `MetricsHost` at the `new CorePostHogMetrics` call.
    PostHogMetrics.prototype._createHost = function () {
        var ph = this._instance;
        var self = this;
        return {
            get isDisabled() {
                return false;
            },
            // The browser gates capture through `is_capturing()`.
            get optedOut() {
                return !ph.is_capturing();
            },
            _sendMetricsBatch: function (payload) { return self._sendMetricsBatch(payload); },
            getLibraryId: function () { return config_1.default.LIB_NAME; },
            getLibraryVersion: function () { return config_1.default.LIB_VERSION; },
        };
    };
    PostHogMetrics.prototype._sendMetricsBatch = function (payload, transport) {
        var _this = this;
        // eslint-disable-next-line compat/compat
        return new Promise(function (resolve) {
            var settled = false;
            var settle = function (outcome) {
                if (settled) {
                    return;
                }
                settled = true;
                clearTimeout(timer);
                resolve(outcome);
            };
            // Backstop for `_send_request` paths that never call back, so the
            // promise always settles and core's flush can't wedge.
            var timer = setTimeout(function () { return settle({ kind: 'retry-later', error: new Error('metrics request timed out') }); }, METRICS_SEND_TIMEOUT_MS);
            _this._instance._send_request(__assign(__assign({ method: 'POST', url: _this._metricsUrl(), data: payload, compression: 'best-available', batchKey: 'metrics' }, (transport && { transport: transport })), { 
                // Notify on the drop paths (not loaded, rate limited) so they retry, not stall.
                fireCallbackOnDrop: true, callback: function (response) {
                    var _a;
                    var status = response.statusCode;
                    if (status >= 200 && status < 300) {
                        settle({ kind: 'ok' });
                    }
                    else if (status === 413) {
                        settle({ kind: 'too-large' });
                    }
                    else if (status === 0 || status === 429 || status >= 500) {
                        // Transient (network / rate-limit / server error): keep and retry.
                        settle({
                            kind: 'retry-later',
                            error: (_a = response.error) !== null && _a !== void 0 ? _a : new Error("metrics request failed with status ".concat(status)),
                        });
                    }
                    else {
                        // Client error (4xx): won't succeed on retry, drop.
                        settle({ kind: 'fatal', error: new Error("metrics request failed with status ".concat(status)) });
                    }
                } }));
        });
    };
    PostHogMetrics.prototype._metricsUrl = function () {
        return (this._instance.requestRouter.endpointFor('api', METRICS_ENDPOINT) +
            '?token=' +
            encodeURIComponent(this._instance.config.token));
    };
    return PostHogMetrics;
}());
exports.PostHogMetrics = PostHogMetrics;
//# sourceMappingURL=posthog-metrics.js.map