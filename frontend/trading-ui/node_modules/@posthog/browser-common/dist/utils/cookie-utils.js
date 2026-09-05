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
    getCookieValue: ()=>getCookieValue
});
const external_globals_js_namespaceObject = require("./globals.js");
const getCookieValue = (name)=>{
    if (!external_globals_js_namespaceObject.document) return;
    try {
        const nameEQ = name + '=';
        const cookies = external_globals_js_namespaceObject.document.cookie.split(';').filter((cookie)=>cookie.length);
        for(let i = 0; i < cookies.length; i++){
            let cookie = cookies[i];
            while(' ' == cookie.charAt(0))cookie = cookie.substring(1, cookie.length);
            if (0 === cookie.indexOf(nameEQ)) return decodeURIComponent(cookie.substring(nameEQ.length, cookie.length));
        }
    } catch  {}
    return null;
};
exports.getCookieValue = __webpack_exports__.getCookieValue;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "getCookieValue"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
