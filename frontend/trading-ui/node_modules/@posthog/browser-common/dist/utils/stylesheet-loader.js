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
    prepareStylesheet: ()=>prepareStylesheet
});
const core_namespaceObject = require("@posthog/core");
const external_logger_js_namespaceObject = require("./logger.js");
const logger = (0, external_logger_js_namespaceObject.createLogger)('[Stylesheet Loader]');
const getStylesheetPreparer = (context)=>{
    if ((0, core_namespaceObject.isFunction)(context)) return context;
    return context?.config?.prepare_external_dependency_stylesheet;
};
const prepareStylesheet = (document, innerText, context)=>{
    let stylesheet = document.createElement('style');
    stylesheet.innerText = innerText;
    const prepareExternalDependencyStylesheet = getStylesheetPreparer(context);
    if (prepareExternalDependencyStylesheet) stylesheet = prepareExternalDependencyStylesheet(stylesheet);
    if (!stylesheet) {
        logger.error('prepare_external_dependency_stylesheet returned null');
        return null;
    }
    return stylesheet;
};
exports.prepareStylesheet = __webpack_exports__.prepareStylesheet;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "prepareStylesheet"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
