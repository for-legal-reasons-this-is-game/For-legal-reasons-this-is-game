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
    SDK_DIST_CHANNEL: ()=>SDK_DIST_CHANNEL,
    TOOLBAR_CONTAINER_CLASS: ()=>TOOLBAR_CONTAINER_CLASS,
    TOOLBAR_ID: ()=>TOOLBAR_ID
});
const TOOLBAR_ID = '__POSTHOG_TOOLBAR__';
const TOOLBAR_CONTAINER_CLASS = 'toolbar-global-fade-container';
const SDK_DIST_CHANNEL = '$sdk_dist_channel';
exports.SDK_DIST_CHANNEL = __webpack_exports__.SDK_DIST_CHANNEL;
exports.TOOLBAR_CONTAINER_CLASS = __webpack_exports__.TOOLBAR_CONTAINER_CLASS;
exports.TOOLBAR_ID = __webpack_exports__.TOOLBAR_ID;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "SDK_DIST_CHANNEL",
    "TOOLBAR_CONTAINER_CLASS",
    "TOOLBAR_ID"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
