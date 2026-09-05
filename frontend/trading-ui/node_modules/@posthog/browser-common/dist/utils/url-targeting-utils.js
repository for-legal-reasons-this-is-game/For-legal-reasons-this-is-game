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
    getTargetingUrl: ()=>getTargetingUrl,
    applyUrlTargetingOverride: ()=>applyUrlTargetingOverride
});
const core_namespaceObject = require("@posthog/core");
const external_globals_js_namespaceObject = require("./globals.js");
const external_logger_js_namespaceObject = require("./logger.js");
function applyUrlTargetingOverride(instance, defaultUrl) {
    const override = instance?.config?.get_current_url;
    if (!(0, core_namespaceObject.isFunction)(override)) return defaultUrl;
    try {
        const result = override(defaultUrl);
        return (0, core_namespaceObject.isString)(result) && result ? result : defaultUrl;
    } catch (e) {
        external_logger_js_namespaceObject.logger.error('Error in get_current_url, falling back to window.location.href', e);
        return defaultUrl;
    }
}
function getTargetingUrl(instance) {
    const defaultUrl = external_globals_js_namespaceObject.window?.location?.href;
    return (0, core_namespaceObject.isUndefined)(defaultUrl) ? void 0 : applyUrlTargetingOverride(instance, defaultUrl);
}
exports.applyUrlTargetingOverride = __webpack_exports__.applyUrlTargetingOverride;
exports.getTargetingUrl = __webpack_exports__.getTargetingUrl;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "applyUrlTargetingOverride",
    "getTargetingUrl"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
