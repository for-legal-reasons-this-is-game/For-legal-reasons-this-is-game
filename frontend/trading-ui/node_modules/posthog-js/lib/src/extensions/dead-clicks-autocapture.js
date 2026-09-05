"use strict";
var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.DeadClicksAutocapture = exports.isDeadClicksEnabledForAutocapture = exports.isDeadClicksEnabledForHeatmaps = void 0;
var constants_1 = require("../constants");
var core_1 = require("@posthog/core");
var globals_1 = require("@posthog/browser-common/utils/globals");
var globals_2 = require("../utils/globals");
var logger_1 = require("@posthog/browser-common/utils/logger");
var logger = (0, logger_1.createLogger)('[Dead Clicks]');
var isDeadClicksEnabledForHeatmaps = function () {
    return true;
};
exports.isDeadClicksEnabledForHeatmaps = isDeadClicksEnabledForHeatmaps;
var isDeadClicksEnabledForAutocapture = function (instance) {
    var _a;
    var isRemoteEnabled = !!((_a = instance.instance.persistence) === null || _a === void 0 ? void 0 : _a.get_property(constants_1.DEAD_CLICKS_ENABLED_SERVER_SIDE));
    var clientConfig = instance.instance.config.capture_dead_clicks;
    if ((0, core_1.isBoolean)(clientConfig)) {
        return clientConfig;
    }
    if ((0, core_1.isObject)(clientConfig)) {
        return true;
    }
    return isRemoteEnabled;
};
exports.isDeadClicksEnabledForAutocapture = isDeadClicksEnabledForAutocapture;
var DeadClicksAutocapture = /** @class */ (function () {
    function DeadClicksAutocapture(instance, isEnabled, onCapture) {
        this.instance = instance;
        this.isEnabled = isEnabled;
        this.onCapture = onCapture;
        this.startIfEnabledOrStop();
    }
    Object.defineProperty(DeadClicksAutocapture.prototype, "lazyLoadedDeadClicksAutocapture", {
        get: function () {
            return this._lazyLoadedDeadClicksAutocapture;
        },
        enumerable: false,
        configurable: true
    });
    DeadClicksAutocapture.prototype.onRemoteConfig = function (result) {
        var _a;
        if (!result.ok) {
            // Failure behaves like a response without a captureDeadClicks key.
            return;
        }
        var response = result.config;
        if (!('captureDeadClicks' in response)) {
            return;
        }
        if (this.instance.persistence) {
            this.instance.persistence.register((_a = {},
                _a[constants_1.DEAD_CLICKS_ENABLED_SERVER_SIDE] = response.captureDeadClicks,
                _a));
        }
        this.startIfEnabledOrStop();
    };
    DeadClicksAutocapture.prototype.startIfEnabledOrStop = function () {
        var _this = this;
        if (this.isEnabled(this)) {
            this._loadScript(function () {
                _this._start();
            });
        }
        else {
            this.stop();
        }
    };
    DeadClicksAutocapture.prototype._loadScript = function (cb) {
        var _a, _b, _c;
        if ((_a = globals_2.assignableWindow.__PosthogExtensions__) === null || _a === void 0 ? void 0 : _a.initDeadClicksAutocapture) {
            // already loaded
            cb();
            return;
        }
        (_c = (_b = globals_2.assignableWindow.__PosthogExtensions__) === null || _b === void 0 ? void 0 : _b.loadExternalDependency) === null || _c === void 0 ? void 0 : _c.call(_b, this.instance, 'dead-clicks-autocapture', function (err) {
            if (err) {
                logger.error('failed to load script', err);
                return;
            }
            cb();
        });
    };
    DeadClicksAutocapture.prototype._start = function () {
        var _a;
        if (!globals_1.document) {
            logger.error('`document` not found. Cannot start.');
            return;
        }
        if (!this._lazyLoadedDeadClicksAutocapture &&
            ((_a = globals_2.assignableWindow.__PosthogExtensions__) === null || _a === void 0 ? void 0 : _a.initDeadClicksAutocapture)) {
            // copied so the writes below can't leak into the user's config object, which is
            // shared between the autocapture and heatmaps instances of this extension
            var config = (0, core_1.isObject)(this.instance.config.capture_dead_clicks)
                ? __assign({}, this.instance.config.capture_dead_clicks) : {};
            config.__onCapture = this.onCapture;
            if (this.onCapture) {
                // an external consumer (heatmaps) only plots click points, so it never observes swipes
                config.capture_dead_swipes = false;
            }
            this._lazyLoadedDeadClicksAutocapture = globals_2.assignableWindow.__PosthogExtensions__.initDeadClicksAutocapture(this.instance, config);
            this._lazyLoadedDeadClicksAutocapture.start(globals_1.document);
            logger.info("starting...");
        }
    };
    DeadClicksAutocapture.prototype.stop = function () {
        if (this._lazyLoadedDeadClicksAutocapture) {
            this._lazyLoadedDeadClicksAutocapture.stop();
            this._lazyLoadedDeadClicksAutocapture = undefined;
            logger.info("stopping...");
        }
    };
    return DeadClicksAutocapture;
}());
exports.DeadClicksAutocapture = DeadClicksAutocapture;
//# sourceMappingURL=dead-clicks-autocapture.js.map