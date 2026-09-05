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
    isTag: ()=>isTag,
    isElementInToolbar: ()=>isElementInToolbar,
    isShadowRoot: ()=>isShadowRoot,
    isTextNode: ()=>isTextNode,
    isElementNode: ()=>isElementNode
});
const external_constants_js_namespaceObject = require("../constants.js");
const NODE_TYPE_ELEMENT = 1;
const NODE_TYPE_TEXT = 3;
const NODE_TYPE_DOCUMENT_FRAGMENT = 11;
function isElementInToolbar(el) {
    if (el instanceof Element) return el.id === external_constants_js_namespaceObject.TOOLBAR_ID || !!el.closest?.('.' + external_constants_js_namespaceObject.TOOLBAR_CONTAINER_CLASS);
    return false;
}
function isElementNode(el) {
    return !!el && el.nodeType === NODE_TYPE_ELEMENT;
}
function isTag(el, tag) {
    return !!el && !!el.tagName && el.tagName.toLowerCase() === tag.toLowerCase();
}
function isTextNode(el) {
    return !!el && el.nodeType === NODE_TYPE_TEXT;
}
function isShadowRoot(el) {
    return !!el && el.nodeType === NODE_TYPE_DOCUMENT_FRAGMENT && isElementNode(el.host);
}
exports.isElementInToolbar = __webpack_exports__.isElementInToolbar;
exports.isElementNode = __webpack_exports__.isElementNode;
exports.isShadowRoot = __webpack_exports__.isShadowRoot;
exports.isTag = __webpack_exports__.isTag;
exports.isTextNode = __webpack_exports__.isTextNode;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "isElementInToolbar",
    "isElementNode",
    "isShadowRoot",
    "isTag",
    "isTextNode"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
