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
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
Object.defineProperty(exports, "__esModule", { value: true });
var globals_1 = require("../utils/globals");
var core_1 = require("@posthog/core");
var LOG_BODY_SIZE_LIMIT = 10000;
var LOG_ATTRIBUTES_LIMIT = 50;
var appendWithLimit = function (parts, text, budget) {
    if (budget.remaining <= 0) {
        budget.truncated = true;
        return false;
    }
    if (text.length <= budget.remaining) {
        parts.push(text);
        budget.remaining -= text.length;
        return true;
    }
    parts.push(text.slice(0, budget.remaining));
    budget.remaining = 0;
    budget.truncated = true;
    return false;
};
var stringifyStringWithLimit = function (value, parts, budget) {
    var serialized = JSON.stringify(value);
    if (serialized.length <= budget.remaining) {
        return appendWithLimit(parts, serialized, budget);
    }
    budget.truncated = true;
    if (budget.remaining < 2) {
        return false;
    }
    var low = 0;
    var high = Math.min(value.length, budget.remaining - 2);
    while (low < high) {
        var mid = Math.ceil((low + high) / 2);
        if (JSON.stringify(value.slice(0, mid)).length <= budget.remaining) {
            low = mid;
        }
        else {
            high = mid - 1;
        }
    }
    return appendWithLimit(parts, JSON.stringify(value.slice(0, low)), budget);
};
var isJSONSerializablePrimitive = function (value) {
    return typeof value !== 'undefined' &&
        typeof value !== 'function' &&
        typeof value !== 'symbol' &&
        typeof value !== 'bigint';
};
var isNumberOrBoolean = function (value) {
    try {
        return (0, core_1.isNumber)(value) || (0, core_1.isBoolean)(value);
    }
    catch (_a) {
        return false;
    }
};
var collectAttributeValue = function (key, value, collector) {
    if (collector.truncated) {
        return;
    }
    collector.keysRemaining -= 1;
    collector.sizeRemaining -= String(value).length + key.length;
    if (collector.keysRemaining <= 0 || collector.sizeRemaining <= 0) {
        collector.truncated = true;
        collector.result['attributes_truncated'] = true;
        return;
    }
    collector.result[key] = value;
};
var collectFlattenedAttributes = function (value, key, collector) {
    if (collector.truncated) {
        return;
    }
    if ((0, core_1.isObject)(value)) {
        if (collector.seen.has(value)) {
            collectAttributeValue(key || 'circular', '[Circular]', collector);
            return;
        }
        collector.seen.add(value);
        try {
            for (var childKey in value) {
                try {
                    if (!Object.prototype.hasOwnProperty.call(value, childKey)) {
                        continue;
                    }
                    var childValue = value[childKey];
                    collectFlattenedAttributes(childValue, key ? "".concat(key, ".").concat(childKey) : childKey, collector);
                    if (collector.truncated) {
                        return;
                    }
                }
                catch (_a) {
                    continue;
                }
            }
        }
        catch (_b) {
            // we'll omit this object's properties considering we can't enumerate them
        }
        return;
    }
    collectAttributeValue(key, value, collector);
};
var stringifyValueWithLimit = function (value, parts, budget, seen, inArray, attributeCollector) {
    if (inArray === void 0) { inArray = false; }
    if (!isJSONSerializablePrimitive(value)) {
        return inArray ? appendWithLimit(parts, 'null', budget) : true;
    }
    if ((0, core_1.isNull)(value) || isNumberOrBoolean(value)) {
        return appendWithLimit(parts, JSON.stringify(value), budget);
    }
    if (typeof value === 'string') {
        return stringifyStringWithLimit(value, parts, budget);
    }
    if (!(0, core_1.isObject)(value) && !(0, core_1.isArray)(value)) {
        return appendWithLimit(parts, JSON.stringify(value), budget);
    }
    if (seen.has(value)) {
        return stringifyStringWithLimit('[Circular]', parts, budget);
    }
    seen.add(value);
    try {
        var toJSON = value.toJSON;
        if ((0, core_1.isFunction)(toJSON)) {
            return stringifyValueWithLimit(toJSON.call(value), parts, budget, seen, inArray);
        }
    }
    catch (_a) {
        // If toJSON can't be read or throws, fall through to safe property enumeration.
    }
    try {
        var objectTag = Object.prototype.toString.call(value);
        if (objectTag === '[object String]') {
            return stringifyStringWithLimit(String(value.valueOf()), parts, budget);
        }
        if (objectTag === '[object Number]' || objectTag === '[object Boolean]') {
            return appendWithLimit(parts, JSON.stringify(value.valueOf()), budget);
        }
    }
    catch (_b) {
        // If Object.prototype.toString or valueOf throws, fall through to safe property enumeration.
    }
    if (value instanceof Error) {
        var errorObject = {};
        try {
            for (var key in value) {
                if (Object.prototype.hasOwnProperty.call(value, key)) {
                    errorObject[key] = value[key];
                }
            }
        }
        catch (_c) { }
        try {
            errorObject.name = value.name;
        }
        catch (_d) { }
        try {
            errorObject.message = value.message;
        }
        catch (_e) { }
        try {
            errorObject.stack = value.stack;
        }
        catch (_f) { }
        return stringifyValueWithLimit(errorObject, parts, budget, seen, inArray, attributeCollector);
    }
    if ((0, core_1.isArray)(value)) {
        if (!appendWithLimit(parts, '[', budget)) {
            return false;
        }
        for (var i = 0; i < value.length; i++) {
            if (i > 0 && !appendWithLimit(parts, ',', budget)) {
                return false;
            }
            var item = void 0;
            try {
                item = value[i];
            }
            catch (_g) {
                item = undefined;
            }
            if (!stringifyValueWithLimit(item, parts, budget, seen, true)) {
                return false;
            }
        }
        return appendWithLimit(parts, ']', budget);
    }
    if (!appendWithLimit(parts, '{', budget)) {
        return false;
    }
    var isFirst = true;
    try {
        for (var key in value) {
            if (!Object.prototype.hasOwnProperty.call(value, key)) {
                continue;
            }
            if (budget.remaining <= 0) {
                budget.truncated = true;
                return false;
            }
            var propertyValue = void 0;
            try {
                propertyValue = value[key];
            }
            catch (_h) {
                continue;
            }
            if (attributeCollector) {
                try {
                    collectFlattenedAttributes(propertyValue, key, attributeCollector);
                }
                catch (_j) {
                    // we'll omit this object's attributes considering we can't read them safely
                }
            }
            if (!isJSONSerializablePrimitive(propertyValue)) {
                continue;
            }
            var propertyPrefix = "".concat(isFirst ? '' : ',').concat(JSON.stringify(key), ":");
            if (propertyPrefix.length >= budget.remaining) {
                budget.truncated = true;
                return false;
            }
            var partsBeforeProperty = parts.length;
            var remainingBeforeProperty = budget.remaining;
            var truncatedBeforeProperty = budget.truncated;
            if (!appendWithLimit(parts, propertyPrefix, budget)) {
                return false;
            }
            var partsBeforeValue = parts.length;
            var serialized = stringifyValueWithLimit(propertyValue, parts, budget, seen, false);
            var truncatedAfterValue = budget.truncated;
            if (parts.length === partsBeforeValue) {
                parts.length = partsBeforeProperty;
                budget.remaining = remainingBeforeProperty;
                budget.truncated = serialized ? truncatedBeforeProperty : truncatedAfterValue;
                if (!serialized) {
                    return false;
                }
                continue;
            }
            isFirst = false;
            if (!serialized) {
                return false;
            }
        }
    }
    catch (_k) {
        // we'll omit this object's properties considering we can't enumerate them
    }
    return appendWithLimit(parts, '}', budget);
};
var stringifyArgsSafely = function (args, sizeLimit) {
    var parts = [];
    var budget = { remaining: sizeLimit, truncated: false };
    var attributeCollector = (0, core_1.isObject)(args[0])
        ? {
            result: {},
            keysRemaining: LOG_ATTRIBUTES_LIMIT,
            sizeRemaining: LOG_BODY_SIZE_LIMIT,
            truncated: false,
            seen: new WeakSet([args[0]]),
        }
        : undefined;
    for (var i = 0; i < args.length; i++) {
        if (i > 0 && !appendWithLimit(parts, ' ', budget)) {
            break;
        }
        if (!stringifyValueWithLimit(args[i], parts, budget, new WeakSet(), false, i === 0 ? attributeCollector : undefined)) {
            break;
        }
    }
    return {
        body: parts.join('') + (budget.truncated ? '...' : ''),
        truncated: budget.truncated,
        attributes: (attributeCollector === null || attributeCollector === void 0 ? void 0 : attributeCollector.result) || {},
    };
};
// Console method → OTLP severity level. `log` and `info` both map to `info`;
// the originating method is preserved separately via the `log.source` attribute.
var LEVEL_MAP = {
    debug: 'debug',
    log: 'info',
    warn: 'warn',
    error: 'error',
    info: 'info',
};
var originalConsoleMethod = function (method) {
    while (method === null || method === void 0 ? void 0 : method.__rrweb_original__) {
        method = method.__rrweb_original__;
    }
    return method;
};
var initializeLogs = function (posthog) {
    var e_1, _a;
    // `host` is carried here because the core SDK context has no equivalent. Session
    // attributes (window.id, sessionStartTimestamp, lastActivityTimestamp) are added
    // downstream by the core pipeline from the SDK context, alongside sessionId.
    var attributes = { host: globals_1.assignableWindow.location.host };
    // Re-entrancy guard: the capture path itself logs — `_captureConsoleLog` calls into
    // session management, which emits internal debug lines through PostHog's own logger,
    // which in turn writes to the (now wrapped) console. Without this flag that would
    // re-enter capture and recurse until the stack overflows.
    var isCapturingLog = false;
    var _loop_1 = function (level) {
        var logWrapper = function (originalConsoleLog) {
            return function () {
                var _a;
                var args = [];
                for (var _i = 0; _i < arguments.length; _i++) {
                    args[_i] = arguments[_i];
                }
                // Tracks whether *this* invocation acquired the re-entrancy guard, so that a
                // nested console call which skips capture doesn't release the guard early and
                // reopen the capture path while the outer invocation is still running.
                var acquiredGuard = false;
                try {
                    if (args.length > 0 && !isCapturingLog && posthog.is_capturing()) {
                        isCapturingLog = true;
                        acquiredGuard = true;
                        var _b = stringifyArgsSafely(args, LOG_BODY_SIZE_LIMIT), body = _b.body, truncated = _b.truncated, flattenedAttributes = _b.attributes;
                        var logAttributes = __assign(__assign({}, attributes), (truncated ? { body_truncated: 'true' } : {}));
                        // The core pipeline adds posthogDistinctId and url.full from the SDK context.
                        (_a = posthog.logs) === null || _a === void 0 ? void 0 : _a._captureConsoleLog({
                            level: LEVEL_MAP[level],
                            body: body,
                            attributes: __assign(__assign({ 'log.source': "console.".concat(level) }, logAttributes), flattenedAttributes),
                        });
                    }
                }
                catch (_c) {
                    // Capture must never break the page's own console output, so the
                    // real console call below always runs even if capture throws.
                }
                finally {
                    if (acquiredGuard) {
                        isCapturingLog = false;
                    }
                    originalConsoleLog.apply(globals_1.assignableWindow.console, args);
                }
            };
        };
        var originalConsoleLog = globals_1.assignableWindow.console[level];
        var wrapped = logWrapper(originalConsoleLog);
        wrapped.__rrweb_original__ = originalConsoleMethod(originalConsoleLog);
        globals_1.assignableWindow.console[level] = wrapped;
    };
    try {
        for (var _b = __values(Object.keys(LEVEL_MAP)), _c = _b.next(); !_c.done; _c = _b.next()) {
            var level = _c.value;
            _loop_1(level);
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
        }
        finally { if (e_1) throw e_1.error; }
    }
};
globals_1.assignableWindow.__PosthogExtensions__ = globals_1.assignableWindow.__PosthogExtensions__ || {};
globals_1.assignableWindow.__PosthogExtensions__.logs = { initializeLogs: initializeLogs };
//# sourceMappingURL=logs.js.map