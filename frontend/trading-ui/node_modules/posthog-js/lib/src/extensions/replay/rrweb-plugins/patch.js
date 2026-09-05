"use strict";
// Portions of this file are derived from getsentry/sentry-javascript
// Copyright (c) 2012 Functional Software, Inc. dba Sentry
// Licensed under the MIT License: https://github.com/getsentry/sentry-javascript/blob/develop/LICENSE
Object.defineProperty(exports, "__esModule", { value: true });
exports.patch = patch;
// import { patch } from 'rrweb/typings/utils'
// copied from https://github.com/rrweb-io/rrweb/blob/8aea5b00a4dfe5a6f59bd2ae72bb624f45e51e81/packages/rrweb/src/utils.ts#L129
var core_1 = require("@posthog/core");
var noop = function () {
    //
};
function patch(source, name, replacement) {
    try {
        if (!(name in source)) {
            return noop;
        }
        var original = source[name];
        var layer_1 = {
            next: original,
        };
        // The wrapper receives this stable delegate instead of `original`, so the
        // function it actually calls can be re-pointed when a lower layer is removed.
        var callNext = function () {
            var args = [];
            for (var _i = 0; _i < arguments.length; _i++) {
                args[_i] = arguments[_i];
            }
            return layer_1.next.apply(this, args);
        };
        var wrapped_1 = replacement(callNext);
        // Make sure it's a function first, as we need to attach an empty prototype for `defineProperties` to work
        // otherwise it'll throw "TypeError: Object.defineProperties called on non-object"
        if ((0, core_1.isFunction)(wrapped_1)) {
            wrapped_1.prototype = wrapped_1.prototype || {};
            Object.defineProperties(wrapped_1, {
                __posthog_wrapped__: {
                    enumerable: false,
                    value: true,
                },
                __posthog_layer__: {
                    enumerable: false,
                    value: layer_1,
                },
            });
        }
        source[name] = wrapped_1;
        return function () {
            // If we're still on top, hand back whatever we currently delegate to
            // (lower layers may already have been removed, so this is not necessarily
            // the `original` we captured at install time).
            if (source[name] === wrapped_1) {
                source[name] = layer_1.next;
                return;
            }
            // Otherwise newer wrappers sit on top of us. Find the posthog layer directly
            // above us and re-point it past us, removing our wrapper from the call path
            // without disturbing the newer wrappers.
            var current = source[name];
            while ((0, core_1.isFunction)(current) && current.__posthog_layer__) {
                var currentLayer = current.__posthog_layer__;
                if (currentLayer.next === wrapped_1) {
                    currentLayer.next = layer_1.next;
                    return;
                }
                current = currentLayer.next;
            }
            // If we get here we're buried under a non-posthog wrapper that closed over
            // us directly, or we've already been removed / replaced wholesale. There's
            // nothing safe to do, so leave the chain untouched.
        };
    }
    catch (_a) {
        return noop;
        // This can throw when multiple instrumentation layers try to wrap the same global object,
        // such as XMLHttpRequest, and redefine the same non-configurable wrapper marker.
    }
}
//# sourceMappingURL=patch.js.map