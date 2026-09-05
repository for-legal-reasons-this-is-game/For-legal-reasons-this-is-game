"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.assignableWindow = exports.XMLHttpRequest = exports.window = exports.userAgent = exports.navigator = exports.location = exports.isBrowserOnline = exports.document = exports.CompressionStream = exports.AbortController = void 0;
var globals_1 = require("@posthog/browser-common/utils/globals");
var globalObject = typeof globalThis !== 'undefined' ? globalThis : globals_1.window;
// React Native polyfills retained for browser-v1 compatibility.
if (globalObject && typeof self === 'undefined') {
    ;
    globalObject.self = globalObject;
}
if (globalObject && typeof File === 'undefined') {
    ;
    globalObject.File = function () { };
}
var globals_2 = require("@posthog/browser-common/utils/globals");
Object.defineProperty(exports, "AbortController", { enumerable: true, get: function () { return globals_2.AbortController; } });
Object.defineProperty(exports, "CompressionStream", { enumerable: true, get: function () { return globals_2.CompressionStream; } });
Object.defineProperty(exports, "document", { enumerable: true, get: function () { return globals_2.document; } });
Object.defineProperty(exports, "fetch", { enumerable: true, get: function () { return globals_2.fetch; } });
Object.defineProperty(exports, "isBrowserOnline", { enumerable: true, get: function () { return globals_2.isBrowserOnline; } });
Object.defineProperty(exports, "location", { enumerable: true, get: function () { return globals_2.location; } });
Object.defineProperty(exports, "navigator", { enumerable: true, get: function () { return globals_2.navigator; } });
Object.defineProperty(exports, "userAgent", { enumerable: true, get: function () { return globals_2.userAgent; } });
Object.defineProperty(exports, "window", { enumerable: true, get: function () { return globals_2.window; } });
Object.defineProperty(exports, "XMLHttpRequest", { enumerable: true, get: function () { return globals_2.XMLHttpRequest; } });
exports.assignableWindow = globals_1.window !== null && globals_1.window !== void 0 ? globals_1.window : {};
//# sourceMappingURL=globals.js.map