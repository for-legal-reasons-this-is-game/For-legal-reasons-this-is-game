"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.HistoryAutocapture = void 0;
var constants_1 = require("../constants");
var globals_1 = require("@posthog/browser-common/utils/globals");
var general_utils_1 = require("@posthog/browser-common/utils/general-utils");
var logger_1 = require("@posthog/browser-common/utils/logger");
var patch_1 = require("./replay/rrweb-plugins/patch");
/**
 * This class is used to capture pageview events when the user navigates using the history API (pushState, replaceState)
 * and when the user navigates using the browser's back/forward buttons.
 *
 * The behavior is controlled by the `capture_pageview` configuration option:
 * - When set to `'history_change'`, this class will capture pageviews on history API changes
 */
var HistoryAutocapture = /** @class */ (function () {
    function HistoryAutocapture(instance) {
        var _a;
        this._instance = instance;
        this._lastPathname = ((_a = globals_1.window === null || globals_1.window === void 0 ? void 0 : globals_1.window.location) === null || _a === void 0 ? void 0 : _a.pathname) || '';
    }
    HistoryAutocapture.prototype.initialize = function () {
        this.startIfEnabled();
    };
    Object.defineProperty(HistoryAutocapture.prototype, "isEnabled", {
        get: function () {
            return this._instance.config.capture_pageview === 'history_change';
        },
        enumerable: false,
        configurable: true
    });
    HistoryAutocapture.prototype.startIfEnabled = function () {
        if (this.isEnabled) {
            logger_1.logger.info('History API monitoring enabled, starting...');
            this.monitorHistoryChanges();
        }
    };
    HistoryAutocapture.prototype.stop = function () {
        if (this._popstateListener) {
            this._popstateListener();
        }
        this._popstateListener = undefined;
        logger_1.logger.info('History API monitoring stopped');
    };
    HistoryAutocapture.prototype.monitorHistoryChanges = function () {
        if (!globals_1.window || !globals_1.window.history) {
            return;
        }
        this._patchHistoryMethod('pushState');
        this._patchHistoryMethod('replaceState');
        this._setupPopstateListener();
    };
    HistoryAutocapture.prototype._patchHistoryMethod = function (method) {
        var _a;
        if (!globals_1.window || ((_a = globals_1.window.history[method]) === null || _a === void 0 ? void 0 : _a.__posthog_wrapped__)) {
            return;
        }
        // Old fashioned, we could also use arrow functions but I think the closure for a patch is more reliable
        var self = this;
        (0, patch_1.patch)(globals_1.window.history, method, function (originalMethod) {
            return function patchedHistoryMethod(state, title, url) {
                ;
                originalMethod.call(this, state, title, url);
                self._capturePageview(method);
            };
        });
    };
    HistoryAutocapture.prototype._capturePageview = function (navigationType) {
        var _a;
        try {
            var currentPathname = (_a = globals_1.window === null || globals_1.window === void 0 ? void 0 : globals_1.window.location) === null || _a === void 0 ? void 0 : _a.pathname;
            if (!currentPathname) {
                return;
            }
            // Only capture pageview if the pathname has changed and the feature is enabled
            if (currentPathname !== this._lastPathname && this.isEnabled) {
                this._instance.capture(constants_1.EVENT_PAGEVIEW, { navigation_type: navigationType });
            }
            this._lastPathname = currentPathname;
        }
        catch (error) {
            logger_1.logger.error("Error capturing ".concat(navigationType, " pageview"), error);
        }
    };
    HistoryAutocapture.prototype._setupPopstateListener = function () {
        var _this = this;
        if (this._popstateListener) {
            return;
        }
        var handler = function () {
            _this._capturePageview('popstate');
        };
        (0, general_utils_1.addEventListener)(globals_1.window, 'popstate', handler);
        this._popstateListener = function () {
            if (globals_1.window) {
                globals_1.window.removeEventListener('popstate', handler);
            }
        };
    };
    return HistoryAutocapture;
}());
exports.HistoryAutocapture = HistoryAutocapture;
//# sourceMappingURL=history-autocapture.js.map