import { TOOLBAR_CONTAINER_CLASS, TOOLBAR_ID } from "../constants.mjs";
const NODE_TYPE_ELEMENT = 1;
const NODE_TYPE_TEXT = 3;
const NODE_TYPE_DOCUMENT_FRAGMENT = 11;
function isElementInToolbar(el) {
    if (el instanceof Element) return el.id === TOOLBAR_ID || !!el.closest?.('.' + TOOLBAR_CONTAINER_CLASS);
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
export { isElementInToolbar, isElementNode, isShadowRoot, isTag, isTextNode };
