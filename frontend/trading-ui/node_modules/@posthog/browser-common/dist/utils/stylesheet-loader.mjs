import { isFunction } from "@posthog/core";
import { createLogger } from "./logger.mjs";
const logger = createLogger('[Stylesheet Loader]');
const getStylesheetPreparer = (context)=>{
    if (isFunction(context)) return context;
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
export { prepareStylesheet };
