import { isArray, isNull, isUndefined } from "@posthog/core";
import { jsonStringify } from "./request-utils.mjs";
import { isMatchingRegex } from "./regex-utils.mjs";
function getPersonPropertiesHash(distinct_id, userPropertiesToSet, userPropertiesToSetOnce) {
    return jsonStringify({
        distinct_id,
        userPropertiesToSet,
        userPropertiesToSetOnce
    });
}
const propertyComparisons = {
    exact: (targets, values)=>values.some((value)=>targets.some((target)=>value === target)),
    is_not: (targets, values)=>values.every((value)=>targets.every((target)=>value !== target)),
    regex: (targets, values)=>values.some((value)=>targets.some((target)=>isMatchingRegex(value, target))),
    not_regex: (targets, values)=>values.every((value)=>targets.every((target)=>!isMatchingRegex(value, target))),
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
        if (isUndefined(propertyValue) || isNull(propertyValue)) return NEGATIVE_OPERATORS.has(operator);
        const comparisonFunction = propertyComparisons[operator];
        if (!comparisonFunction) return false;
        if (isUndefined(filter.value) || isNull(filter.value)) return false;
        const targetValues = isArray(filter.value) ? filter.value.map(String) : [
            String(filter.value)
        ];
        const actualValues = isArray(propertyValue) ? propertyValue.map(String) : [
            String(propertyValue)
        ];
        return comparisonFunction(targetValues, actualValues);
    });
}
function matchPropertyFilters(propertyFilters, eventProperties) {
    if (!propertyFilters) return true;
    return Object.entries(propertyFilters).every(([propertyName, filter])=>{
        const eventPropertyValue = eventProperties?.[propertyName];
        if (isUndefined(eventPropertyValue) || isNull(eventPropertyValue)) return false;
        const eventValues = [
            String(eventPropertyValue)
        ];
        const comparisonFunction = propertyComparisons[filter.operator];
        if (!comparisonFunction) return false;
        return comparisonFunction(filter.values, eventValues);
    });
}
export { getPersonPropertiesHash, matchPropertyFilters, matchTriggerPropertyFilters, propertyComparisons };
