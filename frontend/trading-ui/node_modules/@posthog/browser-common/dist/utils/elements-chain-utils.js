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
    matchString: ()=>matchString,
    extractHref: ()=>extractHref,
    extractTexts: ()=>extractTexts,
    matchTexts: ()=>matchTexts
});
const core_namespaceObject = require("@posthog/core");
function extractHref(elementsChain) {
    const match = elementsChain.match(/(?::|")href="(.*?)"/);
    return match ? match[1] : '';
}
function extractTexts(elementsChain) {
    const texts = [];
    const regex = /(?::|")text="(.*?)"/g;
    let match;
    while(!(0, core_namespaceObject.isNullish)(match = regex.exec(elementsChain)))if (!texts.includes(match[1])) texts.push(match[1]);
    return texts;
}
function matchString(value, pattern, matching) {
    if ((0, core_namespaceObject.isNullish)(value)) return false;
    switch(matching){
        case 'exact':
            return value === pattern;
        case 'contains':
            {
                const likePattern = pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/_/g, '.').replace(/%/g, '.*');
                return new RegExp(likePattern, 'i').test(value);
            }
        case 'regex':
            try {
                return new RegExp(pattern).test(value);
            } catch  {
                return false;
            }
        default:
            return false;
    }
}
function matchTexts(texts, pattern, matching) {
    return texts.some((text)=>matchString(text, pattern, matching));
}
exports.extractHref = __webpack_exports__.extractHref;
exports.extractTexts = __webpack_exports__.extractTexts;
exports.matchString = __webpack_exports__.matchString;
exports.matchTexts = __webpack_exports__.matchTexts;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "extractHref",
    "extractTexts",
    "matchString",
    "matchTexts"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
