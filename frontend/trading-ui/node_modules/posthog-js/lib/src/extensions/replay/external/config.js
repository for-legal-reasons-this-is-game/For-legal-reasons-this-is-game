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
exports.buildNetworkRequestOptions = exports.defaultNetworkOptions = exports.MAX_PAYLOAD_SIZE_BYTES = void 0;
exports.effectivePayloadLimitBytes = effectivePayloadLimitBytes;
var core_1 = require("@posthog/core");
var request_utils_1 = require("@posthog/browser-common/utils/request-utils");
var logger_1 = require("@posthog/browser-common/utils/logger");
var autocapture_utils_1 = require("@posthog/browser-common/utils/autocapture-utils");
var general_utils_1 = require("@posthog/browser-common/utils/general-utils");
var LOGGER_PREFIX = '[SessionRecording]';
var REDACTED = 'redacted';
// the hard ceiling for a recorded request/response body — 1MB, even if a larger limit is configured
exports.MAX_PAYLOAD_SIZE_BYTES = 1000000;
// the smallest of the 1MB ceiling or the configured limit if there is one
function effectivePayloadLimitBytes(options) {
    var _a;
    return Math.min(exports.MAX_PAYLOAD_SIZE_BYTES, (_a = options.payloadSizeLimitBytes) !== null && _a !== void 0 ? _a : exports.MAX_PAYLOAD_SIZE_BYTES);
}
exports.defaultNetworkOptions = {
    initiatorTypes: [
        'audio',
        'beacon',
        'body',
        'css',
        'early-hint',
        'embed',
        'fetch',
        'frame',
        'iframe',
        'icon',
        'image',
        'img',
        'input',
        'link',
        'navigation',
        'object',
        'ping',
        'script',
        'track',
        'video',
        'xmlhttprequest',
    ],
    maskRequestFn: function (data) { return data; },
    recordHeaders: false,
    recordBody: false,
    recordInitialRequests: false,
    recordPerformance: false,
    performanceEntryTypeToObserve: [
        // 'event', // This is too noisy as it covers all browser events
        'first-input',
        // 'mark', // Mark is used too liberally. We would need to filter for specific marks
        // 'measure', // Measure is used too liberally. We would need to filter for specific measures
        'navigation',
        'paint',
        'resource',
    ],
    payloadSizeLimitBytes: exports.MAX_PAYLOAD_SIZE_BYTES,
    payloadHostDenyList: [
        '.lr-ingest.io',
        '.ingest.sentry.io',
        '.clarity.ms',
        // NB no leading dot here
        // GA4/gtag beacons go to *.google-analytics.com; with Google Signals on they also hit
        // analytics.google.com (region1.analytics.google.com/g/collect), so deny both
        'google-analytics.com',
        'analytics.google.com',
        // New Relic browser agent (bam + bam-cell)
        'nr-data.net',
        // Datadog browser RUM intake
        'datadoghq.com',
        'datadoghq.eu',
        'ddog-gov.com',
        // other third-party analytics / session-replay vendors whose telemetry has no replay value
        'segment.io',
        'rudderstack.com',
        'amplitude.com',
        'mixpanel.com',
        // Hotjar uses both .com and .io for data collection
        'hotjar.com',
        'hotjar.io',
        'fullstory.com',
    ],
    streamNetworkBody: false,
};
var HEADER_DENY_LIST = [
    'authorization',
    'x-forwarded-for',
    'authorization',
    'cookie',
    'set-cookie',
    'x-api-key',
    'x-real-ip',
    'remote-addr',
    'forwarded',
    'proxy-authorization',
    'x-csrf-token',
    'x-csrftoken',
    'x-xsrf-token',
];
var PAYLOAD_CONTENT_DENY_LIST = [
    'password',
    'secret',
    'passwd',
    'api_key',
    'apikey',
    'auth',
    'credentials',
    'mysql_pwd',
    'privatekey',
    'private_key',
    'token',
];
// substrings that mark a header as credential-bearing - catches custom names
// (e.g. x-gist-encoded-user-token) that aren't in the exact deny list above
var HEADER_DENY_SUBSTRINGS = [
    'auth',
    'token',
    'secret',
    'session',
    'api-key',
    'apikey',
    'api_key',
    'credential',
    'password',
    'passwd',
    'cookie',
    'csrf',
    'xsrf',
];
var isDeniedHeader = function (header) {
    var lower = header.toLowerCase();
    return HEADER_DENY_LIST.includes(lower) || HEADER_DENY_SUBSTRINGS.some(function (s) { return lower.includes(s); });
};
var redactDeniedHeaders = function (headers) {
    if (!(0, core_1.isNullish)(headers)) {
        (0, general_utils_1.each)(Object.keys(headers), function (header) {
            if (isDeniedHeader(header)) {
                headers[header] = REDACTED;
            }
        });
    }
};
// we always remove headers on the deny list because we never want to capture this sensitive data.
// applies to both request and response headers (e.g. set-cookie is a response header)
var removeAuthorizationHeader = function (data) {
    redactDeniedHeaders(data.requestHeaders);
    redactDeniedHeaders(data.responseHeaders);
    return data;
};
var POSTHOG_PATHS_TO_IGNORE = ['/s/', '/e/', '/i/'];
// want to ignore posthog paths when capturing requests, or we can get trapped in a loop
// because calls to PostHog would be reported using a call to PostHog which would be reported....
var ignorePostHogPaths = function (data, apiHostConfig) {
    var _a;
    var url = (0, request_utils_1.convertToURL)(data.name);
    // we need to account for api host config as e.g. pathname could be /ingest/s/ and we want to ignore that
    var replaceValue = apiHostConfig.indexOf('http') === 0 ? (_a = (0, request_utils_1.convertToURL)(apiHostConfig)) === null || _a === void 0 ? void 0 : _a.pathname : apiHostConfig;
    if (replaceValue === '/') {
        replaceValue = '';
    }
    var pathname = url === null || url === void 0 ? void 0 : url.pathname.replace(replaceValue || '', '');
    if (url && pathname && POSTHOG_PATHS_TO_IGNORE.some(function (path) { return pathname.indexOf(path) === 0; })) {
        return undefined;
    }
    return data;
};
function estimateBytes(payload) {
    return new Blob([payload]).size;
}
function enforcePayloadSizeLimit(payload, headers, limit, description) {
    if ((0, core_1.isNullish)(payload)) {
        return payload;
    }
    var requestContentLength = (headers === null || headers === void 0 ? void 0 : headers['content-length']) || estimateBytes(payload);
    if ((0, core_1.isString)(requestContentLength)) {
        requestContentLength = parseInt(requestContentLength);
    }
    if (requestContentLength > limit) {
        return LOGGER_PREFIX + " ".concat(description, " body too large to record (").concat(requestContentLength, " bytes)");
    }
    return payload;
}
// people can have arbitrarily large payloads on their site, but we don't want to ingest them
var limitPayloadSize = function (options) {
    var limit = effectivePayloadLimitBytes(options);
    return function (data) {
        if (data === null || data === void 0 ? void 0 : data.requestBody) {
            data.requestBody = enforcePayloadSizeLimit(data.requestBody, data.requestHeaders, limit, 'Request');
        }
        if (data === null || data === void 0 ? void 0 : data.responseBody) {
            data.responseBody = enforcePayloadSizeLimit(data.responseBody, data.responseHeaders, limit, 'Response');
        }
        return data;
    };
};
function scrubPayload(payload, label) {
    if ((0, core_1.isNullish)(payload)) {
        return payload;
    }
    var scrubbed = payload;
    if (!(0, autocapture_utils_1.shouldCaptureValue)(scrubbed, false)) {
        scrubbed = LOGGER_PREFIX + ' ' + label + ' body ' + REDACTED;
    }
    (0, general_utils_1.each)(PAYLOAD_CONTENT_DENY_LIST, function (text) {
        if ((scrubbed === null || scrubbed === void 0 ? void 0 : scrubbed.length) && (scrubbed === null || scrubbed === void 0 ? void 0 : scrubbed.indexOf(text)) !== -1) {
            scrubbed = LOGGER_PREFIX + ' ' + label + ' body ' + REDACTED + ' as might contain: ' + text;
        }
    });
    return scrubbed;
}
function scrubPayloads(capturedRequest) {
    if ((0, core_1.isUndefined)(capturedRequest)) {
        return undefined;
    }
    capturedRequest.requestBody = scrubPayload(capturedRequest.requestBody, 'Request');
    capturedRequest.responseBody = scrubPayload(capturedRequest.responseBody, 'Response');
    return capturedRequest;
}
/**
 *  whether a maskRequestFn is provided or not,
 *  we ensure that we remove the denied header from requests
 *  we _never_ want to record that header by accident
 *  if someone complains then we'll add an opt-in to let them override it
 */
var buildNetworkRequestOptions = function (instanceConfig, remoteNetworkOptions) {
    var config = {
        payloadSizeLimitBytes: exports.defaultNetworkOptions.payloadSizeLimitBytes,
        performanceEntryTypeToObserve: __spreadArray([], __read(exports.defaultNetworkOptions.performanceEntryTypeToObserve), false),
        payloadHostDenyList: __spreadArray(__spreadArray([], __read((remoteNetworkOptions.payloadHostDenyList || [])), false), __read(exports.defaultNetworkOptions.payloadHostDenyList), false),
    };
    // client can always disable despite remote options
    var canRecordHeaders = instanceConfig.session_recording.recordHeaders === false ? false : remoteNetworkOptions.recordHeaders;
    var canRecordBody = instanceConfig.session_recording.recordBody === false ? false : remoteNetworkOptions.recordBody;
    var canRecordPerformance = instanceConfig.capture_performance === false ? false : remoteNetworkOptions.recordPerformance;
    var payloadLimiter = limitPayloadSize(config);
    var enforcedCleaningFn = function (d) {
        return payloadLimiter(ignorePostHogPaths(removeAuthorizationHeader(d), instanceConfig.api_host));
    };
    var hasDeprecatedMaskFunction = (0, core_1.isFunction)(instanceConfig.session_recording.maskNetworkRequestFn);
    if (hasDeprecatedMaskFunction && (0, core_1.isFunction)(instanceConfig.session_recording.maskCapturedNetworkRequestFn)) {
        logger_1.logger.warn('Both `maskNetworkRequestFn` and `maskCapturedNetworkRequestFn` are defined. `maskNetworkRequestFn` will be ignored.');
    }
    if (hasDeprecatedMaskFunction) {
        instanceConfig.session_recording.maskCapturedNetworkRequestFn = function (data) {
            var cleanedURL = instanceConfig.session_recording.maskNetworkRequestFn({ url: data.name });
            // the deprecated mask fn can suppress the URL, leaving `name` undefined on purpose
            // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
            return __assign(__assign({}, data), { name: cleanedURL === null || cleanedURL === void 0 ? void 0 : cleanedURL.url });
        };
    }
    config.maskRequestFn = (0, core_1.isFunction)(instanceConfig.session_recording.maskCapturedNetworkRequestFn)
        ? function (data) {
            var _a, _b, _c;
            var cleanedRequest = enforcedCleaningFn(data);
            return cleanedRequest
                ? ((_c = (_b = (_a = instanceConfig.session_recording).maskCapturedNetworkRequestFn) === null || _b === void 0 ? void 0 : _b.call(_a, cleanedRequest)) !== null && _c !== void 0 ? _c : undefined)
                : undefined;
        }
        : function (data) { return scrubPayloads(enforcedCleaningFn(data)); };
    return __assign(__assign(__assign({}, exports.defaultNetworkOptions), config), { recordHeaders: canRecordHeaders, recordBody: canRecordBody, recordPerformance: canRecordPerformance, recordInitialRequests: canRecordPerformance, streamNetworkBody: instanceConfig.session_recording.streamNetworkBody === true });
};
exports.buildNetworkRequestOptions = buildNetworkRequestOptions;
//# sourceMappingURL=config.js.map