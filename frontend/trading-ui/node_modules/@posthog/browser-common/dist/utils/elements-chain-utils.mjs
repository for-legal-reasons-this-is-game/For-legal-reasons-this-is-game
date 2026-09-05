import { isNullish } from "@posthog/core";
function extractHref(elementsChain) {
    const match = elementsChain.match(/(?::|")href="(.*?)"/);
    return match ? match[1] : '';
}
function extractTexts(elementsChain) {
    const texts = [];
    const regex = /(?::|")text="(.*?)"/g;
    let match;
    while(!isNullish(match = regex.exec(elementsChain)))if (!texts.includes(match[1])) texts.push(match[1]);
    return texts;
}
function matchString(value, pattern, matching) {
    if (isNullish(value)) return false;
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
export { extractHref, extractTexts, matchString, matchTexts };
