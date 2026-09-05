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
    find: ()=>find,
    migrateConfigField: ()=>migrateConfigField,
    each: ()=>each,
    entries: ()=>entries,
    safewrap: ()=>safewrap,
    eachArray: ()=>eachArray,
    safewrapClass: ()=>safewrapClass,
    isCrossDomainCookie: ()=>isCrossDomainCookie,
    _copyAndTruncateStrings: ()=>_copyAndTruncateStrings,
    addEventListener: ()=>addEventListener,
    extend: ()=>extend,
    stripEmptyProperties: ()=>stripEmptyProperties,
    isToolbarInstance: ()=>isToolbarInstance,
    trySafe: ()=>trySafe
});
const core_namespaceObject = require("@posthog/core");
const external_logger_js_namespaceObject = require("./logger.js");
function find(value, predicate) {
    for(let i = 0; i < value.length; i++)if (predicate(value[i])) return value[i];
}
function eachArray(obj, iterator) {
    if ((0, core_namespaceObject.isArray)(obj)) obj.forEach(iterator);
}
function each(obj, iterator) {
    if ((0, core_namespaceObject.isNullish)(obj)) return;
    if ((0, core_namespaceObject.isArray)(obj)) return void obj.forEach(iterator);
    if ((0, core_namespaceObject.isFormData)(obj)) return void obj.forEach((val, key)=>iterator(val, key));
    for(const key in obj)if (core_namespaceObject.hasOwnProperty.call(obj, key)) iterator(obj[key], key);
}
const extend = function(obj, ...args) {
    for (const source of args)for(const prop in source)if (void 0 !== source[prop]) obj[prop] = source[prop];
    return obj;
};
function entries(obj) {
    const ownProps = Object.keys(obj);
    let i = ownProps.length;
    const resArray = new Array(i);
    while(i--)resArray[i] = [
        ownProps[i],
        obj[ownProps[i]]
    ];
    return resArray;
}
const trySafe = function(fn) {
    try {
        return fn();
    } catch  {
        return;
    }
};
const safewrap = function(f) {
    return function(...args) {
        try {
            return f.apply(this, args);
        } catch (e) {
            external_logger_js_namespaceObject.logger.critical('Implementation error. Please turn on debug mode and open a ticket on https://app.posthog.com/home#panel=support%3Asupport%3A.');
            external_logger_js_namespaceObject.logger.critical(e);
        }
    };
};
const safewrapClass = function(klass, functions) {
    for(let i = 0; i < functions.length; i++)klass.prototype[functions[i]] = safewrap(klass.prototype[functions[i]]);
};
const stripEmptyProperties = function(p) {
    const ret = {};
    each(p, function(v, k) {
        if ((0, core_namespaceObject.isString)(v) && v.length > 0 || (0, core_namespaceObject.isNumber)(v)) ret[k] = v;
    });
    return ret;
};
function deepCircularCopy(value, customizer) {
    const COPY_IN_PROGRESS_SET = new Set();
    function internalDeepCircularCopy(value, key) {
        if (value !== Object(value)) return customizer ? customizer(value, key) : value;
        if (COPY_IN_PROGRESS_SET.has(value)) return;
        COPY_IN_PROGRESS_SET.add(value);
        let result;
        if ((0, core_namespaceObject.isArray)(value)) {
            result = [];
            eachArray(value, (it)=>{
                result.push(internalDeepCircularCopy(it));
            });
        } else {
            const copy = {};
            each(value, (val, key)=>{
                if (!COPY_IN_PROGRESS_SET.has(val)) copy[key] = internalDeepCircularCopy(val, key);
            });
            result = copy;
        }
        return result;
    }
    return internalDeepCircularCopy(value);
}
function _copyAndTruncateStrings(object, maxStringLength) {
    return deepCircularCopy(object, (value)=>{
        if ((0, core_namespaceObject.isString)(value)) return value.slice(0, maxStringLength);
        return value;
    });
}
const EXCLUDED_FROM_CROSS_SUBDOMAIN_COOKIE = [
    'herokuapp.com',
    'vercel.app',
    'netlify.app'
];
function isCrossDomainCookie(documentLocation) {
    const hostname = documentLocation?.hostname;
    if (!(0, core_namespaceObject.isString)(hostname)) return false;
    const lastTwoParts = hostname.split('.').slice(-2).join('.');
    for (const excluded of EXCLUDED_FROM_CROSS_SUBDOMAIN_COOKIE)if (lastTwoParts === excluded) return false;
    return true;
}
function addEventListener(element, event, callback, options) {
    const { capture = false, passive = true } = options ?? {};
    element?.addEventListener(event, callback, {
        capture,
        passive
    });
}
function migrateConfigField(config, newField, oldField, defaultValue, loggerInstance) {
    const hasNewField = newField in config && !(0, core_namespaceObject.isNullish)(config[newField]);
    const hasOldField = oldField in config && !(0, core_namespaceObject.isNullish)(config[oldField]);
    if (hasNewField) return config[newField];
    if (hasOldField) {
        if (loggerInstance) loggerInstance.warn(`Config field '${oldField}' is deprecated. Please use '${newField}' instead. The old field will be removed in a future major version.`);
        return config[oldField];
    }
    return defaultValue;
}
const TOOLBAR_INTERNAL_INSTANCE_NAME = 'ph_toolbar_internal';
function isToolbarInstance(config) {
    return config.name === TOOLBAR_INTERNAL_INSTANCE_NAME;
}
exports._copyAndTruncateStrings = __webpack_exports__._copyAndTruncateStrings;
exports.addEventListener = __webpack_exports__.addEventListener;
exports.each = __webpack_exports__.each;
exports.eachArray = __webpack_exports__.eachArray;
exports.entries = __webpack_exports__.entries;
exports.extend = __webpack_exports__.extend;
exports.find = __webpack_exports__.find;
exports.isCrossDomainCookie = __webpack_exports__.isCrossDomainCookie;
exports.isToolbarInstance = __webpack_exports__.isToolbarInstance;
exports.migrateConfigField = __webpack_exports__.migrateConfigField;
exports.safewrap = __webpack_exports__.safewrap;
exports.safewrapClass = __webpack_exports__.safewrapClass;
exports.stripEmptyProperties = __webpack_exports__.stripEmptyProperties;
exports.trySafe = __webpack_exports__.trySafe;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "_copyAndTruncateStrings",
    "addEventListener",
    "each",
    "eachArray",
    "entries",
    "extend",
    "find",
    "isCrossDomainCookie",
    "isToolbarInstance",
    "migrateConfigField",
    "safewrap",
    "safewrapClass",
    "stripEmptyProperties",
    "trySafe"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
