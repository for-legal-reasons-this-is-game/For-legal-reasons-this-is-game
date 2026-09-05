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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PostHogLogs = void 0;
var constants_1 = require("./constants");
var config_1 = __importDefault(require("./config"));
var core_1 = require("@posthog/core");
var globals_1 = require("@posthog/browser-common/utils/globals");
var globals_2 = require("./utils/globals");
var general_utils_1 = require("@posthog/browser-common/utils/general-utils");
var logger_1 = require("@posthog/browser-common/utils/logger");
var logs_defaults_1 = require("./logs-defaults");
var request_utils_1 = require("@posthog/browser-common/utils/request-utils");
var LOGS_ENDPOINT = '/i/v1/logs';
// OTLP instrumentation-scope name for console auto-capture, distinguishing it from
// programmatic logs (which use the SDK scope) in scope-based dashboards/queries.
var CONSOLE_SCOPE_NAME = 'console';
// Safety backstop for a `_send_request` that never calls back. Set above the
// request layer's own 60s timeout so a real (slow-but-completing) request always
// settles via its callback first; this only fires on a genuinely callback-less
// send (e.g. request enqueued before load, or a transport that never reports).
var LOGS_SEND_TIMEOUT_MS = 90000;
// Mirrors the event retry queue's status-0 budget (see retry-queue.ts
// `STATUS_CODE_ZERO_MAX_RETRIES`): a request that dies before any HTTP response
// while the browser reports itself online is almost always deterministically
// blocked (ad blocker, CORS, extension), so retrying forever only burns network.
// After this many consecutive such failures we stop sending and drop batches;
// the `online` event reopens the pipe.
// NOTE: keep the constant value and the warning copy in sync with retry-queue.ts.
var MAX_CONSECUTIVE_STATUS_ZERO_FAILURES = 3;
var PostHogLogs = /** @class */ (function () {
    function PostHogLogs(_instance) {
        var _this = this;
        var _a;
        this._instance = _instance;
        this._isLogsEnabled = false;
        this._isLoaded = false;
        this._logger = (0, logger_1.createLogger)('[logs]');
        // In-memory only; records do not survive a page reload.
        this._queue = [];
        // Console auto-capture uses a dedicated core + queue (its `service.name`
        // defaults to `posthog-browser-logs`). Built lazily, only when console runs.
        this._consoleQueue = [];
        // Shared across both cores: they send to the same endpoint, so one blocker
        // verdict covers both.
        this._consecutiveStatusZeroFailures = 0;
        this._onReconnect = function () {
            var _a, _b;
            _this._consecutiveStatusZeroFailures = 0;
            (_a = _this._core) === null || _a === void 0 ? void 0 : _a.onReconnect();
            (_b = _this._consoleCore) === null || _b === void 0 ? void 0 : _b.onReconnect();
        };
        if (this._instance && ((_a = this._instance.config.logs) === null || _a === void 0 ? void 0 : _a.captureConsoleLogs)) {
            this._isLogsEnabled = true;
        }
        // Flush on reconnect rather than waiting out the retry backoff.
        if (globals_1.window) {
            (0, general_utils_1.addEventListener)(globals_1.window, 'online', this._onReconnect);
        }
    }
    // Cores are built lazily (the extension exists before `init` applies config)
    // and rebuilt when `config.logs` is swapped. Callers reset the old core first
    // so its timer can't double-flush the shared queue; a flush already in flight
    // may still re-send its head batch on a mid-swap — a duplicate, never a loss.
    PostHogLogs.prototype._buildCore = function (getQueue, setQueue, opts, scopeName) {
        var _this = this;
        var _a, _b;
        var config = (0, logs_defaults_1.resolveLogsConfig)((_b = (_a = this._instance) === null || _a === void 0 ? void 0 : _a.config) === null || _b === void 0 ? void 0 : _b.logs, opts);
        var core = new core_1.PostHogLogs(this._createHost(getQueue, setQueue), config, this._logger, function () { return _this._getSdkContext(); }, function (fn) { return fn(); }, undefined, scopeName);
        return [core, config];
    };
    PostHogLogs.prototype._getCore = function () {
        var _a;
        var _this = this;
        var _b, _c, _d;
        var logsConfig = (_c = (_b = this._instance) === null || _b === void 0 ? void 0 : _b.config) === null || _c === void 0 ? void 0 : _c.logs;
        if (!this._core || this._resolvedFrom !== logsConfig) {
            (_d = this._core) === null || _d === void 0 ? void 0 : _d.reset();
            this._resolvedFrom = logsConfig;
            _a = __read(this._buildCore(function () { return _this._queue; }, function (q) {
                _this._queue = q;
            }), 2), this._core = _a[0], this._resolvedConfig = _a[1];
        }
        return this._core;
    };
    // Like `_getCore`, but with the console service name + scope, backed by `_consoleQueue`.
    PostHogLogs.prototype._getConsoleCore = function () {
        var _a;
        var _this = this;
        var _b, _c, _d;
        var logsConfig = (_c = (_b = this._instance) === null || _b === void 0 ? void 0 : _b.config) === null || _c === void 0 ? void 0 : _c.logs;
        if (!this._consoleCore || this._consoleResolvedFrom !== logsConfig) {
            (_d = this._consoleCore) === null || _d === void 0 ? void 0 : _d.reset();
            this._consoleResolvedFrom = logsConfig;
            _a = __read(this._buildCore(function () { return _this._consoleQueue; }, function (q) {
                _this._consoleQueue = q;
            }, { serviceNameDefault: 'posthog-browser-logs', consoleCapture: true }, CONSOLE_SCOPE_NAME), 2), this._consoleCore = _a[0], this._consoleResolvedConfig = _a[1];
        }
        return this._consoleCore;
    };
    PostHogLogs.prototype.initialize = function () {
        this.loadIfEnabled();
    };
    PostHogLogs.prototype.onRemoteConfig = function (result) {
        var _a;
        if (!result.ok) {
            // Failure behaves like a response without a logs key.
            return;
        }
        var logCapture = (_a = result.config.logs) === null || _a === void 0 ? void 0 : _a.captureConsoleLogs;
        if ((0, core_1.isNullish)(logCapture) || !logCapture) {
            return;
        }
        this._isLogsEnabled = true;
        this.loadIfEnabled();
    };
    PostHogLogs.prototype.reset = function () {
        var _a, _b;
        this._queue = [];
        (_a = this._core) === null || _a === void 0 ? void 0 : _a.reset();
        this._consoleQueue = [];
        (_b = this._consoleCore) === null || _b === void 0 ? void 0 : _b.reset();
        this._consecutiveStatusZeroFailures = 0;
    };
    PostHogLogs.prototype.captureLog = function (options) {
        this._getCore().captureLog(options);
    };
    // Console auto-capture (the lazy `logs` chunk) routes here so its records run
    // through the shared core pipeline and carry `service.name: posthog-browser-logs`.
    /** @internal */
    PostHogLogs.prototype._captureConsoleLog = function (options) {
        this._getConsoleCore().captureLog(options);
    };
    Object.defineProperty(PostHogLogs.prototype, "logger", {
        get: function () {
            var _this = this;
            if (!this._capture_logger) {
                this._capture_logger = {
                    trace: function (body, attributes) { return _this.captureLog({ body: body, level: 'trace', attributes: attributes }); },
                    debug: function (body, attributes) { return _this.captureLog({ body: body, level: 'debug', attributes: attributes }); },
                    info: function (body, attributes) { return _this.captureLog({ body: body, level: 'info', attributes: attributes }); },
                    warn: function (body, attributes) { return _this.captureLog({ body: body, level: 'warn', attributes: attributes }); },
                    error: function (body, attributes) { return _this.captureLog({ body: body, level: 'error', attributes: attributes }); },
                    fatal: function (body, attributes) { return _this.captureLog({ body: body, level: 'fatal', attributes: attributes }); },
                };
            }
            return this._capture_logger;
        },
        enumerable: false,
        configurable: true
    });
    // An explicit transport drains the whole queue in one request over that transport
    // (core's batched flush can't force a transport, and the unload sendBeacon must be
    // synchronous). No transport → core's batched, 413-aware, retrying flush.
    PostHogLogs.prototype.flushLogs = function (transport) {
        var _this = this;
        if (transport) {
            this._flushViaTransport(transport);
            return;
        }
        if (this._core) {
            void this._core.flush().catch(function (err) { return _this._logger.error('PostHog logs flush failed:', err); });
        }
        if (this._consoleCore) {
            void this._consoleCore.flush().catch(function (err) { return _this._logger.error('PostHog logs flush failed:', err); });
        }
    };
    PostHogLogs.prototype.loadIfEnabled = function () {
        var _this = this;
        if (!this._isLogsEnabled || this._isLoaded) {
            return;
        }
        var phExtensions = globals_2.assignableWindow === null || globals_2.assignableWindow === void 0 ? void 0 : globals_2.assignableWindow.__PosthogExtensions__;
        if (!phExtensions) {
            this._logger.error('PostHog Extensions not found.');
            return;
        }
        var loadExternalDependency = phExtensions.loadExternalDependency;
        if (!loadExternalDependency) {
            this._logger.error(constants_1.LOAD_EXT_NOT_FOUND);
            return;
        }
        loadExternalDependency(this._instance, 'logs', function (err) {
            var _a;
            if (err || !((_a = phExtensions.logs) === null || _a === void 0 ? void 0 : _a.initializeLogs)) {
                _this._logger.error('Could not load logs script', err);
            }
            else {
                phExtensions.logs.initializeLogs(_this._instance);
                _this._isLoaded = true;
            }
        });
    };
    // Host adapter for core's `PostHogLogs`; structurally checked against `LogsHost`
    // at the `new CorePostHogLogs` call, so no explicit annotation is needed. The
    // queue accessors are parameterized so the programmatic and console instances
    // each bind to their own queue.
    PostHogLogs.prototype._createHost = function (getQueue, setQueue) {
        var _this = this;
        var ph = this._instance;
        return {
            // The browser gates capture through `is_capturing()` (see `optedOut`).
            get isDisabled() {
                return false;
            },
            get optedOut() {
                return !ph.is_capturing();
            },
            // Live queue by reference; core mutates it in place and persists via the setter.
            getPersistedProperty: function (key) {
                return key === core_1.PostHogPersistedProperty.LogsQueue ? getQueue() : undefined;
            },
            setPersistedProperty: function (key, value) {
                var _a;
                if (key === core_1.PostHogPersistedProperty.LogsQueue) {
                    setQueue((_a = value) !== null && _a !== void 0 ? _a : []);
                }
            },
            _sendLogsBatch: function (payload) { return _this._sendLogsBatch(payload); },
            getLibraryId: function () { return config_1.default.LIB_NAME; },
            getLibraryVersion: function () { return config_1.default.LIB_VERSION; },
        };
    };
    PostHogLogs.prototype._sendLogsBatch = function (payload) {
        var _this = this;
        // eslint-disable-next-line compat/compat
        return new Promise(function (resolve) {
            if ((0, request_utils_1.isStatusZeroFailureCircuitBreakerTripped)(_this._consecutiveStatusZeroFailures, MAX_CONSECUTIVE_STATUS_ZERO_FAILURES)) {
                // Tripped: drop the batch without touching the network. `fatal`
                // advances the queue so records don't pile up while blocked.
                // The `onLine` guard ensures genuine offline periods still queue
                // for the reconnect flush instead of being fatally dropped.
                resolve({ kind: 'fatal', error: new Error('logs endpoint is unreachable, dropping batch') });
                return;
            }
            var settled = false;
            var settle = function (outcome) {
                if (settled) {
                    return;
                }
                settled = true;
                clearTimeout(timer);
                resolve(outcome);
            };
            // Backstop for `_send_request` paths that never call back, so the promise
            // always settles and core's flush can't wedge. Keeps records for retry.
            var timer = setTimeout(function () { return settle({ kind: 'retry-later', error: new Error('logs request timed out') }); }, LOGS_SEND_TIMEOUT_MS);
            _this._instance._send_request({
                method: 'POST',
                url: _this._logsUrl(),
                data: payload,
                compression: 'best-available',
                batchKey: 'logs',
                // Notify on the drop paths (not loaded, rate limited) so they retry, not stall.
                fireCallbackOnDrop: true,
                callback: function (response) {
                    var _a;
                    var status = response.statusCode;
                    _this._trackEndpointReachability(status);
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
                            error: (_a = response.error) !== null && _a !== void 0 ? _a : new Error("logs request failed with status ".concat(status)),
                        });
                    }
                    else {
                        // Client error (4xx): won't succeed on retry, drop.
                        settle({ kind: 'fatal', error: new Error("logs request failed with status ".concat(status)) });
                    }
                },
            });
        });
    };
    // Feeds the status-0 circuit breaker checked at the top of `_sendLogsBatch`.
    PostHogLogs.prototype._trackEndpointReachability = function (statusCode) {
        var _this = this;
        // Before `init` completes, `_send_request` synthesizes `{ statusCode: 0 }`
        // without any network attempt (the `fireCallbackOnDrop` path), so only
        // post-load failures count — a deferred init must not arrive to an
        // already-tripped breaker. `__loaded` flips on init, not on a successful
        // request, so a blocked-from-the-start page still trips as intended.
        if (statusCode === 0 && !this._instance.__loaded) {
            return;
        }
        this._consecutiveStatusZeroFailures = (0, request_utils_1.updateStatusZeroFailureCount)(statusCode, this._consecutiveStatusZeroFailures, MAX_CONSECUTIVE_STATUS_ZERO_FAILURES, function () {
            return _this._logger.warn('Log requests are failing before receiving an HTTP response; this can happen due to network issues, CORS, browser blocking, or ad blockers. Stopped sending logs; will try again when connectivity changes.');
        });
    };
    // Drains both the programmatic and console queues over the given transport.
    // Each queue carries its own resolved config so the two `service.name`s are
    // preserved. Non-empty queue → its core was built → its resolved config is set,
    // so the length guards also avoid lazily building an unused core for config.
    // TODO: future optimization — merge both into one multi-`resourceLogs` payload
    //       so a page-unload only fires a single sendBeacon instead of two.
    PostHogLogs.prototype._flushViaTransport = function (transport) {
        var _this = this;
        if (this._queue.length > 0) {
            // Invariant: _resolvedConfig is set whenever _queue has items.
            this._drainQueueViaTransport(transport, this._queue, this._resolvedConfig, config_1.default.LIB_NAME, function (q) {
                _this._queue = q;
            });
        }
        if (this._consoleQueue.length > 0) {
            // Invariant: _consoleResolvedConfig is set whenever _consoleQueue has items.
            this._drainQueueViaTransport(transport, this._consoleQueue, this._consoleResolvedConfig, CONSOLE_SCOPE_NAME, function (q) {
                _this._consoleQueue = q;
            });
        }
    };
    PostHogLogs.prototype._drainQueueViaTransport = function (transport, queue, config, scopeName, setQueue) {
        if (queue.length === 0) {
            return;
        }
        var records = queue.map(function (e) { return e.record; });
        setQueue([]);
        // Shared with the core flush path so resource attributes can't drift. The
        // scope name labels the stream (console vs SDK); `telemetry.sdk.name` stays
        // the SDK id (`Config.LIB_NAME`) regardless.
        var payload = (0, core_1.buildOtlpLogsPayload)(records, (0, core_1.buildResourceAttributes)(config, config_1.default.LIB_NAME, config_1.default.LIB_VERSION), scopeName, config_1.default.LIB_VERSION);
        // Intentionally bypasses the circuit breaker and does not feed
        // `_trackEndpointReachability`: this is a best-effort "last gasp" send
        // (page unload or explicit transport flush) where `sendBeacon` in particular
        // is sometimes honoured even by blockers, and the callback-less path means
        // we can't track the outcome anyway.
        this._instance._send_request({
            method: 'POST',
            url: this._logsUrl(),
            data: payload,
            compression: 'best-available',
            batchKey: 'logs',
            transport: transport,
        });
    };
    PostHogLogs.prototype._logsUrl = function () {
        return (this._instance.requestRouter.endpointFor('api', LOGS_ENDPOINT) +
            '?token=' +
            encodeURIComponent(this._instance.config.token));
    };
    PostHogLogs.prototype._getSdkContext = function () {
        var _a;
        var context = {};
        context.distinctId = this._instance.get_distinct_id();
        if (this._instance.sessionManager) {
            var _b = this._instance.sessionManager.checkAndGetSessionAndWindowId(true), sessionId = _b.sessionId, windowId = _b.windowId, sessionStartTimestamp = _b.sessionStartTimestamp, lastActivityTimestamp = _b.lastActivityTimestamp;
            context.sessionId = sessionId;
            context.windowId = windowId;
            if (!(0, core_1.isNullish)(sessionStartTimestamp)) {
                context.sessionStartTimestamp = sessionStartTimestamp;
            }
            if (!(0, core_1.isNullish)(lastActivityTimestamp)) {
                context.lastActivityTimestamp = lastActivityTimestamp;
            }
        }
        if ((_a = globals_2.assignableWindow === null || globals_2.assignableWindow === void 0 ? void 0 : globals_2.assignableWindow.location) === null || _a === void 0 ? void 0 : _a.href) {
            context.currentUrl = this._instance.config.disable_capture_url_hashes
                ? (0, core_1.stripUrlHash)(globals_2.assignableWindow.location.href)
                : globals_2.assignableWindow.location.href;
        }
        if (this._instance.featureFlags) {
            var flags = this._instance.featureFlags.getFlags();
            if (flags && flags.length > 0) {
                context.activeFeatureFlags = flags;
            }
        }
        return context;
    };
    return PostHogLogs;
}());
exports.PostHogLogs = PostHogLogs;
//# sourceMappingURL=posthog-logs.js.map