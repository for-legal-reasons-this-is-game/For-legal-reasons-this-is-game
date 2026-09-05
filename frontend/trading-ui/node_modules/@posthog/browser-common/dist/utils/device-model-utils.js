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
    getDeviceModel: ()=>getDeviceModel
});
const core_namespaceObject = require("@posthog/core");
const external_globals_js_namespaceObject = require("./globals.js");
const external_logger_js_namespaceObject = require("./logger.js");
async function getDeviceModel() {
    const uaData = external_globals_js_namespaceObject.navigator?.userAgentData;
    if (!uaData?.getHighEntropyValues) return;
    try {
        const hints = await uaData.getHighEntropyValues([
            'model'
        ]);
        const model = hints?.model;
        return (0, core_namespaceObject.isString)(model) && model.length > 0 ? model : void 0;
    } catch (e) {
        external_logger_js_namespaceObject.logger.info('Unable to resolve $device_model from userAgentData.getHighEntropyValues', e);
        return;
    }
}
exports.getDeviceModel = __webpack_exports__.getDeviceModel;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "getDeviceModel"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
