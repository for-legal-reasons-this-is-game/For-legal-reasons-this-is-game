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
    DEFAULT_BLOCKED_UA_STRS: ()=>core_namespaceObject.DEFAULT_BLOCKED_UA_STRS,
    isBlockedUA: ()=>core_namespaceObject.isBlockedUA,
    isLikelyBot: ()=>isLikelyBot
});
const core_namespaceObject = require("@posthog/core");
const isLikelyBot = function(navigator, customBlockedUserAgents) {
    if (!navigator) return false;
    const ua = navigator.userAgent;
    if (ua) {
        if ((0, core_namespaceObject.isBlockedUA)(ua, customBlockedUserAgents)) return true;
    }
    try {
        const uaData = navigator?.userAgentData;
        if (uaData?.brands && uaData.brands.some((brandObj)=>(0, core_namespaceObject.isBlockedUA)(brandObj?.brand, customBlockedUserAgents))) return true;
    } catch  {}
    return !!navigator.webdriver;
};
exports.DEFAULT_BLOCKED_UA_STRS = __webpack_exports__.DEFAULT_BLOCKED_UA_STRS;
exports.isBlockedUA = __webpack_exports__.isBlockedUA;
exports.isLikelyBot = __webpack_exports__.isLikelyBot;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "DEFAULT_BLOCKED_UA_STRS",
    "isBlockedUA",
    "isLikelyBot"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
