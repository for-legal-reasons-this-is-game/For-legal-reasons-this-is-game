import { convertToURL, getQueryParam, maskQueryParams } from "./request-utils.mjs";
import { detectBrowser, detectBrowserVersion, detectDevice, detectDeviceType, detectOS, isNull, isUndefined, stripLeadingDollar, stripUrlHash } from "@posthog/core";
import config from "../config.mjs";
import { SDK_DIST_CHANNEL } from "../constants.mjs";
import { each, extend, stripEmptyProperties } from "./general-utils.mjs";
import { document as external_globals_mjs_document, location as external_globals_mjs_location, userAgent, window as external_globals_mjs_window } from "./globals.mjs";
import { getCookieValue } from "./cookie-utils.mjs";
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
    if (!external_globals_mjs_document) return {};
    const paramsToMask = maskPersonalDataProperties ? [
        ...PERSONAL_DATA_CAMPAIGN_PARAMS,
        ...customPersonalDataProperties || []
    ] : [];
    const urlCampaignParams = _getCampaignParamsFromUrl(maskQueryParams(external_globals_mjs_document.URL, paramsToMask, MASKED), customTrackedParams);
    const cookieCampaignParams = _getCampaignParamsFromCookie();
    return extend(cookieCampaignParams, urlCampaignParams);
}
function _getCampaignParamsFromUrl(url, customParams) {
    const campaign_keywords = CAMPAIGN_PARAMS.concat(customParams || []);
    const params = {};
    each(campaign_keywords, function(kwkey) {
        const kw = getQueryParam(url, kwkey);
        params[kwkey] = kw ? kw : null;
    });
    return params;
}
function _getCampaignParamsFromCookie() {
    const params = {};
    each(COOKIE_CAMPAIGN_PARAMS, function(kwkey) {
        const kw = getCookieValue(kwkey);
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
    if (!isNull(search)) {
        ret['$search_engine'] = search;
        const keyword = external_globals_mjs_document ? getQueryParam(external_globals_mjs_document.referrer, param) : '';
        if (keyword.length) ret['ph_keyword'] = keyword;
    }
    return ret;
}
function getSearchInfo() {
    const referrer = external_globals_mjs_document?.referrer;
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
    return external_globals_mjs_document?.referrer || DIRECT;
}
function getReferringDomain() {
    if (!external_globals_mjs_document?.referrer) return DIRECT;
    return convertToURL(external_globals_mjs_document.referrer)?.host || DIRECT;
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
    const href = disableCaptureUrlHashes ? stripUrlHash(external_globals_mjs_location?.href) : external_globals_mjs_location?.href;
    const url = href?.substring(0, 1000);
    return {
        r: getReferrer().substring(0, 1000),
        u: url ? maskQueryParams(url, paramsToMask, MASKED) : void 0
    };
}
function getPersonPropsFromInfo(info, disableCaptureUrlHashes = false) {
    const { r: referrer, u } = info;
    const url = disableCaptureUrlHashes ? stripUrlHash(u) : u;
    const referring_domain = null == referrer ? void 0 : referrer == DIRECT ? DIRECT : convertToURL(referrer)?.host;
    const props = {
        $referrer: referrer,
        $referring_domain: referring_domain
    };
    if (url) {
        props['$current_url'] = url;
        const location = convertToURL(url);
        props['$host'] = location?.host;
        props['$pathname'] = location?.pathname;
        const campaignParams = _getCampaignParamsFromUrl(url);
        extend(props, campaignParams);
    }
    if (referrer) {
        const searchInfo = _getSearchInfoFromReferrer(referrer);
        extend(props, searchInfo);
    }
    return props;
}
function getInitialPersonPropsFromInfo(info, disableCaptureUrlHashes = false) {
    const personProps = getPersonPropsFromInfo(info, disableCaptureUrlHashes);
    const props = {};
    each(personProps, function(val, key) {
        props[`$initial_${stripLeadingDollar(key)}`] = val;
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
    if (!userAgent) return {};
    const paramsToMask = maskPersonalDataProperties ? [
        ...PERSONAL_DATA_CAMPAIGN_PARAMS,
        ...customPersonalDataProperties || []
    ] : [];
    const [os_name, os_version] = detectOS(userAgent);
    const browserHints = getBrowserDetectionHints();
    const browserOptions = {};
    if (!isUndefined(detectGoogleSearchApp)) browserOptions.detectGoogleSearchApp = detectGoogleSearchApp;
    const deviceOptions = {};
    const userAgentDataPlatform = navigator?.userAgentData?.platform;
    const maxTouchPoints = navigator?.maxTouchPoints;
    const screenWidth = external_globals_mjs_window?.screen?.width;
    const screenHeight = external_globals_mjs_window?.screen?.height;
    const devicePixelRatio = external_globals_mjs_window?.devicePixelRatio;
    if (!isUndefined(userAgentDataPlatform)) deviceOptions.userAgentDataPlatform = userAgentDataPlatform;
    if (!isUndefined(maxTouchPoints)) deviceOptions.maxTouchPoints = maxTouchPoints;
    if (!isUndefined(screenWidth)) deviceOptions.screenWidth = screenWidth;
    if (!isUndefined(screenHeight)) deviceOptions.screenHeight = screenHeight;
    if (!isUndefined(devicePixelRatio)) deviceOptions.devicePixelRatio = devicePixelRatio;
    const properties = extend(stripEmptyProperties({
        $os: os_name,
        $os_version: os_version,
        $browser: detectBrowser(userAgent, navigator.vendor, browserHints, browserOptions),
        $device: detectDevice(userAgent),
        $device_type: detectDeviceType(userAgent, deviceOptions),
        $timezone: getTimezone(),
        $timezone_offset: getTimezoneOffset()
    }), {
        $current_url: maskQueryParams(disableCaptureUrlHashes ? stripUrlHash(external_globals_mjs_location?.href) : external_globals_mjs_location?.href, paramsToMask, MASKED),
        $host: external_globals_mjs_location?.host,
        $pathname: external_globals_mjs_location?.pathname,
        $raw_user_agent: userAgent.length > 1000 ? userAgent.substring(0, 997) + '...' : userAgent,
        $browser_version: detectBrowserVersion(userAgent, navigator.vendor, browserHints, browserOptions),
        $browser_language: getBrowserLanguage(),
        $browser_language_prefix: getBrowserLanguagePrefix(),
        $screen_height: external_globals_mjs_window?.screen.height,
        $screen_width: external_globals_mjs_window?.screen.width,
        $viewport_height: external_globals_mjs_window?.innerHeight,
        $viewport_width: external_globals_mjs_window?.innerWidth,
        $lib: config.LIB_NAME,
        $lib_version: config.LIB_VERSION,
        $insert_id: Math.random().toString(36).substring(2, 10) + Math.random().toString(36).substring(2, 10),
        $time: Date.now() / 1000
    });
    if (config.SDK_DIST_CHANNEL) properties[SDK_DIST_CHANNEL] = config.SDK_DIST_CHANNEL;
    return properties;
}
export { CAMPAIGN_PARAMS, COOKIE_CAMPAIGN_PARAMS, EVENT_TO_PERSON_PROPERTIES, MASKED, PERSONAL_DATA_CAMPAIGN_PARAMS, getBrowserDetectionHints, getBrowserLanguage, getBrowserLanguagePrefix, getCampaignParams, getEventProperties, getInitialPersonPropsFromInfo, getPersonInfo, getPersonPropsFromInfo, getReferrer, getReferrerInfo, getReferringDomain, getSearchInfo, getTimezone, getTimezoneOffset };
