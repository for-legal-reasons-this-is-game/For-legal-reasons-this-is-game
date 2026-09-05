import type { Logger } from '@posthog/types';
export type CreateLoggerOptions = {
    debugEnabled?: boolean;
};
export type PosthogJsLogger = Omit<Logger, 'createLogger' | 'debug' | 'info' | 'warn' | 'error' | 'trace' | 'fatal'> & {
    _log: (level: 'debug' | 'log' | 'warn' | 'error', ...args: any[]) => void;
    debug: (...args: any[]) => void;
    info: (...args: any[]) => void;
    warn: (...args: any[]) => void;
    error: (...args: any[]) => void;
    critical: (...args: any[]) => void;
    uninitializedWarning: (methodName: string) => void;
    createLogger: (prefix: string, options?: CreateLoggerOptions) => PosthogJsLogger;
};
export declare const logger: PosthogJsLogger;
export declare const createLogger: (prefix: string, options?: CreateLoggerOptions) => PosthogJsLogger;
