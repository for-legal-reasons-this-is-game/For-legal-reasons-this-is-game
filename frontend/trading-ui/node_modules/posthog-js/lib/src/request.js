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
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g = Object.create((typeof Iterator === "function" ? Iterator : Object).prototype);
    return g.next = verb(0), g["throw"] = verb(1), g["return"] = verb(2), typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (g && (g = 0, op[0] && (_ = 0)), _) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
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
exports.request = exports.extendURLParams = exports.SUPPORTS_REQUEST = exports.jsonStringify = void 0;
var general_utils_1 = require("@posthog/browser-common/utils/general-utils");
var config_1 = __importDefault(require("./config"));
var types_1 = require("./types");
var request_utils_1 = require("@posthog/browser-common/utils/request-utils");
Object.defineProperty(exports, "jsonStringify", { enumerable: true, get: function () { return request_utils_1.jsonStringify; } });
var logger_1 = require("@posthog/browser-common/utils/logger");
var globals_1 = require("@posthog/browser-common/utils/globals");
var fflate_1 = require("fflate");
var encode_utils_1 = require("@posthog/browser-common/utils/encode-utils");
var core_1 = require("@posthog/core");
exports.SUPPORTS_REQUEST = !!globals_1.XMLHttpRequest || !!globals_1.fetch;
var CONTENT_TYPE_PLAIN = 'text/plain';
var CONTENT_TYPE_JSON = 'application/json';
var CONTENT_TYPE_FORM = 'application/x-www-form-urlencoded';
var SIXTY_FOUR_KILOBYTES = 64 * 1024;
/*
 fetch will fail if we request keepalive with a body greater than 64kb
 sets the threshold lower than that so that
 any overhead doesn't push over the threshold after checking here
*/
var KEEP_ALIVE_THRESHOLD = SIXTY_FOUR_KILOBYTES * 0.8;
var nativeAsyncGzipDisabled = false;
var removeURLParam = function (url, param) {
    var _a = __read(url.split('#'), 2), urlWithoutHash = _a[0], hash = _a[1];
    var _b = __read(urlWithoutHash.split('?'), 2), baseUrl = _b[0], search = _b[1];
    if (!search) {
        return url;
    }
    var updatedSearch = search
        .split('&')
        .filter(function (pair) { return pair.split('=')[0] !== param; })
        .join('&');
    return "".concat(baseUrl).concat(updatedSearch ? "?".concat(updatedSearch) : '').concat(hash ? "#".concat(hash) : '');
};
/**
 * Extends a URL with additional query parameters
 * @param url - The URL to extend
 * @param params - The parameters to add
 * @param replace - When true (default), new params overwrite existing ones with same key. When false, existing params are preserved.
 * @returns The URL with extended parameters
 */
var extendURLParams = function (url, params, replace) {
    var _a;
    if (replace === void 0) { replace = true; }
    var _b = __read(url.split('?'), 2), baseUrl = _b[0], search = _b[1];
    var newParams = __assign({}, params);
    var updatedSearch = (_a = search === null || search === void 0 ? void 0 : search.split('&').map(function (pair) {
        var _a;
        var _b = __read(pair.split('='), 2), key = _b[0], origValue = _b[1];
        var value = replace ? ((_a = newParams[key]) !== null && _a !== void 0 ? _a : origValue) : origValue;
        delete newParams[key];
        return "".concat(key, "=").concat(value);
    })) !== null && _a !== void 0 ? _a : [];
    var remaining = (0, request_utils_1.formDataToQuery)(newParams);
    if (remaining) {
        updatedSearch.push(remaining);
    }
    return updatedSearch.length > 0 ? "".concat(baseUrl, "?").concat(updatedSearch.join('&')) : baseUrl;
};
exports.extendURLParams = extendURLParams;
var encodeToDataString = function (data) {
    return 'data=' + encodeURIComponent(typeof data === 'string' ? data : (0, request_utils_1.jsonStringify)(data));
};
var encodePostData = function (options) {
    // Use pre-encoded body if available (set by async compression in the request entrypoint)
    if (options._encodedBody) {
        return options._encodedBody;
    }
    var data = options.data, compression = options.compression;
    if (!data) {
        return;
    }
    if (compression === types_1.Compression.GZipJS) {
        var gzipData = (0, fflate_1.gzipSync)((0, fflate_1.strToU8)((0, request_utils_1.jsonStringify)(data)), { mtime: 0 });
        return {
            contentType: CONTENT_TYPE_PLAIN,
            body: gzipData.buffer.slice(gzipData.byteOffset, gzipData.byteOffset + gzipData.byteLength),
            estimatedSize: gzipData.byteLength,
        };
    }
    if (compression === types_1.Compression.Base64) {
        var b64data = (0, encode_utils_1._base64Encode)((0, request_utils_1.jsonStringify)(data));
        var encodedBody = encodeToDataString(b64data);
        return {
            contentType: CONTENT_TYPE_FORM,
            body: encodedBody,
            estimatedSize: new Blob([encodedBody]).size,
        };
    }
    var jsonBody = (0, request_utils_1.jsonStringify)(data);
    return {
        contentType: CONTENT_TYPE_JSON,
        body: jsonBody,
        estimatedSize: new Blob([jsonBody]).size,
    };
};
var encodePostDataSafely = function (options) {
    var fallbackToUncompressed = function () {
        // beacon bodies must keep a CORS-simple content type even on the gzip-failure
        // fallback — uncompressed application/json preflights, base64 form data does not
        if (options.transport === 'sendBeacon') {
            return {
                url: (0, exports.extendURLParams)(options.url, { compression: types_1.Compression.Base64 }),
                encodedBody: encodePostData(__assign(__assign({}, options), { compression: types_1.Compression.Base64, _encodedBody: undefined })),
            };
        }
        return {
            url: removeURLParam(options.url, 'compression'),
            encodedBody: encodePostData(__assign(__assign({}, options), { compression: undefined, _encodedBody: undefined })),
        };
    };
    var encodedBody;
    try {
        encodedBody = encodePostData(options);
    }
    catch (error) {
        if ((0, core_1.isGzipRequest)(options.compression, (0, request_utils_1.getQueryParam)(options.url, 'compression'))) {
            logger_1.logger.error('Failed to gzip request body, sending uncompressed payload', error);
            return fallbackToUncompressed();
        }
        throw error;
    }
    if (!encodedBody ||
        !(0, core_1.isGzipRequest)(options.compression, (0, request_utils_1.getQueryParam)(options.url, 'compression')) ||
        (0, core_1.isGzipData)(encodedBody.body)) {
        return { url: options.url, encodedBody: encodedBody };
    }
    nativeAsyncGzipDisabled = true;
    return fallbackToUncompressed();
};
var encodeRequest = function (options) {
    var _a;
    try {
        return encodePostDataSafely(options);
    }
    catch (error) {
        logger_1.logger.error(error);
        (_a = options.callback) === null || _a === void 0 ? void 0 : _a.call(options, { statusCode: 0, error: error });
        return undefined;
    }
};
/**
 * Pre-encode the request body using async native CompressionStream.
 * This avoids blocking the main thread with fflate's synchronous gzip,
 * which can take 300ms+ on constrained devices.
 *
 * Callers must check preconditions (data exists, gzip compression, CompressionStream available)
 * before calling this function.
 */
var preEncodeAsync = function (options) { return __awaiter(void 0, void 0, void 0, function () {
    var jsonData, compressed, body;
    return __generator(this, function (_a) {
        switch (_a.label) {
            case 0:
                jsonData = (0, request_utils_1.jsonStringify)(options.data);
                return [4 /*yield*/, (0, core_1.gzipCompress)(jsonData, config_1.default.DEBUG, { rethrow: true })];
            case 1:
                compressed = _a.sent();
                if (!compressed) {
                    return [2 /*return*/, options];
                }
                return [4 /*yield*/, compressed.arrayBuffer()];
            case 2:
                body = _a.sent();
                return [2 /*return*/, __assign(__assign({}, options), { _encodedBody: {
                            contentType: CONTENT_TYPE_PLAIN,
                            body: body,
                            estimatedSize: body.byteLength,
                        } })];
        }
    });
}); };
/**
 * Builds the reason used when aborting a fetch because our own request timeout elapsed.
 * It keeps `name === 'AbortError'` (so callers that detect timeouts by error name keep working)
 * but carries a descriptive message, so it is never a reason-less
 * `signal is aborted without reason` exception.
 */
var timeoutAbortReason = function (timeout) {
    var reason = new Error("PostHog request timed out".concat(timeout ? " after ".concat(timeout, "ms") : ''));
    reason.name = 'AbortError';
    return reason;
};
// A failed fetch at the network layer (ad blocker, dropped connection, CORS, page teardown)
// rejects with a generic `TypeError` whose message varies by browser - Chrome
// `Failed to fetch`, Firefox `NetworkError when attempting to fetch resource.`, Safari
// `Load failed`. These are expected, retried failures rather than genuine errors, so we
// log them at `warn` rather than `error`.
var NETWORK_ERROR_MESSAGES = /Failed to fetch|NetworkError|Load failed/i;
var isExpectedNetworkError = function (error) {
    var err = error;
    return (err === null || err === void 0 ? void 0 : err.name) === 'TypeError' && NETWORK_ERROR_MESSAGES.test((err === null || err === void 0 ? void 0 : err.message) || '');
};
var xhr = function (options) {
    var encodedRequest = encodeRequest(options);
    if (!encodedRequest) {
        return;
    }
    var req = new globals_1.XMLHttpRequest();
    var url = encodedRequest.url, encodedBody = encodedRequest.encodedBody;
    req.open(options.method || 'GET', url, true);
    var _a = encodedBody !== null && encodedBody !== void 0 ? encodedBody : {}, contentType = _a.contentType, body = _a.body;
    (0, general_utils_1.each)(options.headers, function (headerValue, headerName) {
        req.setRequestHeader(headerName, headerValue);
    });
    if (contentType) {
        req.setRequestHeader('Content-Type', contentType);
    }
    if (options.timeout) {
        req.timeout = options.timeout;
    }
    req.onreadystatechange = function () {
        var _a;
        // XMLHttpRequest.DONE == 4, except in safari 4
        if (req.readyState === 4) {
            var response = {
                statusCode: req.status,
                text: req.responseText,
            };
            if (req.status === 200) {
                try {
                    response.json = JSON.parse(req.responseText);
                }
                catch (_b) {
                    // logger.error(e)
                }
            }
            (_a = options.callback) === null || _a === void 0 ? void 0 : _a.call(options, response);
        }
    };
    req.send(body);
};
var _fetch = function (options) {
    var encodedRequest = encodeRequest(options);
    if (!encodedRequest) {
        return;
    }
    var url = encodedRequest.url, encodedBody = encodedRequest.encodedBody;
    var _a = encodedBody !== null && encodedBody !== void 0 ? encodedBody : {}, contentType = _a.contentType, body = _a.body, estimatedSize = _a.estimatedSize;
    var headers = new Headers();
    (0, general_utils_1.each)(options.headers, function (headerValue, headerName) {
        headers.append(headerName, headerValue);
    });
    if (contentType) {
        headers.append('Content-Type', contentType);
    }
    var aborter = null;
    // Set the instant our own timeout fires, before the abort propagates. This is the source of
    // truth for "we timed out ourselves" - see the `.catch` below for why we can't rely on the
    // abort reason.
    var timedOut = false;
    if (globals_1.AbortController) {
        var controller_1 = new globals_1.AbortController();
        aborter = {
            signal: controller_1.signal,
            timeout: setTimeout(function () {
                timedOut = true;
                // Abort with an explicit reason. Without one, the browser rejects the fetch with a
                // reason-less `DOMException: AbortError: signal is aborted without reason`, which is
                // indistinguishable from a host app's own aborted fetches wherever it surfaces (the
                // `{ statusCode: 0, error }` callback, logs, stack traces). An explicit reason makes
                // our own request timeouts identifiable. We keep `name === 'AbortError'` so existing
                // timeout handling (e.g. feature flag timeout detection) keeps working.
                controller_1.abort(timeoutAbortReason(options.timeout));
            }, options.timeout),
        };
    }
    var handleError = function (error) {
        var _a;
        // Detect our own timeout via the `timedOut` flag rather than by comparing `error`
        // against the reason we passed to `controller.abort(...)`. Not every browser propagates
        // the abort reason to the fetch rejection - some reject with a generic native
        // `DOMException: AbortError: The operation was aborted.` instead - so a reference (or
        // message) comparison misses those and misclassifies our own timeout as a real error.
        // The flag is set synchronously the instant our timeout fires, and we additionally
        // require `name === 'AbortError'` so a genuine network error that happens to settle
        // just after the timeout is never mislabelled.
        if ((timedOut && (error === null || error === void 0 ? void 0 : error.name) === 'AbortError') || isExpectedNetworkError(error)) {
            // Expected, benign failures the request queue already retries - our own request
            // timeout (an intentional abort), or a network-level `TypeError` (ad blocker,
            // dropped connection, CORS, page teardown). Neither is a genuine failure, so log
            // at `warn` rather than `error`. (For setups running with debug logging and
            // `capture_console_errors` both enabled, this also keeps them out of error
            // tracking's console-error capture.)
            logger_1.logger.warn(error);
        }
        else {
            logger_1.logger.error(error);
        }
        (_a = options.callback) === null || _a === void 0 ? void 0 : _a.call(options, { statusCode: 0, error: error });
    };
    try {
        globals_1.fetch(url, __assign({ method: (options === null || options === void 0 ? void 0 : options.method) || 'GET', headers: headers, 
            // if body is greater than 64kb, then fetch with keepalive will error
            // see 8:10:5 at https://fetch.spec.whatwg.org/#http-network-or-cache-fetch,
            // but we do want to set keepalive sometimes as it can  help with success
            // when e.g. a page is being closed
            // so let's get the best of both worlds and only set keepalive for POST requests
            // where the body is less than 64kb
            // NB this is fetch keepalive and not http keepalive
            // _keepaliveDisabled: a beacon-rejected payload would fail a keepalive fetch too (shared quota)
            keepalive: options.method === 'POST' && !options._keepaliveDisabled && (estimatedSize || 0) < KEEP_ALIVE_THRESHOLD, body: body, signal: aborter === null || aborter === void 0 ? void 0 : aborter.signal }, options.fetchOptions))
            .then(function (response) {
            return response.text().then(function (responseText) {
                var _a;
                var res = {
                    statusCode: response.status,
                    text: responseText,
                };
                if (response.status === 200) {
                    try {
                        res.json = JSON.parse(responseText);
                    }
                    catch (e) {
                        logger_1.logger.error(e);
                    }
                }
                (_a = options.callback) === null || _a === void 0 ? void 0 : _a.call(options, res);
            });
        })
            .catch(handleError)
            .finally(function () { return (aborter ? clearTimeout(aborter.timeout) : null); });
    }
    catch (error) {
        // `window.fetch` can be monkey-patched by third-party scripts (e.g. a storefront/analytics
        // wrapper) to throw *synchronously* instead of returning a rejected promise. Because we may
        // call `_fetch` synchronously inside the host app's call stack (e.g. web experiments loaded
        // via `onFeatureFlags`), that throw would otherwise escape as an unhandled exception and
        // pollute error tracking. Route it through the same handling as an async rejection so the
        // request queue just retries. `.finally()` never runs when the call throws synchronously,
        // so clear the timeout here too.
        if (aborter) {
            clearTimeout(aborter.timeout);
        }
        handleError(error);
    }
    return;
};
// below this size a rejection means the shared quota is exhausted, not that the payload is too big
var BEACON_SPLIT_FLOOR_BYTES = 16 * 1024;
var addSentAtToBody = function (data, sentAt) {
    if (sentAt === void 0) { sentAt = new Date().toISOString(); }
    if (!(0, core_1.isArray)(data)) {
        return __assign(__assign({}, data), { sent_at: sentAt });
    }
    return data.map(function (item) { return (__assign(__assign({}, item), { sent_at: sentAt })); });
};
var _sendBeacon = function (options) {
    // beacon documentation https://w3c.github.io/beacon/
    // beacons format the message and use the type property
    var _a;
    try {
        var _b = encodePostDataSafely(options), url = _b.url, encodedBody = _b.encodedBody;
        var _c = encodedBody !== null && encodedBody !== void 0 ? encodedBody : {}, contentType = _c.contentType, body = _c.body, estimatedSize = _c.estimatedSize;
        if (!body) {
            return;
        }
        // sendBeacon requires a Blob to set the Content-Type header correctly.
        // Without wrapping, ArrayBuffer bodies are sent with no Content-Type,
        // which can cause issues with proxies/WAFs that require it.
        var sendBeaconBody = body instanceof Blob ? body : new Blob([body], { type: contentType });
        if (globals_1.navigator.sendBeacon(url, sendBeaconBody)) {
            return;
        }
        // rejected: over the page's shared ~64KiB in-flight keepalive quota
        // (https://fetch.spec.whatwg.org/#http-network-or-cache-fetch) — halve so what fits still delivers
        var batch = (0, core_1.isArray)(options.data) ? options.data : (_a = options.data) === null || _a === void 0 ? void 0 : _a.batch;
        if ((0, core_1.isArray)(batch) && batch.length > 1 && (estimatedSize !== null && estimatedSize !== void 0 ? estimatedSize : 0) > BEACON_SPLIT_FLOOR_BYTES) {
            var mid = Math.ceil(batch.length / 2);
            var splitData = function (events) {
                return (0, core_1.isArray)(options.data) ? events : __assign(__assign({}, options.data), { batch: events });
            };
            _sendBeacon(__assign(__assign({}, options), { data: splitData(batch.slice(0, mid)) }));
            _sendBeacon(__assign(__assign({}, options), { data: splitData(batch.slice(mid)) }));
            return;
        }
        logger_1.logger.warn("Beacon of ~".concat(estimatedSize !== null && estimatedSize !== void 0 ? estimatedSize : 0, " bytes was rejected by the browser, falling back to fetch"));
        _fetch(__assign(__assign({}, options), { _keepaliveDisabled: true }));
    }
    catch (error) {
        // send beacon is a best-effort, fire-and-forget mechanism on page unload,
        // we don't want to throw errors here
        logger_1.logger.warn('Beacon send failed', error);
    }
};
var buildRequestURL = function (url, method, compression, timestampMode) {
    var _a;
    var timestampParam = timestampMode === 'query' ? (method === 'POST' ? 'sent_at' : '_') : undefined;
    return (0, exports.extendURLParams)(compression === types_1.Compression.GZipJS ? removeURLParam(url, 'compression') : url, __assign(__assign({}, (timestampParam ? (_a = {}, _a[timestampParam] = Date.now().toString(), _a) : {})), (compression === types_1.Compression.GZipJS ? {} : { compression: compression })));
};
var addSentAtToCaptureBody = function (data) {
    var _a, _b;
    var batch = (0, core_1.isArray)(data) ? data : [data];
    var firstEvent = batch[0];
    return {
        api_key: (_b = (_a = firstEvent === null || firstEvent === void 0 ? void 0 : firstEvent.properties) === null || _a === void 0 ? void 0 : _a.token) !== null && _b !== void 0 ? _b : firstEvent === null || firstEvent === void 0 ? void 0 : firstEvent.token,
        batch: batch,
        sent_at: new Date().toISOString(),
    };
};
var AVAILABLE_TRANSPORTS = [];
// We add the transports in order of preference
if (globals_1.fetch) {
    AVAILABLE_TRANSPORTS.push({
        transport: 'fetch',
        method: _fetch,
    });
}
if (globals_1.XMLHttpRequest) {
    AVAILABLE_TRANSPORTS.push({
        transport: 'XHR',
        method: xhr,
    });
}
if (globals_1.navigator === null || globals_1.navigator === void 0 ? void 0 : globals_1.navigator.sendBeacon) {
    AVAILABLE_TRANSPORTS.push({
        transport: 'sendBeacon',
        method: _sendBeacon,
    });
}
// This is the entrypoint. It takes care of sanitizing the options and then calls the appropriate request method.
var request = function (_options) {
    var _a, _b, _c;
    // Clone the options so we don't modify the original object
    var options = __assign({}, _options);
    options.timeout = options.timeout || 60000;
    var transport = (_a = options.transport) !== null && _a !== void 0 ? _a : 'fetch';
    // beacons fire during page unload, where a CORS preflight cannot complete — the body
    // must keep a CORS-simple content type, which uncompressed (application/json) is not
    if (transport === 'sendBeacon' && (0, core_1.isUndefined)(options.compression) && options.data) {
        options.compression = types_1.Compression.Base64;
    }
    if (options.method === 'POST' && options.data) {
        if (options.timestampMode === 'capture-body') {
            options.data = addSentAtToCaptureBody(options.data);
        }
        else if (options.timestampMode === 'body') {
            options.data = addSentAtToBody(options.data);
        }
    }
    options.url = buildRequestURL(options.url, options.method, options.compression, options.timestampMode);
    var availableTransports = AVAILABLE_TRANSPORTS.filter(function (t) { return !options.disableTransport || !t.transport || !options.disableTransport.includes(t.transport); });
    var transportMethod = (_c = (_b = (0, general_utils_1.find)(availableTransports, function (t) { return t.transport === transport; })) === null || _b === void 0 ? void 0 : _b.method) !== null && _c !== void 0 ? _c : availableTransports[0].method;
    if (!transportMethod) {
        throw new Error('No available transport method');
    }
    // For non-sendBeacon transports, use async native CompressionStream when available
    // to avoid blocking the main thread with fflate's synchronous gzip (which can take 300ms+).
    // sendBeacon must remain synchronous as it's used during page unload.
    if (transport !== 'sendBeacon' &&
        options.data &&
        options.compression === types_1.Compression.GZipJS &&
        !!globals_1.CompressionStream &&
        typeof Promise !== 'undefined' &&
        !nativeAsyncGzipDisabled) {
        preEncodeAsync(options)
            .then(function (encodedOptions) {
            transportMethod(encodedOptions);
        })
            .catch(function (error) {
            if ((0, core_1.isNativeAsyncGzipReadError)(error)) {
                nativeAsyncGzipDisabled = true;
                transportMethod(__assign(__assign({}, options), { compression: undefined, url: buildRequestURL(_options.url, _options.method, undefined, _options.timestampMode) }));
                return;
            }
            if ((0, core_1.isNativeAsyncGzipError)(error)) {
                nativeAsyncGzipDisabled = true;
            }
            // If async compression fails for another reason, fall back to the synchronous fflate path
            transportMethod(options);
        });
    }
    else {
        transportMethod(options);
    }
};
exports.request = request;
//# sourceMappingURL=request.js.map