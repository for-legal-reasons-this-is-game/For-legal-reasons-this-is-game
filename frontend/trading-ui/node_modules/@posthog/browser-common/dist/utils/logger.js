"use strict";
var __webpack_require__ = {};
(()=>{
    __webpack_require__.n = (module)=>{
        var getter = module && module.__esModule ? ()=>module['default'] : ()=>module;
        __webpack_require__.d(getter, {
            a: getter
        });
        return getter;
    };
})();
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
    logger: ()=>logger_logger,
    createLogger: ()=>createLogger
});
const external_config_js_namespaceObject = require("../config.js");
var external_config_js_default = /*#__PURE__*/ __webpack_require__.n(external_config_js_namespaceObject);
const core_namespaceObject = require("@posthog/core");
const external_globals_js_namespaceObject = require("./globals.js");
const _createLogger = (prefix, { debugEnabled } = {})=>{
    const logger = {
        _log: (level, ...args)=>{
            if (external_globals_js_namespaceObject.window && (external_config_js_default().DEBUG || external_globals_js_namespaceObject.window.POSTHOG_DEBUG || debugEnabled) && !(0, core_namespaceObject.isUndefined)(external_globals_js_namespaceObject.window.console) && external_globals_js_namespaceObject.window.console) {
                const consoleLog = '__rrweb_original__' in external_globals_js_namespaceObject.window.console[level] ? external_globals_js_namespaceObject.window.console[level]['__rrweb_original__'] : external_globals_js_namespaceObject.window.console[level];
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
exports.createLogger = __webpack_exports__.createLogger;
exports.logger = __webpack_exports__.logger;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "createLogger",
    "logger"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
