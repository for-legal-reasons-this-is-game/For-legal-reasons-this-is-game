"use strict";
// Portions of this file are derived from getsentry/sentry-javascript
// Copyright (c) 2012 Functional Software, Inc. dba Sentry
// Licensed under the MIT License: https://github.com/getsentry/sentry-javascript/blob/develop/LICENSE
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
exports.getRecordNetworkPlugin = exports.NETWORK_PLUGIN_NAME = exports.NEVER_RECORD_BODY_CONTENT_TYPES = void 0;
exports.findLast = findLast;
exports.shouldRecordBody = shouldRecordBody;
exports._contentLengthExceedsLimit = _contentLengthExceedsLimit;
exports._readBody = _readBody;
exports._tryReadBodyStreaming = _tryReadBodyStreaming;
var core_1 = require("@posthog/core");
var type_utils_1 = require("@posthog/browser-common/utils/type-utils");
var logger_1 = require("@posthog/browser-common/utils/logger");
var request_utils_1 = require("@posthog/browser-common/utils/request-utils");
var patch_1 = require("../rrweb-plugins/patch");
var denylist_1 = require("../../../extensions/replay/external/denylist");
var config_1 = require("./config");
var logger = (0, logger_1.createLogger)('[Recorder]');
var isNavigationTiming = function (entry) {
    return entry.entryType === 'navigation';
};
var isResourceTiming = function (entry) { return entry.entryType === 'resource'; };
function findLast(array, predicate) {
    var length = array.length;
    for (var i = length - 1; i >= 0; i -= 1) {
        if (predicate(array[i])) {
            return array[i];
        }
    }
    return undefined;
}
function initPerformanceObserver(cb, win, options) {
    // if we are only observing timings then we could have a single observer for all types, with buffer true,
    // but we are going to filter by initiatorType _if we are wrapping fetch and xhr as the wrapped functions
    // will deal with those.
    // so we have a block which captures requests from before fetch/xhr is wrapped
    // these are marked `isInitial` so playback can display them differently if needed
    // they will never have method/status/headers/body because they are pre-wrapping that provides that
    if (options.recordInitialRequests) {
        var initialPerformanceEntries = win.performance
            .getEntries()
            .filter(function (entry) {
            return isNavigationTiming(entry) ||
                (isResourceTiming(entry) && options.initiatorTypes.includes(entry.initiatorType));
        });
        cb({
            requests: initialPerformanceEntries.flatMap(function (entry) {
                return prepareRequest({ entry: entry, method: undefined, status: undefined, networkRequest: {}, isInitial: true });
            }),
            isInitial: true,
        });
    }
    var observer = new win.PerformanceObserver(function (entries) {
        // if recordBody or recordHeaders is true then we don't want to record fetch or xhr here
        // as the wrapped functions will do that. Otherwise, this filter becomes a noop
        // because we do want to record them here
        var wrappedInitiatorFilter = function (entry) {
            return options.recordBody || options.recordHeaders
                ? entry.initiatorType !== 'xmlhttprequest' && entry.initiatorType !== 'fetch'
                : true;
        };
        var performanceEntries = entries.getEntries().filter(function (entry) {
            return isNavigationTiming(entry) ||
                (isResourceTiming(entry) &&
                    options.initiatorTypes.includes(entry.initiatorType) &&
                    // TODO if we are _only_ capturing timing we don't want to filter initiator here
                    wrappedInitiatorFilter(entry));
        });
        cb({
            requests: performanceEntries.flatMap(function (entry) {
                return prepareRequest({ entry: entry, method: undefined, status: undefined, networkRequest: {} });
            }),
        });
    });
    // compat checked earlier
    var entryTypes = PerformanceObserver.supportedEntryTypes.filter(function (x) {
        return options.performanceEntryTypeToObserve.includes(x);
    });
    // initial records are gathered above, so we don't need to observe and buffer each type separately
    observer.observe({ entryTypes: entryTypes });
    return function () {
        observer.disconnect();
    };
}
function shouldRecordHeaders(type, recordHeaders) {
    return !!recordHeaders && ((0, core_1.isBoolean)(recordHeaders) || recordHeaders[type]);
}
function isRequest(value) {
    if (typeof Request === 'undefined') {
        return false;
    }
    if (value instanceof Request) {
        return true;
    }
    try {
        return Object.prototype.toString.call(value) === '[object Request]';
    }
    catch (_a) {
        return false;
    }
}
// binary/asset bodies are large and useless for replay debugging (and capturing an image body
// duplicates what the recording already shows), so we never record them even when recordBody is on
exports.NEVER_RECORD_BODY_CONTENT_TYPES = [
    'image/',
    'video/',
    'audio/',
    'font/',
    'application/octet-stream',
    'application/pdf',
    'application/zip',
    'application/wasm',
];
function shouldRecordBody(_a) {
    var type = _a.type, recordBody = _a.recordBody, headers = _a.headers, url = _a.url;
    function matchesContentType(contentTypes) {
        var contentTypeHeader = Object.keys(headers).find(function (key) { return key.toLowerCase() === 'content-type'; });
        var contentType = contentTypeHeader && headers[contentTypeHeader];
        return contentTypes.some(function (ct) { return contentType === null || contentType === void 0 ? void 0 : contentType.toLowerCase().includes(ct); });
    }
    /**
     * particularly in canvas applications we see many requests to blob URLs
     * e.g. blob:https://video_url
     * these blob/object URLs are local to the browser, we can never capture that body
     * so we can just return false here
     */
    function isBlobURL(url) {
        try {
            if (typeof url === 'string') {
                return url.startsWith('blob:');
            }
            if (url instanceof URL) {
                return url.protocol === 'blob:';
            }
            if (isRequest(url)) {
                return isBlobURL(url.url);
            }
            return false;
        }
        catch (_a) {
            return false;
        }
    }
    if (!recordBody)
        return false;
    if (isBlobURL(url))
        return false;
    // never record binary/asset bodies, regardless of the recordBody setting
    if (matchesContentType(exports.NEVER_RECORD_BODY_CONTENT_TYPES))
        return false;
    if ((0, core_1.isBoolean)(recordBody))
        return true;
    if ((0, core_1.isArray)(recordBody))
        return matchesContentType(recordBody);
    var recordBodyType = recordBody[type];
    if ((0, core_1.isBoolean)(recordBodyType))
        return recordBodyType;
    return matchesContentType(recordBodyType);
}
function getRequestPerformanceEntry(win_1, initiatorType_1, url_1, start_1, end_1) {
    return __awaiter(this, arguments, void 0, function (win, initiatorType, url, start, end, attempt) {
        var urlPerformanceEntries, performanceEntry;
        if (attempt === void 0) { attempt = 0; }
        return __generator(this, function (_a) {
            switch (_a.label) {
                case 0:
                    if (attempt > 10) {
                        logger.warn('Failed to get performance entry for request', { url: url, initiatorType: initiatorType });
                        return [2 /*return*/, null];
                    }
                    urlPerformanceEntries = win.performance.getEntriesByName(url);
                    performanceEntry = findLast(urlPerformanceEntries, function (entry) {
                        return isResourceTiming(entry) &&
                            entry.initiatorType === initiatorType &&
                            ((0, core_1.isUndefined)(start) || entry.startTime >= start) &&
                            ((0, core_1.isUndefined)(end) || entry.startTime <= end);
                    });
                    if (!!performanceEntry) return [3 /*break*/, 2];
                    return [4 /*yield*/, new Promise(function (resolve) { return setTimeout(resolve, 50 * attempt); })];
                case 1:
                    _a.sent();
                    return [2 /*return*/, getRequestPerformanceEntry(win, initiatorType, url, start, end, attempt + 1)];
                case 2: return [2 /*return*/, performanceEntry];
            }
        });
    });
}
/**
 * According to MDN https://developer.mozilla.org/en-US/docs/Web/API/XMLHttpRequest/response
 * xhr response is typed as any but can be an ArrayBuffer, a Blob, a Document, a JavaScript object,
 * or a string, depending on the value of XMLHttpRequest.responseType, that contains the response entity body.
 *
 * XHR request body is Document | XMLHttpRequestBodyInit | null | undefined
 */
function _tryReadXHRBody(_a) {
    var body = _a.body, options = _a.options, url = _a.url;
    if ((0, core_1.isNullish)(body)) {
        return null;
    }
    var _b = (0, denylist_1.isHostOnDenyList)(url, options), hostname = _b.hostname, isHostDenied = _b.isHostDenied;
    if (isHostDenied) {
        return hostname + ' is in deny list';
    }
    if ((0, core_1.isString)(body)) {
        return body;
    }
    if ((0, type_utils_1.isDocument)(body)) {
        return body.textContent;
    }
    if ((0, core_1.isFormData)(body)) {
        return (0, request_utils_1.formDataToQuery)(body);
    }
    if ((0, core_1.isObject)(body)) {
        try {
            return JSON.stringify(body);
        }
        catch (_c) {
            return '[SessionReplay] Failed to stringify response object';
        }
    }
    return '[SessionReplay] Cannot read body of type ' + toString.call(body);
}
function initXhrObserver(cb, win, options) {
    if (!options.initiatorTypes.includes('xmlhttprequest')) {
        return function () {
            //
        };
    }
    var recordRequestHeaders = shouldRecordHeaders('request', options.recordHeaders);
    var recordResponseHeaders = shouldRecordHeaders('response', options.recordHeaders);
    var restorePatch = (0, patch_1.patch)(win.XMLHttpRequest.prototype, 'open', 
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    function (originalOpen) {
        return function (method, url, async, username, password) {
            if (async === void 0) { async = true; }
            // because this function is returned in its actual context `this` _is_ an XMLHttpRequest
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            var xhr = this;
            // All of the capture instrumentation below runs _before_ we delegate to the original
            // `open`. If any of it throws (e.g. `new Request(url)` rejecting a URL/method the host
            // application would have handled), we must not let that exception escape into the host's
            // `xhr.open()` call and misattribute a failure to session replay. Wrap it so that if
            // instrumentation fails we degrade gracefully and the original request still proceeds.
            try {
                // check IE earlier than this, we only initialize if Request is present
                var req_1 = new Request(url);
                var networkRequest_1 = {};
                var start_1;
                var end_1;
                var requestHeaders_1 = {};
                var originalSetRequestHeader_1 = xhr.setRequestHeader.bind(xhr);
                xhr.setRequestHeader = function (header, value) {
                    requestHeaders_1[header] = value;
                    return originalSetRequestHeader_1(header, value);
                };
                if (recordRequestHeaders) {
                    networkRequest_1.requestHeaders = requestHeaders_1;
                }
                var originalSend_1 = xhr.send.bind(xhr);
                xhr.send = function (body) {
                    if (shouldRecordBody({
                        type: 'request',
                        headers: requestHeaders_1,
                        url: url,
                        recordBody: options.recordBody,
                    })) {
                        networkRequest_1.requestBody = _tryReadXHRBody({ body: body, options: options, url: url });
                    }
                    start_1 = win.performance.now();
                    return originalSend_1(body);
                };
                // Cleanup function to remove all event listeners and prevent memory leaks
                var cleanup_1 = function () {
                    xhr.removeEventListener('readystatechange', readyStateListener_1);
                    xhr.removeEventListener('error', cleanup_1);
                    xhr.removeEventListener('abort', cleanup_1);
                    xhr.removeEventListener('timeout', cleanup_1);
                };
                var readyStateListener_1 = function () {
                    if (xhr.readyState !== xhr.DONE) {
                        return;
                    }
                    // Clean up all listeners immediately when done to prevent memory leaks
                    cleanup_1();
                    end_1 = win.performance.now();
                    var responseHeaders = {};
                    var rawHeaders = xhr.getAllResponseHeaders();
                    var headers = rawHeaders.trim().split(/[\r\n]+/);
                    headers.forEach(function (line) {
                        var parts = line.split(': ');
                        var header = parts.shift();
                        var value = parts.join(': ');
                        if (header) {
                            responseHeaders[header] = value;
                        }
                    });
                    if (recordResponseHeaders) {
                        networkRequest_1.responseHeaders = responseHeaders;
                    }
                    if (shouldRecordBody({
                        type: 'response',
                        headers: responseHeaders,
                        url: url,
                        recordBody: options.recordBody,
                    })) {
                        networkRequest_1.responseBody = _tryReadXHRBody({ body: xhr.response, options: options, url: url });
                    }
                    getRequestPerformanceEntry(win, 'xmlhttprequest', req_1.url, start_1, end_1)
                        .then(function (entry) {
                        var requests = prepareRequest({
                            entry: entry,
                            method: method,
                            status: xhr === null || xhr === void 0 ? void 0 : xhr.status,
                            networkRequest: networkRequest_1,
                            start: start_1,
                            end: end_1,
                            url: url.toString(),
                            initiatorType: 'xmlhttprequest',
                        });
                        cb({ requests: requests });
                    })
                        .catch(function () {
                        //
                    });
                };
                // This is very tricky code, and making it passive won't bring many performance benefits,
                // so let's ignore the rule here.
                // eslint-disable-next-line posthog-js/no-add-event-listener
                xhr.addEventListener('readystatechange', readyStateListener_1);
                // Also clean up on error, abort, and timeout to prevent memory leaks
                // eslint-disable-next-line posthog-js/no-add-event-listener
                xhr.addEventListener('error', cleanup_1);
                // eslint-disable-next-line posthog-js/no-add-event-listener
                xhr.addEventListener('abort', cleanup_1);
                // eslint-disable-next-line posthog-js/no-add-event-listener
                xhr.addEventListener('timeout', cleanup_1);
            }
            catch (e) {
                logger.error('Failed to instrument XHR for network capture', e);
            }
            originalOpen.call(xhr, method, url.toString(), async, username, password);
        };
    });
    return function () {
        restorePatch();
    };
}
/**
 *  Check if this PerformanceEntry is either a PerformanceResourceTiming or a PerformanceNavigationTiming
 *  NB PerformanceNavigationTiming extends PerformanceResourceTiming
 *  Here we don't care which interface it implements as both expose `serverTimings`
 */
var exposesServerTiming = function (event) {
    return !(0, core_1.isNull)(event) && (event.entryType === 'navigation' || event.entryType === 'resource');
};
function prepareRequest(_a) {
    var e_1, _b;
    var entry = _a.entry, method = _a.method, status = _a.status, networkRequest = _a.networkRequest, isInitial = _a.isInitial, start = _a.start, end = _a.end, url = _a.url, initiatorType = _a.initiatorType;
    start = entry ? entry.startTime : start;
    end = entry ? entry.responseEnd : end;
    // kudos to sentry javascript sdk for excellent background on why to use Date.now() here
    // https://github.com/getsentry/sentry-javascript/blob/e856e40b6e71a73252e788cd42b5260f81c9c88e/packages/utils/src/time.ts#L70
    // can't start observer if performance.now() is not available
    // eslint-disable-next-line compat/compat
    var timeOrigin = Math.floor(Date.now() - performance.now());
    // clickhouse can't ingest timestamps that are floats
    // (in this case representing fractions of a millisecond we don't care about anyway)
    // use timeOrigin if we really can't gather a start time
    var timestamp = Math.floor(timeOrigin + (start || 0));
    var entryJSON = entry ? entry.toJSON() : { name: url };
    var requests = [
        __assign(__assign({}, entryJSON), { startTime: (0, core_1.isUndefined)(start) ? undefined : Math.round(start), endTime: (0, core_1.isUndefined)(end) ? undefined : Math.round(end), timeOrigin: timeOrigin, timestamp: timestamp, method: method, initiatorType: initiatorType ? initiatorType : entry ? entry.initiatorType : undefined, status: status, requestHeaders: networkRequest.requestHeaders, requestBody: networkRequest.requestBody, responseHeaders: networkRequest.responseHeaders, responseBody: networkRequest.responseBody, isInitial: isInitial }),
    ];
    if (exposesServerTiming(entry)) {
        try {
            for (var _c = __values(entry.serverTiming || []), _d = _c.next(); !_d.done; _d = _c.next()) {
                var timing = _d.value;
                requests.push({
                    timeOrigin: timeOrigin,
                    timestamp: timestamp,
                    startTime: Math.round(entry.startTime),
                    name: timing.name,
                    duration: timing.duration,
                    // the spec has a closed list of possible types
                    // https://developer.mozilla.org/en-US/docs/Web/API/PerformanceEntry/entryType
                    // but, we need to know this was a server timing so that we know to
                    // match it to the appropriate navigation or resource timing
                    // that matching will have to be on timestamp and $current_url
                    entryType: 'serverTiming',
                });
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_d && !_d.done && (_b = _c.return)) _b.call(_c);
            }
            finally { if (e_1) throw e_1.error; }
        }
    }
    return requests;
}
var contentTypePrefixDenyList = ['video/', 'audio/'];
function _checkForCannotReadResponseBody(_a) {
    var _b;
    var r = _a.r, options = _a.options, url = _a.url;
    if (r.headers.get('Transfer-Encoding') === 'chunked') {
        return 'Chunked Transfer-Encoding is not supported';
    }
    // `get` and `has` are case-insensitive
    // but return the header value with the casing that was supplied
    var contentType = (_b = r.headers.get('Content-Type')) === null || _b === void 0 ? void 0 : _b.toLowerCase();
    var contentTypeIsDenied = contentTypePrefixDenyList.some(function (prefix) { return contentType === null || contentType === void 0 ? void 0 : contentType.startsWith(prefix); });
    if (contentType && contentTypeIsDenied) {
        return "Content-Type ".concat(contentType, " is not supported");
    }
    var _c = (0, denylist_1.isHostOnDenyList)(url, options), hostname = _c.hostname, isHostDenied = _c.isHostDenied;
    if (isHostDenied) {
        return hostname + ' is in deny list';
    }
    return null;
}
function isReadableStreamBody(body) {
    return (0, core_1.isObject)(body) && (0, core_1.isFunction)(body.getReader) && (0, core_1.isFunction)(body.tee);
}
// reading a body must never hold up the page's request for long, so we cap every read attempt
var BODY_READ_TIMEOUT_MS = 500;
var BODY_READ_TIMEOUT_MESSAGE = '[SessionReplay] Timeout while trying to read body';
var BODY_READ_FAILED_MESSAGE = '[SessionReplay] Failed to read body';
function bodyReadFailedMessage(reason) {
    return "".concat(BODY_READ_FAILED_MESSAGE, ": ").concat(reason);
}
function bodyTooLargeMessage(limitBytes) {
    return "[SessionReplay] Body too large to record (> ".concat(limitBytes, " bytes)");
}
// when the body declares a content-length over the limit we can skip reading it entirely.
// this trusts content-length the same way config.ts enforcePayloadSizeLimit does.
// known accepted risk: for a compressed response content-length is the compressed size while the
// streaming reader counts decoded bytes, so a body that is over the limit compressed but under it
// decoded is dropped to the placeholder rather than recorded. it never breaks the page, so we accept it.
function _contentLengthExceedsLimit(r, limitBytes) {
    var _a, _b;
    try {
        var headerValue = (_b = (_a = r.headers) === null || _a === void 0 ? void 0 : _a.get) === null || _b === void 0 ? void 0 : _b.call(_a, 'content-length');
        if (!headerValue) {
            return false;
        }
        var contentLength = parseInt(headerValue, 10);
        return Number.isFinite(contentLength) && contentLength > limitBytes;
    }
    catch (_c) {
        return false;
    }
}
function _readBody(r, options) {
    if (!options.streamNetworkBody) {
        return _tryReadBody(r);
    }
    var limitBytes = (0, config_1.effectivePayloadLimitBytes)(options);
    // skip the read for bodies the headers already tell us are over the limit
    if (_contentLengthExceedsLimit(r, limitBytes)) {
        return Promise.resolve(bodyTooLargeMessage(limitBytes));
    }
    return _tryReadBodyStreaming(r, limitBytes);
}
function _tryReadBody(r) {
    // there are now already multiple places where we're using Promise...
    // eslint-disable-next-line compat/compat
    return new Promise(function (resolve) {
        var timeout = setTimeout(function () { return resolve(BODY_READ_TIMEOUT_MESSAGE); }, BODY_READ_TIMEOUT_MS);
        try {
            r.clone()
                .text()
                .then(function (txt) { return resolve(txt); }, function (reason) { return resolve(bodyReadFailedMessage(reason)); })
                .finally(function () { return clearTimeout(timeout); });
        }
        catch (_a) {
            clearTimeout(timeout);
            resolve(BODY_READ_FAILED_MESSAGE);
        }
    });
}
function concatChunks(chunks, totalBytes) {
    var e_2, _a;
    var merged = new Uint8Array(totalBytes);
    var offset = 0;
    try {
        for (var chunks_1 = __values(chunks), chunks_1_1 = chunks_1.next(); !chunks_1_1.done; chunks_1_1 = chunks_1.next()) {
            var chunk = chunks_1_1.value;
            merged.set(chunk, offset);
            offset += chunk.byteLength;
        }
    }
    catch (e_2_1) { e_2 = { error: e_2_1 }; }
    finally {
        try {
            if (chunks_1_1 && !chunks_1_1.done && (_a = chunks_1.return)) _a.call(chunks_1);
        }
        finally { if (e_2) throw e_2.error; }
    }
    return merged;
}
// Reads a clone of the body chunk by chunk, stopping as soon as the running total exceeds the
// limit, so a very large body is never fully buffered. Like _tryReadBody it only ever reads a
// clone (never the stream the page consumes) and is guaranteed to resolve, never reject.
function _tryReadBodyStreaming(r, limitBytes) {
    // eslint-disable-next-line compat/compat
    return new Promise(function (resolve) {
        var settled = false;
        var reader;
        function cancel() {
            try {
                void (reader === null || reader === void 0 ? void 0 : reader.cancel());
            }
            catch (_a) {
                // the reader may already be released; nothing to clean up
            }
        }
        // resolving always releases the reader, so a slow/hung stream stops being pulled (and stops
        // buffering the clone) the moment we settle — whether via success, the cap, an error, or the timeout
        function done(value) {
            if (settled) {
                return;
            }
            settled = true;
            clearTimeout(timeout);
            cancel();
            resolve(value);
        }
        var timeout = setTimeout(function () { return done(BODY_READ_TIMEOUT_MESSAGE); }, BODY_READ_TIMEOUT_MS);
        var clone;
        try {
            clone = r.clone();
        }
        catch (_a) {
            done(BODY_READ_FAILED_MESSAGE);
            return;
        }
        var body = clone.body;
        // no readable stream (or no TextDecoder) available — fall back to the buffered read of the clone
        if (!isReadableStreamBody(body) || typeof TextDecoder === 'undefined') {
            try {
                clone.text().then(function (txt) { return done(txt); }, function (reason) { return done(bodyReadFailedMessage(reason)); });
            }
            catch (_b) {
                done(BODY_READ_FAILED_MESSAGE);
            }
            return;
        }
        try {
            reader = body.getReader();
        }
        catch (_c) {
            done(BODY_READ_FAILED_MESSAGE);
            return;
        }
        var chunks = [];
        var received = 0;
        function pump() {
            reader.read().then(function (_a) {
                var streamDone = _a.done, value = _a.value;
                // already resolved (e.g. the timeout fired) — stop reading and release the stream
                if (settled) {
                    cancel();
                    return;
                }
                if (streamDone) {
                    done(new TextDecoder().decode(concatChunks(chunks, received)));
                    return;
                }
                if (value) {
                    if (received + value.byteLength > limitBytes) {
                        done(bodyTooLargeMessage(limitBytes));
                        return;
                    }
                    received += value.byteLength;
                    chunks.push(value);
                }
                pump();
            }, function (reason) { return done(bodyReadFailedMessage(reason)); });
        }
        pump();
    });
}
function _tryReadRequestBody(_a) {
    return __awaiter(this, arguments, void 0, function (_b) {
        var _c, hostname, isHostDenied;
        var r = _b.r, options = _b.options, url = _b.url;
        return __generator(this, function (_d) {
            _c = (0, denylist_1.isHostOnDenyList)(url, options), hostname = _c.hostname, isHostDenied = _c.isHostDenied;
            if (isHostDenied) {
                return [2 /*return*/, Promise.resolve(hostname + ' is in deny list')];
            }
            return [2 /*return*/, _readBody(r, options)];
        });
    });
}
function _tryReadResponseBody(_a) {
    return __awaiter(this, arguments, void 0, function (_b) {
        var cannotReadBodyReason;
        var r = _b.r, options = _b.options, url = _b.url;
        return __generator(this, function (_c) {
            cannotReadBodyReason = _checkForCannotReadResponseBody({ r: r, options: options, url: url });
            if (!(0, core_1.isNull)(cannotReadBodyReason)) {
                return [2 /*return*/, Promise.resolve(cannotReadBodyReason)];
            }
            return [2 /*return*/, _readBody(r, options)];
        });
    });
}
function initFetchObserver(cb, win, options) {
    if (!options.initiatorTypes.includes('fetch')) {
        return function () {
            //
        };
    }
    var recordRequestHeaders = shouldRecordHeaders('request', options.recordHeaders);
    var recordResponseHeaders = shouldRecordHeaders('response', options.recordHeaders);
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    var restorePatch = (0, patch_1.patch)(win, 'fetch', function (originalFetch) {
        return function (url, init) {
            return __awaiter(this, void 0, void 0, function () {
                var req, res, networkRequest, start, end, requestHeaders_2, requestBodyIsReadableStream, _a, e_3, _b, responseHeaders_1, _c;
                return __generator(this, function (_d) {
                    switch (_d.label) {
                        case 0:
                            try {
                                // check IE earlier than this, we only initialize if Request is present
                                req = new Request(url, init);
                            }
                            catch (e) {
                                logger.error('Failed to instrument fetch for network capture', e);
                                return [2 /*return*/, originalFetch(url, init)];
                            }
                            networkRequest = {};
                            _d.label = 1;
                        case 1:
                            _d.trys.push([1, , 13, 14]);
                            _d.label = 2;
                        case 2:
                            _d.trys.push([2, 5, , 6]);
                            requestHeaders_2 = {};
                            req.headers.forEach(function (value, header) {
                                requestHeaders_2[header] = value;
                            });
                            if (recordRequestHeaders) {
                                networkRequest.requestHeaders = requestHeaders_2;
                            }
                            requestBodyIsReadableStream = isReadableStreamBody(init === null || init === void 0 ? void 0 : init.body);
                            if (!(!requestBodyIsReadableStream &&
                                shouldRecordBody({
                                    type: 'request',
                                    headers: requestHeaders_2,
                                    url: url,
                                    recordBody: options.recordBody,
                                }))) return [3 /*break*/, 4];
                            _a = networkRequest;
                            return [4 /*yield*/, _tryReadRequestBody({ r: req, options: options, url: url })];
                        case 3:
                            _a.requestBody = _d.sent();
                            _d.label = 4;
                        case 4: return [3 /*break*/, 6];
                        case 5:
                            e_3 = _d.sent();
                            logger.error('Failed to record fetch request for network capture', e_3);
                            return [3 /*break*/, 6];
                        case 6:
                            start = win.performance.now();
                            if (!isRequest(url)) return [3 /*break*/, 8];
                            return [4 /*yield*/, originalFetch(req)];
                        case 7:
                            _b = _d.sent();
                            return [3 /*break*/, 10];
                        case 8: return [4 /*yield*/, originalFetch(url, init)];
                        case 9:
                            _b = _d.sent();
                            _d.label = 10;
                        case 10:
                            // Use `req` for recording metadata/body only. For fetch(url, init), do not pass this internally-created
                            // Request downstream: it exposes request.body as a ReadableStream, and wrappers that forward that body
                            // can trigger Safari's "ReadableStream uploading is not supported" error. For fetch(Request), we must
                            // pass the cloned Request because constructing `req` may consume the original Request body.
                            res = _b;
                            end = win.performance.now();
                            responseHeaders_1 = {};
                            res.headers.forEach(function (value, header) {
                                responseHeaders_1[header] = value;
                            });
                            if (recordResponseHeaders) {
                                networkRequest.responseHeaders = responseHeaders_1;
                            }
                            if (!shouldRecordBody({
                                type: 'response',
                                headers: responseHeaders_1,
                                url: url,
                                recordBody: options.recordBody,
                            })) return [3 /*break*/, 12];
                            _c = networkRequest;
                            return [4 /*yield*/, _tryReadResponseBody({ r: res, options: options, url: url })];
                        case 11:
                            _c.responseBody = _d.sent();
                            _d.label = 12;
                        case 12: return [2 /*return*/, res];
                        case 13:
                            getRequestPerformanceEntry(win, 'fetch', req.url, start, end)
                                .then(function (entry) {
                                var requests = prepareRequest({
                                    entry: entry,
                                    method: req.method,
                                    status: res === null || res === void 0 ? void 0 : res.status,
                                    networkRequest: networkRequest,
                                    start: start,
                                    end: end,
                                    url: req.url,
                                    initiatorType: 'fetch',
                                });
                                cb({ requests: requests });
                            })
                                .catch(function () {
                                //
                            });
                            return [7 /*endfinally*/];
                        case 14: return [2 /*return*/];
                    }
                });
            });
        };
    });
    return function () {
        restorePatch();
    };
}
var initialisedHandler = null;
function initNetworkObserver(callback, win, // top window or in an iframe
options) {
    if (!('performance' in win)) {
        return function () {
            //
        };
    }
    if (initialisedHandler) {
        logger.warn('Network observer already initialised, doing nothing');
        return function () {
            // the first caller should already have this handler and will be responsible for teardown
        };
    }
    var networkOptions = (options ? Object.assign({}, config_1.defaultNetworkOptions, options) : config_1.defaultNetworkOptions);
    var cb = function (data) {
        var requests = [];
        var parentRequestDropped = false;
        data.requests.forEach(function (request) {
            var isServerTiming = request.entryType === 'serverTiming';
            // Server timings are emitted immediately after their resource or navigation entry.
            // If a parent is dropped (i.e. a PostHog ingestion request), derived timings must too or an endless capture loop ensues.
            if (isServerTiming && parentRequestDropped) {
                return;
            }
            var maskedRequest = networkOptions.maskRequestFn(request);
            if (!isServerTiming) {
                parentRequestDropped = !maskedRequest;
            }
            if (maskedRequest) {
                requests.push(maskedRequest);
            }
        });
        if (requests.length > 0) {
            callback(__assign(__assign({}, data), { requests: requests }));
        }
    };
    var performanceObserver = initPerformanceObserver(cb, win, networkOptions);
    // only wrap fetch and xhr if headers or body are being recorded
    var xhrObserver = function () { };
    var fetchObserver = function () { };
    if (networkOptions.recordHeaders || networkOptions.recordBody) {
        xhrObserver = initXhrObserver(cb, win, networkOptions);
        fetchObserver = initFetchObserver(cb, win, networkOptions);
    }
    initialisedHandler = function () {
        performanceObserver();
        xhrObserver();
        fetchObserver();
        initialisedHandler = null;
    };
    return initialisedHandler;
}
// use the plugin name so that when this functionality is adopted into rrweb
// we can remove this plugin and use the core functionality with the same data
exports.NETWORK_PLUGIN_NAME = 'rrweb/network@1';
// TODO how should this be typed?
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore
var getRecordNetworkPlugin = function (options) {
    return {
        name: exports.NETWORK_PLUGIN_NAME,
        observer: initNetworkObserver,
        options: options,
    };
};
exports.getRecordNetworkPlugin = getRecordNetworkPlugin;
// rrweb/networ@1 ends
//# sourceMappingURL=network-plugin.js.map