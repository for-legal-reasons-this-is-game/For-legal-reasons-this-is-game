"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
var config_1 = __importDefault(require("@posthog/browser-common/config"));
var package_json_1 = __importDefault(require("../package.json"));
// overridden in posthog-core,
// e.g.     Config.DEBUG = Config.DEBUG || instance.config.debug
config_1.default.DEBUG = false;
config_1.default.LIB_VERSION = package_json_1.default.version;
config_1.default.LIB_NAME = 'web';
exports.default = config_1.default;
//# sourceMappingURL=config.js.map