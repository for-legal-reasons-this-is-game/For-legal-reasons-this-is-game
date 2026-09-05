"use strict";
var __webpack_require__ = {};
(()=>{
    __webpack_require__.n = (module)=>{
        var getter = module && module.__esModule ? ()=>module['default'] : ()=>module;
        __webpack_require__.d(getter, {
            a: getter
        });
        return getter;
    };
})();
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
    getCampaignParams: ()=>getCampaignParams,
    getReferrerInfo: ()=>getReferrerInfo,
    getEventProperties: ()=>getEventProperties,
    getPersonPropsFromInfo: ()=>getPersonPropsFromInfo,
    COOKIE_CAMPAIGN_PARAMS: ()=>COOKIE_CAMPAIGN_PARAMS,
    getReferrer: ()=>getReferrer,
    getReferringDomain: ()=>getReferringDomain,
    getTimezone: ()=>getTimezone,
    getBrowserDetectionHints: ()=>getBrowserDetectionHints,
    getBrowserLanguagePrefix: ()=>getBrowserLanguagePrefix,
    getInitialPersonPropsFromInfo: ()=>getInitialPersonPropsFromInfo,
    MASKED: ()=>MASKED,
    getPersonInfo: ()=>getPersonInfo,
    CAMPAIGN_PARAMS: ()=>CAMPAIGN_PARAMS,
    getBrowserLanguage: ()=>getBrowserLanguage,
    PERSONAL_DATA_CAMPAIGN_PARAMS: ()=>PERSONAL_DATA_CAMPAIGN_PARAMS,
    getSearchInfo: ()=>getSearchInfo,
    getTimezoneOffset: ()=>getTimezoneOffset,
    EVENT_TO_PERSON_PROPERTIES: ()=>EVENT_TO_PERSON_PROPERTIES
});
const external_request_utils_js_namespaceObject = require("./request-utils.js");
const core_namespaceObject = require("@posthog/core");
const external_config_js_namespaceObject = require("../config.js");
var external_config_js_default = /*#__PURE__*/ __webpack_require__.n(external_config_js_namespaceObject);
const external_constants_js_namespaceObject = require("../constants.js");
const external_general_utils_js_namespaceObject = require("./general-utils.js");
const external_globals_js_namespaceObject = require("./globals.js");
const external_cookie_utils_js_namespaceObject = require("./cookie-utils.js");
const URL_REGEX_PREFIX = 'https?://(.*)';
const PERSONAL_DATA_CAMPAIGN_PARAMS = [
    'gclid',
    'gclsrc',
    'dclid',
    'gbraid',
    'wbraid',
    'fbclid',
    'msclkid',
    'twclid',
    'li_fat_id',
    'igshid',
    'ttclid',
    'rdt_cid',
    'epik',
    'qclid',
    'sccid',
    'irclid',
    '_kx'
];
const CAMPAIGN_PARAMS = [
    'utm_source',
    'utm_medium',
    'utm_campaign',
    'utm_content',
    'utm_term',
    'gad_source',
    'mc_cid',
    ...PERSONAL_DATA_CAMPAIGN_PARAMS
];
const EVENT_TO_PERSON_PROPERTIES = [
    '$app_build',
    '$app_name',
    '$app_namespace',
    '$app_version',
    '$browser',
    '$browser_version',
    '$device_type',
    '$current_url',
    '$pathname',
    '$os',
    '$os_name',
    '$os_version',
    '$referring_domain',
    '$referrer',
    '$screen_height',
    '$screen_width',
    '$viewport_height',
    '$viewport_width',
    '$raw_user_agent'
];
const MASKED = '<masked>';
const COOKIE_CAMPAIGN_PARAMS = [
    'li_fat_id'
];
function getCampaignParams(customTrackedParams, maskPersonalDataProperties, customPersonalDataProperties) {
    if (!external_globals_js_namespaceObject.document) return {};
    const paramsToMask = maskPersonalDataProperties ? [
        ...PERSONAL_DATA_CAMPAIGN_PARAMS,
        ...customPersonalDataProperties || []
    ] : [];
    const urlCampaignParams = _getCampaignParamsFromUrl((0, external_request_utils_js_namespaceObject.maskQueryParams)(external_globals_js_namespaceObject.document.URL, paramsToMask, MASKED), customTrackedParams);
    const cookieCampaignParams = _getCampaignParamsFromCookie();
    return (0, external_general_utils_js_namespaceObject.extend)(cookieCampaignParams, urlCampaignParams);
}
function _getCampaignParamsFromUrl(url, customParams) {
    const campaign_keywords = CAMPAIGN_PARAMS.concat(customParams || []);
    const params = {};
    (0, external_general_utils_js_namespaceObject.each)(campaign_keywords, function(kwkey) {
        const kw = (0, external_request_utils_js_namespaceObject.getQueryParam)(url, kwkey);
        params[kwkey] = kw ? kw : null;
    });
    return params;
}
function _getCampaignParamsFromCookie() {
    const params = {};
    (0, external_general_utils_js_namespaceObject.each)(COOKIE_CAMPAIGN_PARAMS, function(kwkey) {
        const kw = (0, external_cookie_utils_js_namespaceObject.getCookieValue)(kwkey);
        params[kwkey] = kw ? kw : null;
    });
    return params;
}
function _getSearchEngine(referrer) {
    if (!referrer) return null;
    if (0 === referrer.search(URL_REGEX_PREFIX + 'google.([^/?]*)')) return 'google';
    if (0 === referrer.search(URL_REGEX_PREFIX + 'bing.com')) return 'bing';
    if (0 === referrer.search(URL_REGEX_PREFIX + 'yahoo.com')) return 'yahoo';
    if (0 === referrer.search(URL_REGEX_PREFIX + 'duckduckgo.com')) return 'duckduckgo';
    else return null;
}
function _getSearchInfoFromReferrer(referrer) {
    const search = _getSearchEngine(referrer);
    const param = 'yahoo' != search ? 'q' : 'p';
    const ret = {};
    if (!(0, core_namespaceObject.isNull)(search)) {
        ret['$search_engine'] = search;
        const keyword = external_globals_js_namespaceObject.document ? (0, external_request_utils_js_namespaceObject.getQueryParam)(external_globals_js_namespaceObject.document.referrer, param) : '';
        if (keyword.length) ret['ph_keyword'] = keyword;
    }
    return ret;
}
function getSearchInfo() {
    const referrer = external_globals_js_namespaceObject.document?.referrer;
    if (!referrer) return {};
    return _getSearchInfoFromReferrer(referrer);
}
function getBrowserLanguage() {
    return navigator.language || navigator.userLanguage;
}
function getBrowserLanguagePrefix() {
    const lang = getBrowserLanguage();
    return 'string' == typeof lang ? lang.split('-')[0] : void 0;
}
const DIRECT = '$direct';
function getReferrer() {
    return external_globals_js_namespaceObject.document?.referrer || DIRECT;
}
function getReferringDomain() {
    if (!external_globals_js_namespaceObject.document?.referrer) return DIRECT;
    return (0, external_request_utils_js_namespaceObject.convertToURL)(external_globals_js_namespaceObject.document.referrer)?.host || DIRECT;
}
function getReferrerInfo() {
    return {
        $referrer: getReferrer(),
        $referring_domain: getReferringDomain()
    };
}
function getPersonInfo(maskPersonalDataProperties, customPersonalDataProperties, disableCaptureUrlHashes = false) {
    const paramsToMask = maskPersonalDataProperties ? [
        ...PERSONAL_DATA_CAMPAIGN_PARAMS,
        ...customPersonalDataProperties || []
    ] : [];
    const href = disableCaptureUrlHashes ? (0, core_namespaceObject.stripUrlHash)(external_globals_js_namespaceObject.location?.href) : external_globals_js_namespaceObject.location?.href;
    const url = href?.substring(0, 1000);
    return {
        r: getReferrer().substring(0, 1000),
        u: url ? (0, external_request_utils_js_namespaceObject.maskQueryParams)(url, paramsToMask, MASKED) : void 0
    };
}
function getPersonPropsFromInfo(info, disableCaptureUrlHashes = false) {
    const { r: referrer, u } = info;
    const url = disableCaptureUrlHashes ? (0, core_namespaceObject.stripUrlHash)(u) : u;
    const referring_domain = null == referrer ? void 0 : referrer == DIRECT ? DIRECT : (0, external_request_utils_js_namespaceObject.convertToURL)(referrer)?.host;
    const props = {
        $referrer: referrer,
        $referring_domain: referring_domain
    };
    if (url) {
        props['$current_url'] = url;
        const location = (0, external_request_utils_js_namespaceObject.convertToURL)(url);
        props['$host'] = location?.host;
        props['$pathname'] = location?.pathname;
        const campaignParams = _getCampaignParamsFromUrl(url);
        (0, external_general_utils_js_namespaceObject.extend)(props, campaignParams);
    }
    if (referrer) {
        const searchInfo = _getSearchInfoFromReferrer(referrer);
        (0, external_general_utils_js_namespaceObject.extend)(props, searchInfo);
    }
    return props;
}
function getInitialPersonPropsFromInfo(info, disableCaptureUrlHashes = false) {
    const personProps = getPersonPropsFromInfo(info, disableCaptureUrlHashes);
    const props = {};
    (0, external_general_utils_js_namespaceObject.each)(personProps, function(val, key) {
        props[`$initial_${(0, core_namespaceObject.stripLeadingDollar)(key)}`] = val;
    });
    return props;
}
function getTimezone() {
    try {
        return Intl.DateTimeFormat().resolvedOptions().timeZone;
    } catch  {
        return;
    }
}
function getTimezoneOffset() {
    try {
        return new Date().getTimezoneOffset();
    } catch  {
        return;
    }
}
function getBrowserDetectionHints() {
    const nav = 'undefined' != typeof navigator ? navigator : void 0;
    return nav?.brave ? {
        brave: true
    } : {};
}
function getEventProperties(maskPersonalDataProperties, customPersonalDataProperties, detectGoogleSearchApp, disableCaptureUrlHashes = false) {
    if (!external_globals_js_namespaceObject.userAgent) return {};
    const paramsToMask = maskPersonalDataProperties ? [
        ...PERSONAL_DATA_CAMPAIGN_PARAMS,
        ...customPersonalDataProperties || []
    ] : [];
    const [os_name, os_version] = (0, core_namespaceObject.detectOS)(external_globals_js_namespaceObject.userAgent);
    const browserHints = getBrowserDetectionHints();
    const browserOptions = {};
    if (!(0, core_namespaceObject.isUndefined)(detectGoogleSearchApp)) browserOptions.detectGoogleSearchApp = detectGoogleSearchApp;
    const deviceOptions = {};
    const userAgentDataPlatform = navigator?.userAgentData?.platform;
    const maxTouchPoints = navigator?.maxTouchPoints;
    const screenWidth = external_globals_js_namespaceObject.window?.screen?.width;
    const screenHeight = external_globals_js_namespaceObject.window?.screen?.height;
    const devicePixelRatio = external_globals_js_namespaceObject.window?.devicePixelRatio;
    if (!(0, core_namespaceObject.isUndefined)(userAgentDataPlatform)) deviceOptions.userAgentDataPlatform = userAgentDataPlatform;
    if (!(0, core_namespaceObject.isUndefined)(maxTouchPoints)) deviceOptions.maxTouchPoints = maxTouchPoints;
    if (!(0, core_namespaceObject.isUndefined)(screenWidth)) deviceOptions.screenWidth = screenWidth;
    if (!(0, core_namespaceObject.isUndefined)(screenHeight)) deviceOptions.screenHeight = screenHeight;
    if (!(0, core_namespaceObject.isUndefined)(devicePixelRatio)) deviceOptions.devicePixelRatio = devicePixelRatio;
    const properties = (0, external_general_utils_js_namespaceObject.extend)((0, external_general_utils_js_namespaceObject.stripEmptyProperties)({
        $os: os_name,
        $os_version: os_version,
        $browser: (0, core_namespaceObject.detectBrowser)(external_globals_js_namespaceObject.userAgent, navigator.vendor, browserHints, browserOptions),
        $device: (0, core_namespaceObject.detectDevice)(external_globals_js_namespaceObject.userAgent),
        $device_type: (0, core_namespaceObject.detectDeviceType)(external_globals_js_namespaceObject.userAgent, deviceOptions),
        $timezone: getTimezone(),
        $timezone_offset: getTimezoneOffset()
    }), {
        $current_url: (0, external_request_utils_js_namespaceObject.maskQueryParams)(disableCaptureUrlHashes ? (0, core_namespaceObject.stripUrlHash)(external_globals_js_namespaceObject.location?.href) : external_globals_js_namespaceObject.location?.href, paramsToMask, MASKED),
        $host: external_globals_js_namespaceObject.location?.host,
        $pathname: external_globals_js_namespaceObject.location?.pathname,
        $raw_user_agent: external_globals_js_namespaceObject.userAgent.length > 1000 ? external_globals_js_namespaceObject.userAgent.substring(0, 997) + '...' : external_globals_js_namespaceObject.userAgent,
        $browser_version: (0, core_namespaceObject.detectBrowserVersion)(external_globals_js_namespaceObject.userAgent, navigator.vendor, browserHints, browserOptions),
        $browser_language: getBrowserLanguage(),
        $browser_language_prefix: getBrowserLanguagePrefix(),
        $screen_height: external_globals_js_namespaceObject.window?.screen.height,
        $screen_width: external_globals_js_namespaceObject.window?.screen.width,
        $viewport_height: external_globals_js_namespaceObject.window?.innerHeight,
        $viewport_width: external_globals_js_namespaceObject.window?.innerWidth,
        $lib: external_config_js_default().LIB_NAME,
        $lib_version: external_config_js_default().LIB_VERSION,
        $insert_id: Math.random().toString(36).substring(2, 10) + Math.random().toString(36).substring(2, 10),
        $time: Date.now() / 1000
    });
    if (external_config_js_default().SDK_DIST_CHANNEL) properties[external_constants_js_namespaceObject.SDK_DIST_CHANNEL] = external_config_js_default().SDK_DIST_CHANNEL;
    return properties;
}
exports.CAMPAIGN_PARAMS = __webpack_exports__.CAMPAIGN_PARAMS;
exports.COOKIE_CAMPAIGN_PARAMS = __webpack_exports__.COOKIE_CAMPAIGN_PARAMS;
exports.EVENT_TO_PERSON_PROPERTIES = __webpack_exports__.EVENT_TO_PERSON_PROPERTIES;
exports.MASKED = __webpack_exports__.MASKED;
exports.PERSONAL_DATA_CAMPAIGN_PARAMS = __webpack_exports__.PERSONAL_DATA_CAMPAIGN_PARAMS;
exports.getBrowserDetectionHints = __webpack_exports__.getBrowserDetectionHints;
exports.getBrowserLanguage = __webpack_exports__.getBrowserLanguage;
exports.getBrowserLanguagePrefix = __webpack_exports__.getBrowserLanguagePrefix;
exports.getCampaignParams = __webpack_exports__.getCampaignParams;
exports.getEventProperties = __webpack_exports__.getEventProperties;
exports.getInitialPersonPropsFromInfo = __webpack_exports__.getInitialPersonPropsFromInfo;
exports.getPersonInfo = __webpack_exports__.getPersonInfo;
exports.getPersonPropsFromInfo = __webpack_exports__.getPersonPropsFromInfo;
exports.getReferrer = __webpack_exports__.getReferrer;
exports.getReferrerInfo = __webpack_exports__.getReferrerInfo;
exports.getReferringDomain = __webpack_exports__.getReferringDomain;
exports.getSearchInfo = __webpack_exports__.getSearchInfo;
exports.getTimezone = __webpack_exports__.getTimezone;
exports.getTimezoneOffset = __webpack_exports__.getTimezoneOffset;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "CAMPAIGN_PARAMS",
    "COOKIE_CAMPAIGN_PARAMS",
    "EVENT_TO_PERSON_PROPERTIES",
    "MASKED",
    "PERSONAL_DATA_CAMPAIGN_PARAMS",
    "getBrowserDetectionHints",
    "getBrowserLanguage",
    "getBrowserLanguagePrefix",
    "getCampaignParams",
    "getEventProperties",
    "getInitialPersonPropsFromInfo",
    "getPersonInfo",
    "getPersonPropsFromInfo",
    "getReferrer",
    "getReferrerInfo",
    "getReferringDomain",
    "getSearchInfo",
    "getTimezone",
    "getTimezoneOffset"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
