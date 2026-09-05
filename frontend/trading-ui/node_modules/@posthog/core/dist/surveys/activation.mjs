import { SurveySchedule } from "../types.mjs";
function doesSurveyActivateByEvent(survey) {
    return !!survey.conditions?.events?.values?.length;
}
function canSurveyActivateRepeatedly(survey) {
    return doesSurveyActivateByEvent(survey) && !!survey.conditions?.events?.repeatedActivation || survey.schedule === SurveySchedule.Always;
}
function isSurveyIterationBased(survey) {
    return survey.schedule === SurveySchedule.Recurring || (survey.current_iteration ?? 0) > 0;
}
export { canSurveyActivateRepeatedly, doesSurveyActivateByEvent, isSurveyIterationBased };
