export type StylesheetPreparer = (stylesheet: HTMLStyleElement) => HTMLStyleElement | null;
export type StylesheetPreparationContext = StylesheetPreparer | {
    config?: {
        prepare_external_dependency_stylesheet?: StylesheetPreparer;
    };
};
export declare const prepareStylesheet: (document: Document, innerText: string, context?: StylesheetPreparationContext) => HTMLStyleElement | null;
