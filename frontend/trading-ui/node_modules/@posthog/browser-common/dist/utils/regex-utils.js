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
    isMatchingRegex: ()=>isMatchingRegex,
    isValidRegex: ()=>isValidRegex
});
const isValidRegex = function(str) {
    try {
        new RegExp(str);
    } catch  {
        return false;
    }
    return true;
};
const isMatchingRegex = function(value, pattern) {
    if (!isValidRegex(pattern)) return false;
    try {
        return new RegExp(pattern).test(value);
    } catch  {
        return false;
    }
};
exports.isMatchingRegex = __webpack_exports__.isMatchingRegex;
exports.isValidRegex = __webpack_exports__.isValidRegex;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "isMatchingRegex",
    "isValidRegex"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
