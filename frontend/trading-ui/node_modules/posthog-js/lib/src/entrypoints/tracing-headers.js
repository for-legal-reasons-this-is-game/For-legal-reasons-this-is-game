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
Object.defineProperty(exports, "__esModule", { value: true });
var patch_1 = require("../extensions/replay/rrweb-plugins/patch");
var globals_1 = require("@posthog/browser-common/utils/globals");
var globals_2 = require("../utils/globals");
var constants_1 = require("../constants");
var core_1 = require("@posthog/core");
var SESSION_ID_HEADER = 'X-POSTHOG-SESSION-ID';
var WINDOW_ID_HEADER = 'X-POSTHOG-WINDOW-ID';
var DISTINCT_ID_HEADER = 'X-POSTHOG-DISTINCT-ID';
var TRACING_HEADERS = [SESSION_ID_HEADER, WINDOW_ID_HEADER, DISTINCT_ID_HEADER];
var REQUEST_INIT_KEYS = [
    'attributionReporting',
    'body',
    'browsingTopics',
    'cache',
    'credentials',
    'dispatcher',
    'duplex',
    'integrity',
    'keepalive',
    'method',
    'mode',
    'priority',
    'redirect',
    'referrer',
    'referrerPolicy',
    'signal',
    'window',
];
var hasOwnProperty = function (value, property) {
    return Object.prototype.hasOwnProperty.call(value, property);
};
var isObjectLike = function (value) {
    return (typeof value === 'object' && !(0, core_1.isNull)(value)) || (0, core_1.isFunction)(value);
};
var isRequest = function (value) {
    return typeof Request !== 'undefined' &&
        (value instanceof Request || Object.prototype.toString.call(value) === '[object Request]');
};
var getRequestUrl = function (url) {
    var _a;
    try {
        if (isRequest(url)) {
            return url.url;
        }
        // eslint-disable-next-line compat/compat
        return new URL(url instanceof URL ? url.toString() : String(url), (_a = globals_1.window === null || globals_1.window === void 0 ? void 0 : globals_1.window.location) === null || _a === void 0 ? void 0 : _a.href).toString();
    }
    catch (_b) {
        return undefined;
    }
};
// RequestInit is a WebIDL dictionary: native fetch reads known keys via property access, including inherited
// and non-enumerable properties. Avoid `{ ...init, headers }`, which can drop body/signal/duplex/etc. Instead,
// overlay only `headers` while forwarding every other lookup to the caller's original init object.
var createFetchInitWithHeaders = function (init, headers) {
    if ((0, core_1.isUndefined)(init) || (0, core_1.isNull)(init)) {
        return { headers: headers };
    }
    if (!isObjectLike(init)) {
        return undefined;
    }
    var originalInit = init;
    if (typeof Proxy === 'undefined' || typeof Reflect === 'undefined') {
        // Older browsers without Proxy still get an inherited overlay, which preserves native fetch property lookup.
        var initWithHeaders = Object.create(originalInit);
        Object.defineProperty(initWithHeaders, 'headers', {
            configurable: true,
            enumerable: true,
            value: headers,
            writable: true,
        });
        return initWithHeaders;
    }
    var target = { headers: headers };
    return new Proxy(target, {
        get: function (target, property) {
            if (hasOwnProperty(target, property)) {
                return Reflect.get(target, property);
            }
            return Reflect.get(originalInit, property, originalInit);
        },
        has: function (target, property) {
            return hasOwnProperty(target, property) || property in originalInit;
        },
        ownKeys: function (target) {
            var keys = new Set(Reflect.ownKeys(target));
            Reflect.ownKeys(originalInit).forEach(function (key) { return keys.add(key); });
            REQUEST_INIT_KEYS.forEach(function (key) {
                if (key in originalInit) {
                    keys.add(key);
                }
            });
            return Array.from(keys);
        },
        getOwnPropertyDescriptor: function (target, property) {
            var targetDescriptor = Reflect.getOwnPropertyDescriptor(target, property);
            if (targetDescriptor) {
                return targetDescriptor;
            }
            if (typeof property === 'string' && REQUEST_INIT_KEYS.includes(property) && property in originalInit) {
                return {
                    configurable: true,
                    enumerable: true,
                    get: function () { return Reflect.get(originalInit, property, originalInit); },
                };
            }
            var descriptor = Reflect.getOwnPropertyDescriptor(originalInit, property);
            return descriptor ? __assign(__assign({}, descriptor), { configurable: true }) : undefined;
        },
        getPrototypeOf: function () {
            return Object.getPrototypeOf(originalInit);
        },
    });
};
var getDistinctId = function (distinctId) {
    return (0, core_1.isFunction)(distinctId) ? distinctId() : distinctId;
};
var addTracingHeaders = function (hostnames, distinctId, sessionManager, url, headers) {
    var reqHostname;
    try {
        // we don't need to support IE11 here
        // eslint-disable-next-line compat/compat
        reqHostname = new URL(url).hostname;
    }
    catch (_a) {
        // If the URL is invalid, we skip adding tracing headers
        return false;
    }
    if (!hostnames) {
        return false;
    }
    if ((0, core_1.isArray)(hostnames) && !hostnames.includes(reqHostname)) {
        return false;
    }
    var hasAddedHeaders = false;
    if (sessionManager) {
        var _b = sessionManager.checkAndGetSessionAndWindowId(true), sessionId = _b.sessionId, windowId = _b.windowId;
        headers.set(SESSION_ID_HEADER, sessionId);
        headers.set(WINDOW_ID_HEADER, windowId);
        hasAddedHeaders = true;
    }
    var currentDistinctId = getDistinctId(distinctId);
    if (currentDistinctId && currentDistinctId !== constants_1.COOKIELESS_SENTINEL_VALUE) {
        headers.set(DISTINCT_ID_HEADER, currentDistinctId);
        hasAddedHeaders = true;
    }
    return hasAddedHeaders;
};
var patchFetch = function (hostnames, distinctId, sessionManager) {
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    return (0, patch_1.patch)(globals_1.window, 'fetch', function (originalFetch) {
        return function (url, init) {
            var originalArgs = (arguments.length > 1 ? [url, init] : [url]);
            var fetchArgs = originalArgs;
            try {
                var requestUrl = getRequestUrl(url);
                if (requestUrl) {
                    if (isRequest(url)) {
                        // For fetch(Request, init), construct a new Request so init overrides are applied and the
                        // caller's Request is not mutated. For fetch(url, init), avoid this because it exposes string
                        // bodies as ReadableStreams to downstream wrappers in Safari.
                        var req = new Request(url, init);
                        addTracingHeaders(hostnames, distinctId, sessionManager, req.url, req.headers);
                        fetchArgs = [req];
                    }
                    else {
                        var headers = new Headers(isObjectLike(init) ? init.headers : undefined);
                        if (addTracingHeaders(hostnames, distinctId, sessionManager, requestUrl, headers)) {
                            var initWithHeaders = createFetchInitWithHeaders(init, headers);
                            if (initWithHeaders) {
                                fetchArgs = [url, initWithHeaders];
                            }
                        }
                    }
                }
            }
            catch (_a) {
                fetchArgs = originalArgs;
            }
            return originalFetch.apply(this, fetchArgs);
        };
    });
};
var patchXHR = function (hostnames, distinctId, sessionManager) {
    return (0, patch_1.patch)(
    // we can assert this is present because we've checked previously
    globals_1.window.XMLHttpRequest.prototype, 'open', 
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    function (originalOpen) {
        return function (method, url, async, username, password) {
            if (async === void 0) { async = true; }
            // because this function is returned in its actual context `this` _is_ an XMLHttpRequest
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            var xhr = this;
            var headers = new Headers();
            var requestUrl = getRequestUrl(url);
            if (requestUrl) {
                addTracingHeaders(hostnames, distinctId, sessionManager, requestUrl, headers);
            }
            var result = originalOpen.call(xhr, method, url, async, username, password);
            TRACING_HEADERS.forEach(function (header) {
                var value = headers.get(header);
                if (value) {
                    try {
                        xhr.setRequestHeader(header, value);
                    }
                    catch (_a) {
                        // Do not let tracing header injection break the host app's XHR.
                    }
                }
            });
            return result;
        };
    });
};
globals_2.assignableWindow.__PosthogExtensions__ = globals_2.assignableWindow.__PosthogExtensions__ || {};
var patchFns = {
    _patchFetch: patchFetch,
    _patchXHR: patchXHR,
};
globals_2.assignableWindow.__PosthogExtensions__.tracingHeadersPatchFns = patchFns;
// we used to put tracingHeadersPatchFns on window, and now we put it on __PosthogExtensions__
// but that means that old clients which lazily load this extension are looking in the wrong place
// yuck,
// so we also put it directly on the window
// when 1.161.1 is the oldest version seen in production we can remove this
globals_2.assignableWindow.postHogTracingHeadersPatchFns = patchFns;
exports.default = patchFns;
//# sourceMappingURL=tracing-headers.js.map