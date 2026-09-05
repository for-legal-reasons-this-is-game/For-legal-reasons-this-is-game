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
exports.TracingHeaders = void 0;
var globals_1 = require("../utils/globals");
var logger_1 = require("@posthog/browser-common/utils/logger");
var core_1 = require("@posthog/core");
var logger = (0, logger_1.createLogger)('[TracingHeaders]');
var TracingHeaders = /** @class */ (function () {
    function TracingHeaders(_instance) {
        var _this = this;
        this._instance = _instance;
        this._restoreXHRPatch = undefined;
        this._restoreFetchPatch = undefined;
        this._hostnamesForPatch = undefined;
        this._startCapturing = function () {
            var _a, _b, _c, _d;
            var hostnames = _this._syncHostnamesForPatch();
            if (!hostnames) {
                _this._stopCapturing();
                return;
            }
            if ((0, core_1.isUndefined)(_this._restoreXHRPatch)) {
                _this._restoreXHRPatch = (_b = (_a = globals_1.assignableWindow.__PosthogExtensions__) === null || _a === void 0 ? void 0 : _a.tracingHeadersPatchFns) === null || _b === void 0 ? void 0 : _b._patchXHR(hostnames, function () { return _this._instance.get_distinct_id(); }, _this._instance.sessionManager);
            }
            if ((0, core_1.isUndefined)(_this._restoreFetchPatch)) {
                _this._restoreFetchPatch = (_d = (_c = globals_1.assignableWindow.__PosthogExtensions__) === null || _c === void 0 ? void 0 : _c.tracingHeadersPatchFns) === null || _d === void 0 ? void 0 : _d._patchFetch(hostnames, function () { return _this._instance.get_distinct_id(); }, _this._instance.sessionManager);
            }
        };
    }
    TracingHeaders.prototype.initialize = function () {
        this.startIfEnabledOrStop();
    };
    TracingHeaders.prototype._loadScript = function (cb) {
        var _a, _b, _c;
        if ((_a = globals_1.assignableWindow.__PosthogExtensions__) === null || _a === void 0 ? void 0 : _a.tracingHeadersPatchFns) {
            // already loaded
            cb();
            return;
        }
        (_c = (_b = globals_1.assignableWindow.__PosthogExtensions__) === null || _b === void 0 ? void 0 : _b.loadExternalDependency) === null || _c === void 0 ? void 0 : _c.call(_b, this._instance, 'tracing-headers', function (err) {
            if (err) {
                return logger.error('failed to load script', err);
            }
            cb();
        });
    };
    TracingHeaders.prototype._getConfiguredHostnames = function () {
        var _a, _b;
        // Prefer the public `tracing_headers` option; fall back to deprecated aliases.
        return ((_b = (_a = this._instance.config.tracing_headers) !== null && _a !== void 0 ? _a : this._instance.config.addTracingHeaders) !== null && _b !== void 0 ? _b : this._instance.config.__add_tracing_headers);
    };
    TracingHeaders.prototype._syncHostnamesForPatch = function () {
        var _a;
        var hostnames = this._getConfiguredHostnames();
        if ((0, core_1.isArray)(hostnames)) {
            if ((0, core_1.isArray)(this._hostnamesForPatch)) {
                (_a = this._hostnamesForPatch).splice.apply(_a, __spreadArray([0, this._hostnamesForPatch.length], __read(hostnames), false));
            }
            else {
                this._hostnamesForPatch = __spreadArray([], __read(hostnames), false);
            }
            return hostnames.length > 0 ? this._hostnamesForPatch : undefined;
        }
        if ((0, core_1.isArray)(this._hostnamesForPatch)) {
            // we empty the array before reassignment because there may be existing
            // fetch/XHR patches reading this array. if we're in a situation where
            // we're reapplying a patch and it fails for any reason, we've at least
            // avoided sending headers to stale hostnames.
            this._hostnamesForPatch.splice(0);
        }
        this._hostnamesForPatch = hostnames || undefined;
        return this._hostnamesForPatch;
    };
    TracingHeaders.prototype._stopCapturing = function () {
        var _a, _b;
        (_a = this._restoreXHRPatch) === null || _a === void 0 ? void 0 : _a.call(this);
        (_b = this._restoreFetchPatch) === null || _b === void 0 ? void 0 : _b.call(this);
        // we don't want to call these twice so we reset them
        this._restoreXHRPatch = undefined;
        this._restoreFetchPatch = undefined;
    };
    TracingHeaders.prototype.startIfEnabledOrStop = function () {
        if (this._syncHostnamesForPatch()) {
            this._loadScript(this._startCapturing);
        }
        else {
            this._stopCapturing();
        }
    };
    return TracingHeaders;
}());
exports.TracingHeaders = TracingHeaders;
//# sourceMappingURL=tracing-headers.js.map