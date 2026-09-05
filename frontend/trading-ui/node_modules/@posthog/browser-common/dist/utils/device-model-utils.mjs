import { isString } from "@posthog/core";
import { navigator as external_globals_mjs_navigator } from "./globals.mjs";
import { logger } from "./logger.mjs";
async function getDeviceModel() {
    const uaData = external_globals_mjs_navigator?.userAgentData;
    if (!uaData?.getHighEntropyValues) return;
    try {
        const hints = await uaData.getHighEntropyValues([
            'model'
        ]);
        const model = hints?.model;
        return isString(model) && model.length > 0 ? model : void 0;
    } catch (e) {
        logger.info('Unable to resolve $device_model from userAgentData.getHighEntropyValues', e);
        return;
    }
}
export { getDeviceModel };
