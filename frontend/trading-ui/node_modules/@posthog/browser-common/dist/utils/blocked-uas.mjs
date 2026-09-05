import { DEFAULT_BLOCKED_UA_STRS, isBlockedUA } from "@posthog/core";
const isLikelyBot = function(navigator, customBlockedUserAgents) {
    if (!navigator) return false;
    const ua = navigator.userAgent;
    if (ua) {
        if (isBlockedUA(ua, customBlockedUserAgents)) return true;
    }
    try {
        const uaData = navigator?.userAgentData;
        if (uaData?.brands && uaData.brands.some((brandObj)=>isBlockedUA(brandObj?.brand, customBlockedUserAgents))) return true;
    } catch  {}
    return !!navigator.webdriver;
};
export { DEFAULT_BLOCKED_UA_STRS, isBlockedUA, isLikelyBot };
