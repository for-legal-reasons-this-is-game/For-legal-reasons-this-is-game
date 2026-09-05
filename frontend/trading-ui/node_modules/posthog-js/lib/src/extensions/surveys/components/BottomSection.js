"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.BottomSection = BottomSection;
var jsx_runtime_1 = require("preact/jsx-runtime");
var globals_1 = require("@posthog/browser-common/utils/globals");
var hooks_1 = require("preact/hooks");
var surveys_extension_utils_1 = require("../surveys-extension-utils");
var PostHogLogo_1 = require("./PostHogLogo");
function BottomSection(_a) {
    var text = _a.text, submitDisabled = _a.submitDisabled, appearance = _a.appearance, onSubmit = _a.onSubmit, link = _a.link, onPreviewSubmit = _a.onPreviewSubmit, skipSubmitButton = _a.skipSubmitButton, canGoBack = _a.canGoBack, onBack = _a.onBack;
    var isPreviewMode = (0, hooks_1.useContext)(surveys_extension_utils_1.SurveyContext).isPreviewMode;
    var showBackButton = !!canGoBack && !!onBack;
    var submitButton = !skipSubmitButton && ((0, jsx_runtime_1.jsx)("button", { className: "form-submit", disabled: submitDisabled, "aria-label": "Submit survey", type: "button", onClick: function () {
            if (link) {
                globals_1.window === null || globals_1.window === void 0 ? void 0 : globals_1.window.open(link);
            }
            if (isPreviewMode) {
                onPreviewSubmit === null || onPreviewSubmit === void 0 ? void 0 : onPreviewSubmit();
            }
            else {
                onSubmit();
            }
        }, children: text }));
    return ((0, jsx_runtime_1.jsxs)("div", { className: "bottom-section", children: [showBackButton ? ((0, jsx_runtime_1.jsxs)("div", { className: "form-buttons form-buttons-with-back", children: [(0, jsx_runtime_1.jsx)("button", { className: "form-back", type: "button", "aria-label": "Go to previous question", onClick: onBack, children: appearance.backButtonText || 'Back' }), submitButton] })) : (submitButton), !appearance.whiteLabel && (0, jsx_runtime_1.jsx)(PostHogLogo_1.PostHogLogo, { urlParams: { utm_source: 'survey-footer' } })] }));
}
//# sourceMappingURL=BottomSection.js.map