"use strict";
// Portions of this file are derived from getsentry/sentry-javascript
// Copyright (c) 2012 Functional Software, Inc. dba Sentry
// Licensed under the MIT License: https://github.com/getsentry/sentry-javascript/blob/develop/LICENSE
Object.defineProperty(exports, "__esModule", { value: true });
exports.severityLevels = exports.Compression = void 0;
var browser_common_1 = require("@posthog/browser-common");
Object.defineProperty(exports, "Compression", { enumerable: true, get: function () { return browser_common_1.Compression; } });
// levels originally copied from Sentry to work with the sentry integration
// and to avoid relying on a frequently changing @sentry/types dependency
// but provided as an array of literal types, so we can constrain the level below
exports.severityLevels = ['fatal', 'error', 'warning', 'log', 'info', 'debug'];
//# sourceMappingURL=types.js.map