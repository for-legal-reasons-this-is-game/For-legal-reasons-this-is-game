"use strict";
var __webpack_require__ = {};
(()=>{
    __webpack_require__.d = (exports1, definition)=>{
        for(var key in definition)if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports1, key)) Object.defineProperty(exports1, key, {
            enumerable: true,
            get: definition[key]
        });
    };
})();
(()=>{
    __webpack_require__.o = (obj, prop)=>Object.prototype.hasOwnProperty.call(obj, prop);
})();
(()=>{
    __webpack_require__.r = (exports1)=>{
        if ('undefined' != typeof Symbol && Symbol.toStringTag) Object.defineProperty(exports1, Symbol.toStringTag, {
            value: 'Module'
        });
        Object.defineProperty(exports1, '__esModule', {
            value: true
        });
    };
})();
var __webpack_exports__ = {};
__webpack_require__.r(__webpack_exports__);
__webpack_require__.d(__webpack_exports__, {
    jsonStringify: ()=>jsonStringify,
    formDataToQuery: ()=>formDataToQuery,
    getQueryParam: ()=>getQueryParam,
    _getHashParam: ()=>_getHashParam,
    convertToURL: ()=>convertToURL,
    isLocalhost: ()=>isLocalhost,
    updateStatusZeroFailureCount: ()=>updateStatusZeroFailureCount,
    isStatusZeroFailureCircuitBreakerTripped: ()=>isStatusZeroFailureCircuitBreakerTripped,
    maskQueryParams: ()=>maskQueryParams
});
const core_namespaceObject = require("@posthog/core");
const external_general_utils_js_namespaceObject = require("./general-utils.js");
const external_globals_js_namespaceObject = require("./globals.js");
const external_logger_js_namespaceObject = require("./logger.js");
const localDomains = [
    'localhost',
    '127.0.0.1'
];
const jsonStringify = (data, space)=>{
    try {
        return JSON.stringify(data, (_, value)=>'bigint' == typeof value ? value.toString() : value, space);
    } catch  {
        return (0, core_namespaceObject.safeJsonStringify)(data);
    }
};
const convertToURL = (url)=>{
    const location = external_globals_js_namespaceObject.document?.createElement('a');
    if ((0, core_namespaceObject.isUndefined)(location)) return null;
    location.href = url;
    return location;
};
const formDataToQuery = function(formdata, arg_separator = '&') {
    let use_val;
    let use_key;
    const tph_arr = [];
    (0, external_general_utils_js_namespaceObject.each)(formdata, function(val, key) {
        if ((0, core_namespaceObject.isUndefined)(val) || (0, core_namespaceObject.isUndefined)(key) || 'undefined' === key) return;
        use_val = encodeURIComponent((0, core_namespaceObject.isFile)(val) ? val.name : val.toString());
        use_key = encodeURIComponent(key);
        tph_arr[tph_arr.length] = use_key + '=' + use_val;
    });
    return tph_arr.join(arg_separator);
};
const getQueryParam = function(url, param) {
    const withoutHash = url.split('#')[0] || '';
    const queryParams = withoutHash.split(/\?(.*)/)[1] || '';
    const cleanedQueryParams = queryParams.replace(/^\?+/g, '');
    const queryParts = cleanedQueryParams.split('&');
    let keyValuePair;
    for(let i = 0; i < queryParts.length; i++){
        const parts = queryParts[i].split('=');
        if (parts[0] === param) {
            keyValuePair = parts;
            break;
        }
    }
    if (!(0, core_namespaceObject.isArray)(keyValuePair) || keyValuePair.length < 2) return '';
    {
        let result = keyValuePair[1];
        try {
            result = decodeURIComponent(result);
        } catch  {
            external_logger_js_namespaceObject.logger.error('Skipping decoding for malformed query param: ' + result);
        }
        return result.replace(/\+/g, ' ');
    }
};
const maskQueryParams = function(url, maskedParams, mask) {
    if (!url || !maskedParams || !maskedParams.length) return url;
    const splitHash = url.split('#');
    const withoutHash = splitHash[0] || '';
    const hash = splitHash[1];
    const splitQuery = withoutHash.split('?');
    const queryString = splitQuery[1];
    const urlWithoutQueryAndHash = splitQuery[0];
    const queryParts = (queryString || '').split('&');
    const paramStrings = [];
    for(let i = 0; i < queryParts.length; i++){
        const keyValuePair = queryParts[i].split('=');
        if ((0, core_namespaceObject.isArray)(keyValuePair)) if (maskedParams.includes(keyValuePair[0])) paramStrings.push(keyValuePair[0] + '=' + mask);
        else paramStrings.push(queryParts[i]);
    }
    let result = urlWithoutQueryAndHash;
    if (null != queryString) result += '?' + paramStrings.join('&');
    if (null != hash) result += '#' + hash;
    return result;
};
const _getHashParam = function(hash, param) {
    const matches = hash.match(new RegExp(param + '=([^&]*)'));
    return matches ? matches[1] : null;
};
const isLocalhost = ()=>localDomains.includes(external_globals_js_namespaceObject.location.hostname);
const isStatusZeroFailureCircuitBreakerTripped = (consecutiveStatusZeroFailures, maxConsecutiveStatusZeroFailures)=>consecutiveStatusZeroFailures >= maxConsecutiveStatusZeroFailures && (0, external_globals_js_namespaceObject.isBrowserOnline)();
const updateStatusZeroFailureCount = (statusCode, consecutiveStatusZeroFailures, maxConsecutiveStatusZeroFailures, onCircuitBreakerTripped)=>{
    if (0 === statusCode) {
        if ((0, external_globals_js_namespaceObject.isBrowserOnline)()) {
            const updatedConsecutiveStatusZeroFailures = consecutiveStatusZeroFailures + 1;
            if (updatedConsecutiveStatusZeroFailures === maxConsecutiveStatusZeroFailures) onCircuitBreakerTripped();
            return updatedConsecutiveStatusZeroFailures;
        }
        return consecutiveStatusZeroFailures;
    }
    return 0;
};
exports._getHashParam = __webpack_exports__._getHashParam;
exports.convertToURL = __webpack_exports__.convertToURL;
exports.formDataToQuery = __webpack_exports__.formDataToQuery;
exports.getQueryParam = __webpack_exports__.getQueryParam;
exports.isLocalhost = __webpack_exports__.isLocalhost;
exports.isStatusZeroFailureCircuitBreakerTripped = __webpack_exports__.isStatusZeroFailureCircuitBreakerTripped;
exports.jsonStringify = __webpack_exports__.jsonStringify;
exports.maskQueryParams = __webpack_exports__.maskQueryParams;
exports.updateStatusZeroFailureCount = __webpack_exports__.updateStatusZeroFailureCount;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "_getHashParam",
    "convertToURL",
    "formDataToQuery",
    "getQueryParam",
    "isLocalhost",
    "isStatusZeroFailureCircuitBreakerTripped",
    "jsonStringify",
    "maskQueryParams",
    "updateStatusZeroFailureCount"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
