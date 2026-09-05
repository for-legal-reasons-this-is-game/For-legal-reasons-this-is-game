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
    matchPropertyFilters: ()=>matchPropertyFilters,
    propertyComparisons: ()=>propertyComparisons,
    getPersonPropertiesHash: ()=>getPersonPropertiesHash,
    matchTriggerPropertyFilters: ()=>matchTriggerPropertyFilters
});
const core_namespaceObject = require("@posthog/core");
const external_request_utils_js_namespaceObject = require("./request-utils.js");
const external_regex_utils_js_namespaceObject = require("./regex-utils.js");
function getPersonPropertiesHash(distinct_id, userPropertiesToSet, userPropertiesToSetOnce) {
    return (0, external_request_utils_js_namespaceObject.jsonStringify)({
        distinct_id,
        userPropertiesToSet,
        userPropertiesToSetOnce
    });
}
const propertyComparisons = {
    exact: (targets, values)=>values.some((value)=>targets.some((target)=>value === target)),
    is_not: (targets, values)=>values.every((value)=>targets.every((target)=>value !== target)),
    regex: (targets, values)=>values.some((value)=>targets.some((target)=>(0, external_regex_utils_js_namespaceObject.isMatchingRegex)(value, target))),
    not_regex: (targets, values)=>values.every((value)=>targets.every((target)=>!(0, external_regex_utils_js_namespaceObject.isMatchingRegex)(value, target))),
    icontains: (targets, values)=>values.map(toLowerCase).some((value)=>targets.map(toLowerCase).some((target)=>value.includes(target))),
    not_icontains: (targets, values)=>values.map(toLowerCase).every((value)=>targets.map(toLowerCase).every((target)=>!value.includes(target))),
    gt: (targets, values)=>values.some((value)=>{
            const numValue = parseFloat(value);
            return !isNaN(numValue) && targets.some((t)=>numValue > parseFloat(t));
        }),
    lt: (targets, values)=>values.some((value)=>{
            const numValue = parseFloat(value);
            return !isNaN(numValue) && targets.some((t)=>numValue < parseFloat(t));
        })
};
const toLowerCase = (v)=>v.toLowerCase();
const NEGATIVE_OPERATORS = new Set([
    'is_not',
    'not_icontains',
    'not_regex'
]);
function matchTriggerPropertyFilters(filters, eventProperties, personProperties) {
    if (!filters || 0 === filters.length) return true;
    return filters.every((filter)=>{
        const source = 'person' === filter.type ? personProperties : eventProperties;
        const propertyValue = source?.[filter.key];
        const operator = filter.operator || 'exact';
        if ((0, core_namespaceObject.isUndefined)(propertyValue) || (0, core_namespaceObject.isNull)(propertyValue)) return NEGATIVE_OPERATORS.has(operator);
        const comparisonFunction = propertyComparisons[operator];
        if (!comparisonFunction) return false;
        if ((0, core_namespaceObject.isUndefined)(filter.value) || (0, core_namespaceObject.isNull)(filter.value)) return false;
        const targetValues = (0, core_namespaceObject.isArray)(filter.value) ? filter.value.map(String) : [
            String(filter.value)
        ];
        const actualValues = (0, core_namespaceObject.isArray)(propertyValue) ? propertyValue.map(String) : [
            String(propertyValue)
        ];
        return comparisonFunction(targetValues, actualValues);
    });
}
function matchPropertyFilters(propertyFilters, eventProperties) {
    if (!propertyFilters) return true;
    return Object.entries(propertyFilters).every(([propertyName, filter])=>{
        const eventPropertyValue = eventProperties?.[propertyName];
        if ((0, core_namespaceObject.isUndefined)(eventPropertyValue) || (0, core_namespaceObject.isNull)(eventPropertyValue)) return false;
        const eventValues = [
            String(eventPropertyValue)
        ];
        const comparisonFunction = propertyComparisons[filter.operator];
        if (!comparisonFunction) return false;
        return comparisonFunction(filter.values, eventValues);
    });
}
exports.getPersonPropertiesHash = __webpack_exports__.getPersonPropertiesHash;
exports.matchPropertyFilters = __webpack_exports__.matchPropertyFilters;
exports.matchTriggerPropertyFilters = __webpack_exports__.matchTriggerPropertyFilters;
exports.propertyComparisons = __webpack_exports__.propertyComparisons;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "getPersonPropertiesHash",
    "matchPropertyFilters",
    "matchTriggerPropertyFilters",
    "propertyComparisons"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
