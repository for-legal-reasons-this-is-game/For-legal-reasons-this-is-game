"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.FlushedSizeTracker = void 0;
var core_1 = require("@posthog/core");
var constants_1 = require("../../../constants");
function isFlushedSize(value) {
    return (0, core_1.isObject)(value) && (0, core_1.isString)(value.sessionId) && (0, core_1.isNumber)(value.size);
}
var FlushedSizeTracker = /** @class */ (function () {
    function FlushedSizeTracker(posthog) {
        if (!posthog.persistence) {
            throw new Error('it is not valid to not have persistence and be this far into setting up the application');
        }
        this._getProperty = posthog.get_property.bind(posthog);
        this._setProperty = posthog.persistence.set_property.bind(posthog.persistence);
    }
    FlushedSizeTracker.prototype.trackSize = function (sessionId, size) {
        this._setProperty(constants_1.SESSION_RECORDING_FLUSHED_SIZE, {
            sessionId: sessionId,
            size: this.currentTrackedSize(sessionId) + size,
        });
    };
    FlushedSizeTracker.prototype.currentTrackedSize = function (sessionId) {
        var stored = this._getProperty(constants_1.SESSION_RECORDING_FLUSHED_SIZE);
        return isFlushedSize(stored) && stored.sessionId === sessionId ? stored.size : 0;
    };
    return FlushedSizeTracker;
}());
exports.FlushedSizeTracker = FlushedSizeTracker;
//# sourceMappingURL=flushed-size-tracker.js.map