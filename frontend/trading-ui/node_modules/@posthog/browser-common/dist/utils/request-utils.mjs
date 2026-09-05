import { isArray, isFile, isUndefined, safeJsonStringify } from "@posthog/core";
import { each } from "./general-utils.mjs";
import { document as external_globals_mjs_document, isBrowserOnline, location as external_globals_mjs_location } from "./globals.mjs";
import { logger } from "./logger.mjs";
const localDomains = [
    'localhost',
    '127.0.0.1'
];
const jsonStringify = (data, space)=>{
    try {
        return JSON.stringify(data, (_, value)=>'bigint' == typeof value ? value.toString() : value, space);
    } catch  {
        return safeJsonStringify(data);
    }
};
const convertToURL = (url)=>{
    const location = external_globals_mjs_document?.createElement('a');
    if (isUndefined(location)) return null;
    location.href = url;
    return location;
};
const formDataToQuery = function(formdata, arg_separator = '&') {
    let use_val;
    let use_key;
    const tph_arr = [];
    each(formdata, function(val, key) {
        if (isUndefined(val) || isUndefined(key) || 'undefined' === key) return;
        use_val = encodeURIComponent(isFile(val) ? val.name : val.toString());
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
    if (!isArray(keyValuePair) || keyValuePair.length < 2) return '';
    {
        let result = keyValuePair[1];
        try {
            result = decodeURIComponent(result);
        } catch  {
            logger.error('Skipping decoding for malformed query param: ' + result);
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
        if (isArray(keyValuePair)) if (maskedParams.includes(keyValuePair[0])) paramStrings.push(keyValuePair[0] + '=' + mask);
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
const isLocalhost = ()=>localDomains.includes(external_globals_mjs_location.hostname);
const isStatusZeroFailureCircuitBreakerTripped = (consecutiveStatusZeroFailures, maxConsecutiveStatusZeroFailures)=>consecutiveStatusZeroFailures >= maxConsecutiveStatusZeroFailures && isBrowserOnline();
const updateStatusZeroFailureCount = (statusCode, consecutiveStatusZeroFailures, maxConsecutiveStatusZeroFailures, onCircuitBreakerTripped)=>{
    if (0 === statusCode) {
        if (isBrowserOnline()) {
            const updatedConsecutiveStatusZeroFailures = consecutiveStatusZeroFailures + 1;
            if (updatedConsecutiveStatusZeroFailures === maxConsecutiveStatusZeroFailures) onCircuitBreakerTripped();
            return updatedConsecutiveStatusZeroFailures;
        }
        return consecutiveStatusZeroFailures;
    }
    return 0;
};
export { _getHashParam, convertToURL, formDataToQuery, getQueryParam, isLocalhost, isStatusZeroFailureCircuitBreakerTripped, jsonStringify, maskQueryParams, updateStatusZeroFailureCount };
