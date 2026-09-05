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
export { DisplaySurveyType, SurveyEventName, SurveyEventProperties, SurveyEventType, SurveyPosition, SurveyQuestionBranchingType, SurveyQuestionType, SurveySchedule, SurveyTabPosition, SurveyType, SurveyWidgetType };
