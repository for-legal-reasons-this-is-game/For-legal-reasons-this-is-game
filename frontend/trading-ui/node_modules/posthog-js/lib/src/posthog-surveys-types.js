"use strict";
/**
 * Having Survey types in types.ts was confusing tsc
 * and generating an invalid module.d.ts
 * See https://github.com/PostHog/posthog-js/issues/698
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.SurveyWidgetType = exports.SurveyType = exports.SurveyTabPosition = exports.SurveySchedule = exports.SurveyQuestionType = exports.SurveyQuestionBranchingType = exports.SurveyPosition = exports.SurveyEventType = exports.SurveyEventProperties = exports.SurveyEventName = exports.DisplaySurveyType = void 0;
var browser_common_1 = require("@posthog/browser-common");
Object.defineProperty(exports, "DisplaySurveyType", { enumerable: true, get: function () { return browser_common_1.DisplaySurveyType; } });
Object.defineProperty(exports, "SurveyEventName", { enumerable: true, get: function () { return browser_common_1.SurveyEventName; } });
Object.defineProperty(exports, "SurveyEventProperties", { enumerable: true, get: function () { return browser_common_1.SurveyEventProperties; } });
Object.defineProperty(exports, "SurveyEventType", { enumerable: true, get: function () { return browser_common_1.SurveyEventType; } });
Object.defineProperty(exports, "SurveyPosition", { enumerable: true, get: function () { return browser_common_1.SurveyPosition; } });
Object.defineProperty(exports, "SurveyQuestionBranchingType", { enumerable: true, get: function () { return browser_common_1.SurveyQuestionBranchingType; } });
Object.defineProperty(exports, "SurveyQuestionType", { enumerable: true, get: function () { return browser_common_1.SurveyQuestionType; } });
Object.defineProperty(exports, "SurveySchedule", { enumerable: true, get: function () { return browser_common_1.SurveySchedule; } });
Object.defineProperty(exports, "SurveyTabPosition", { enumerable: true, get: function () { return browser_common_1.SurveyTabPosition; } });
Object.defineProperty(exports, "SurveyType", { enumerable: true, get: function () { return browser_common_1.SurveyType; } });
Object.defineProperty(exports, "SurveyWidgetType", { enumerable: true, get: function () { return browser_common_1.SurveyWidgetType; } });
//# sourceMappingURL=posthog-surveys-types.js.map