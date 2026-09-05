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
    DisplaySurveyType: ()=>DisplaySurveyType,
    SurveyEventName: ()=>SurveyEventName,
    SurveyEventProperties: ()=>SurveyEventProperties,
    SurveyEventType: ()=>SurveyEventType,
    SurveyPosition: ()=>SurveyPosition,
    SurveyQuestionBranchingType: ()=>SurveyQuestionBranchingType,
    SurveyQuestionType: ()=>SurveyQuestionType,
    SurveySchedule: ()=>SurveySchedule,
    SurveyTabPosition: ()=>SurveyTabPosition,
    SurveyType: ()=>SurveyType,
    SurveyWidgetType: ()=>SurveyWidgetType
});
const SurveyEventType = {
    Activation: 'events',
    Cancellation: 'cancelEvents'
};
const SurveyWidgetType = {
    Button: 'button',
    Tab: 'tab',
    Selector: 'selector'
};
const SurveyPosition = {
    TopLeft: 'top_left',
    TopRight: 'top_right',
    TopCenter: 'top_center',
    MiddleLeft: 'middle_left',
    MiddleRight: 'middle_right',
    MiddleCenter: 'middle_center',
    Left: 'left',
    Center: 'center',
    Right: 'right',
    NextToTrigger: 'next_to_trigger'
};
const SurveyTabPosition = {
    Top: 'top',
    Left: 'left',
    Right: 'right',
    Bottom: 'bottom'
};
const SurveyType = {
    Popover: 'popover',
    API: 'api',
    Widget: 'widget',
    ExternalSurvey: 'external_survey'
};
const SurveyQuestionType = {
    Open: 'open',
    MultipleChoice: 'multiple_choice',
    SingleChoice: 'single_choice',
    Rating: 'rating',
    Link: 'link'
};
const SurveyQuestionBranchingType = {
    NextQuestion: 'next_question',
    End: 'end',
    ResponseBased: 'response_based',
    SpecificQuestion: 'specific_question'
};
const SurveySchedule = {
    Once: 'once',
    Recurring: 'recurring',
    Always: 'always'
};
const SurveyEventName = {
    SHOWN: 'survey shown',
    DISMISSED: 'survey dismissed',
    SENT: 'survey sent',
    ABANDONED: 'survey abandoned'
};
const SurveyEventProperties = {
    SURVEY_ID: '$survey_id',
    SURVEY_NAME: '$survey_name',
    SURVEY_RESPONSE: '$survey_response',
    SURVEY_ITERATION: '$survey_iteration',
    SURVEY_ITERATION_START_DATE: '$survey_iteration_start_date',
    SURVEY_PARTIALLY_COMPLETED: '$survey_partially_completed',
    SURVEY_SUBMISSION_ID: '$survey_submission_id',
    SURVEY_QUESTIONS: '$survey_questions',
    SURVEY_COMPLETED: '$survey_completed',
    PRODUCT_TOUR_ID: '$product_tour_id',
    SURVEY_LAST_SEEN_DATE: '$survey_last_seen_date',
    SURVEY_LANGUAGE: '$survey_language'
};
const DisplaySurveyType = {
    Popover: 'popover',
    Inline: 'inline'
};
exports.DisplaySurveyType = __webpack_exports__.DisplaySurveyType;
exports.SurveyEventName = __webpack_exports__.SurveyEventName;
exports.SurveyEventProperties = __webpack_exports__.SurveyEventProperties;
exports.SurveyEventType = __webpack_exports__.SurveyEventType;
exports.SurveyPosition = __webpack_exports__.SurveyPosition;
exports.SurveyQuestionBranchingType = __webpack_exports__.SurveyQuestionBranchingType;
exports.SurveyQuestionType = __webpack_exports__.SurveyQuestionType;
exports.SurveySchedule = __webpack_exports__.SurveySchedule;
exports.SurveyTabPosition = __webpack_exports__.SurveyTabPosition;
exports.SurveyType = __webpack_exports__.SurveyType;
exports.SurveyWidgetType = __webpack_exports__.SurveyWidgetType;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "DisplaySurveyType",
    "SurveyEventName",
    "SurveyEventProperties",
    "SurveyEventType",
    "SurveyPosition",
    "SurveyQuestionBranchingType",
    "SurveyQuestionType",
    "SurveySchedule",
    "SurveyTabPosition",
    "SurveyType",
    "SurveyWidgetType"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
