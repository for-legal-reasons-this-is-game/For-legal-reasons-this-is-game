import config from "../config.mjs";
import { isUndefined } from "@posthog/core";
import { window as external_globals_mjs_window } from "./globals.mjs";
const _createLogger = (prefix, { debugEnabled } = {})=>{
    const logger = {
        _log: (level, ...args)=>{
            if (external_globals_mjs_window && (config.DEBUG || external_globals_mjs_window.POSTHOG_DEBUG || debugEnabled) && !isUndefined(external_globals_mjs_window.console) && external_globals_mjs_window.console) {
                const consoleLog = '__rrweb_original__' in external_globals_mjs_window.console[level] ? external_globals_mjs_window.console[level]['__rrweb_original__'] : external_globals_mjs_window.console[level];
                consoleLog(prefix, ...args);
            }
        },
        debug: (...args)=>{
            logger._log('debug', ...args);
        },
        info: (...args)=>{
            logger._log('log', ...args);
        },
        warn: (...args)=>{
            logger._log('warn', ...args);
        },
        error: (...args)=>{
            logger._log('error', ...args);
        },
        critical: (...args)=>{
            console.error(prefix, ...args);
        },
        uninitializedWarning: (methodName)=>{
            logger.error(`You must initialize PostHog before calling ${methodName}`);
        },
        createLogger: (additionalPrefix, options)=>_createLogger(`${prefix} ${additionalPrefix}`, options)
    };
    return logger;
};
const logger_logger = _createLogger('[PostHog.js]');
const createLogger = logger_logger.createLogger;
export { createLogger, logger_logger as logger };
