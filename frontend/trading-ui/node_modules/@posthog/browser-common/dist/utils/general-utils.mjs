import { hasOwnProperty as core_hasOwnProperty, isArray, isFormData, isNullish, isNumber, isString } from "@posthog/core";
import { logger } from "./logger.mjs";
function find(value, predicate) {
    for(let i = 0; i < value.length; i++)if (predicate(value[i])) return value[i];
}
function eachArray(obj, iterator) {
    if (isArray(obj)) obj.forEach(iterator);
}
function each(obj, iterator) {
    if (isNullish(obj)) return;
    if (isArray(obj)) return void obj.forEach(iterator);
    if (isFormData(obj)) return void obj.forEach((val, key)=>iterator(val, key));
    for(const key in obj)if (core_hasOwnProperty.call(obj, key)) iterator(obj[key], key);
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
            logger.critical('Implementation error. Please turn on debug mode and open a ticket on https://app.posthog.com/home#panel=support%3Asupport%3A.');
            logger.critical(e);
        }
    };
};
const safewrapClass = function(klass, functions) {
    for(let i = 0; i < functions.length; i++)klass.prototype[functions[i]] = safewrap(klass.prototype[functions[i]]);
};
const stripEmptyProperties = function(p) {
    const ret = {};
    each(p, function(v, k) {
        if (isString(v) && v.length > 0 || isNumber(v)) ret[k] = v;
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
        if (isArray(value)) {
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
        if (isString(value)) return value.slice(0, maxStringLength);
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
    if (!isString(hostname)) return false;
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
    const hasNewField = newField in config && !isNullish(config[newField]);
    const hasOldField = oldField in config && !isNullish(config[oldField]);
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
export { _copyAndTruncateStrings, addEventListener, each, eachArray, entries, extend, find, isCrossDomainCookie, isToolbarInstance, migrateConfigField, safewrap, safewrapClass, stripEmptyProperties, trySafe };
