import { type PropertyMatchType } from './property-utils';
export declare function doesDeviceTypeMatch(deviceTypes?: string[], matchType?: PropertyMatchType): boolean;
export declare function hasPeriodPassed(periodDays?: number, lastSeenDate?: string | Date | null): boolean;
