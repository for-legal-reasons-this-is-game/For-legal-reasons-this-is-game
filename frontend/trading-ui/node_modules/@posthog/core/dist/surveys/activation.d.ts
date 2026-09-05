import { Survey } from '../types';
type SurveyForRepeatActivation = Pick<Survey, 'schedule'> & {
    conditions?: {
        events?: {
            repeatedActivation?: boolean;
            values?: {
                name: string;
            }[];
        } | null;
    } | null;
};
export declare function doesSurveyActivateByEvent(survey: SurveyForRepeatActivation): boolean;
/**
 * Platform-independent part of "can this survey show again after being seen":
 * event-repeated activation or an 'always' schedule. SDKs may OR in
 * platform-specific state (e.g. the web SDK's in-progress partial responses).
 */
export declare function canSurveyActivateRepeatedly(survey: SurveyForRepeatActivation): boolean;
type SurveyForIterationCheck = Pick<Survey, 'schedule' | 'current_iteration'>;
/**
 * True when a survey is meant to come back around on its own: a recurring schedule, or an
 * iteration already under way. Stored display state is keyed by iteration
 * (see `getSurveyIterationKey`), so for these surveys it rolls over and stops being a
 * dependable "already answered" record; other surveys keep one stable key for good.
 */
export declare function isSurveyIterationBased(survey: SurveyForIterationCheck): boolean;
export {};
//# sourceMappingURL=activation.d.ts.map