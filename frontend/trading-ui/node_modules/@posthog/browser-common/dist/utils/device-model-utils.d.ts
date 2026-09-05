/**
 * Reads the hardware model from `navigator.userAgentData.getHighEntropyValues(['model'])`.
 *
 * Only meaningful on Android Chromium — `undefined` on Safari/Firefox and an empty string on desktop
 * (both treated as absent). A Permissions-Policy block rejects with `NotAllowedError`, which we catch
 * and return `undefined`.
 */
export declare function getDeviceModel(): Promise<string | undefined>;
