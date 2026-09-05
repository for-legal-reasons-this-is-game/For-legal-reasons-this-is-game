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
    AbortController: ()=>AbortController,
    CompressionStream: ()=>CompressionStream,
    XMLHttpRequest: ()=>XMLHttpRequest,
    document: ()=>document,
    fetch: ()=>fetch,
    isBrowserOnline: ()=>isBrowserOnline,
    location: ()=>location,
    navigator: ()=>navigator,
    userAgent: ()=>userAgent,
    window: ()=>win
});
const win = 'undefined' != typeof window ? window : void 0;
const global = 'undefined' != typeof globalThis ? globalThis : win;
const navigator = global?.navigator;
const document = global?.document;
const location = global?.location;
const fetch = global?.fetch;
const XMLHttpRequest = global?.XMLHttpRequest && 'withCredentials' in new global.XMLHttpRequest() ? global.XMLHttpRequest : void 0;
const AbortController = global?.AbortController;
const CompressionStream = global?.CompressionStream;
const userAgent = navigator?.userAgent;
function isBrowserOnline() {
    return !!(win && false !== win.navigator.onLine);
}
exports.AbortController = __webpack_exports__.AbortController;
exports.CompressionStream = __webpack_exports__.CompressionStream;
exports.XMLHttpRequest = __webpack_exports__.XMLHttpRequest;
exports.document = __webpack_exports__.document;
exports.fetch = __webpack_exports__.fetch;
exports.isBrowserOnline = __webpack_exports__.isBrowserOnline;
exports.location = __webpack_exports__.location;
exports.navigator = __webpack_exports__.navigator;
exports.userAgent = __webpack_exports__.userAgent;
exports.window = __webpack_exports__.window;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "AbortController",
    "CompressionStream",
    "XMLHttpRequest",
    "document",
    "fetch",
    "isBrowserOnline",
    "location",
    "navigator",
    "userAgent",
    "window"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
