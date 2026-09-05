import { detectDeviceType } from "@posthog/core";
import { navigator as external_globals_mjs_navigator, userAgent, window as external_globals_mjs_window } from "./globals.mjs";
import { propertyComparisons } from "./property-utils.mjs";
function doesDeviceTypeMatch(deviceTypes, matchType) {
    if (!deviceTypes || 0 === deviceTypes.length) return true;
    if (!userAgent) return false;
    const deviceType = detectDeviceType(userAgent, {
        userAgentDataPlatform: external_globals_mjs_navigator?.userAgentData?.platform,
        maxTouchPoints: external_globals_mjs_navigator?.maxTouchPoints,
        screenWidth: external_globals_mjs_window?.screen?.width,
        screenHeight: external_globals_mjs_window?.screen?.height,
        devicePixelRatio: external_globals_mjs_window?.devicePixelRatio
    });
    return propertyComparisons[matchType ?? 'icontains'](deviceTypes, [
        deviceType
    ]);
}
function hasPeriodPassed(periodDays, lastSeenDate) {
    if (!periodDays || !lastSeenDate) return true;
    const date = 'string' == typeof lastSeenDate ? new Date(lastSeenDate) : lastSeenDate;
    const now = new Date();
    const diffMs = Math.abs(now.getTime() - date.getTime());
    const diffDays = Math.ceil(diffMs / 86400000);
    return diffDays > periodDays;
}
export { doesDeviceTypeMatch, hasPeriodPassed };
