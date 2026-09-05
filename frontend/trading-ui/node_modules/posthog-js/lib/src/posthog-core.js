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
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __generator = (this && this.__generator) || function (thisArg, body) {
    var _ = { label: 0, sent: function() { if (t[0] & 1) throw t[1]; return t[1]; }, trys: [], ops: [] }, f, y, t, g = Object.create((typeof Iterator === "function" ? Iterator : Object).prototype);
    return g.next = verb(0), g["throw"] = verb(1), g["return"] = verb(2), typeof Symbol === "function" && (g[Symbol.iterator] = function() { return this; }), g;
    function verb(n) { return function (v) { return step([n, v]); }; }
    function step(op) {
        if (f) throw new TypeError("Generator is already executing.");
        while (g && (g = 0, op[0] && (_ = 0)), _) try {
            if (f = 1, y && (t = op[0] & 2 ? y["return"] : op[0] ? y["throw"] || ((t = y["return"]) && t.call(y), 0) : y.next) && !(t = t.call(y, op[1])).done) return t;
            if (y = 0, t) op = [op[0] & 2, t.value];
            switch (op[0]) {
                case 0: case 1: t = op; break;
                case 4: _.label++; return { value: op[1], done: false };
                case 5: _.label++; y = op[1]; op = [0]; continue;
                case 7: op = _.ops.pop(); _.trys.pop(); continue;
                default:
                    if (!(t = _.trys, t = t.length > 0 && t[t.length - 1]) && (op[0] === 6 || op[0] === 2)) { _ = 0; continue; }
                    if (op[0] === 3 && (!t || (op[1] > t[0] && op[1] < t[3]))) { _.label = op[1]; break; }
                    if (op[0] === 6 && _.label < t[1]) { _.label = t[1]; t = op; break; }
                    if (t && _.label < t[2]) { _.label = t[2]; _.ops.push(op); break; }
                    if (t[2]) _.ops.pop();
                    _.trys.pop(); continue;
            }
            op = body.call(thisArg, _);
        } catch (e) { op = [6, e]; y = 0; } finally { f = t = 0; }
        if (op[0] & 5) throw op[1]; return { value: op[0] ? op[1] : void 0, done: true };
    }
};
var __values = (this && this.__values) || function(o) {
    var s = typeof Symbol === "function" && Symbol.iterator, m = s && o[s], i = 0;
    if (m) return m.call(o);
    if (o && typeof o.length === "number") return {
        next: function () {
            if (o && i >= o.length) o = void 0;
            return { value: o && o[i++], done: !o };
        }
    };
    throw new TypeError(s ? "Object is not iterable." : "Symbol.iterator is not defined.");
};
var __read = (this && this.__read) || function (o, n) {
    var m = typeof Symbol === "function" && o[Symbol.iterator];
    if (!m) return o;
    var i = m.call(o), r, ar = [], e;
    try {
        while ((n === void 0 || n-- > 0) && !(r = i.next()).done) ar.push(r.value);
    }
    catch (error) { e = { error: error }; }
    finally {
        try {
            if (r && !r.done && (m = i["return"])) m.call(i);
        }
        finally { if (e) throw e.error; }
    }
    return ar;
};
var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PostHog = exports.configRenames = exports.defaultConfig = void 0;
exports.init_from_snippet = init_from_snippet;
exports.init_as_module = init_as_module;
var config_1 = __importDefault(require("./config"));
var consent_1 = require("./consent");
var constants_1 = require("./constants");
var autocapture_utils_1 = require("@posthog/browser-common/utils/autocapture-utils");
var dead_clicks_autocapture_1 = require("./extensions/dead-clicks-autocapture");
var segment_integration_1 = require("./extensions/segment-integration");
var sentry_integration_1 = require("./extensions/sentry-integration");
var page_view_1 = require("./page-view");
var posthog_persistence_1 = require("./posthog-persistence");
var posthog_surveys_types_1 = require("./posthog-surveys-types");
var posthog_product_tours_types_1 = require("./posthog-product-tours-types");
var rate_limiter_1 = require("./rate-limiter");
var remote_config_1 = require("./remote-config");
var request_1 = require("./request");
var request_queue_1 = require("./request-queue");
var retry_queue_1 = require("./retry-queue");
var scroll_manager_1 = require("./scroll-manager");
var session_props_1 = require("./session-props");
var sessionid_1 = require("./sessionid");
var storage_1 = require("./storage");
var types_1 = require("./types");
var general_utils_1 = require("@posthog/browser-common/utils/general-utils");
var blocked_uas_1 = require("@posthog/browser-common/utils/blocked-uas");
var device_model_utils_1 = require("@posthog/browser-common/utils/device-model-utils");
var event_utils_1 = require("@posthog/browser-common/utils/event-utils");
var globals_1 = require("@posthog/browser-common/utils/globals");
var globals_2 = require("./utils/globals");
var logger_1 = require("@posthog/browser-common/utils/logger");
var property_utils_1 = require("@posthog/browser-common/utils/property-utils");
var request_router_1 = require("./utils/request-router");
var simple_event_emitter_1 = require("@posthog/browser-common/utils/simple-event-emitter");
var survey_utils_1 = require("./utils/survey-utils");
var core_1 = require("@posthog/core");
var uuidv7_1 = require("@posthog/browser-common/utils/uuidv7");
var external_integration_1 = require("./extensions/external-integration");
var instances = {};
// Tracks re-entrant calls to _execute_array. Used to detect when a third-party
// Proxy (e.g., TikTok's in-app browser) wraps window.posthog and converts method
// calls into push() calls, which would otherwise cause infinite recursion.
var _executeArrayDepth = 0;
var __NOOP = function () { };
var CONSENT_COOKIELESS_WARN = 'Consent opt in/out is not valid with cookieless_mode="always" and will be ignored';
var SURVEYS_NOT_AVAILABLE = 'Surveys module not available';
var SANITIZE_DEPRECATED = 'sanitize_properties is deprecated. Use before_send instead';
var DENYLIST_INVALID = 'Invalid value for property_denylist config: ';
// Transport-level keys the browser SDK carries inside event properties (unlike other SDKs,
// where they live outside `properties`). They are out of scope of the minimal
// $feature_flag_called allowlist and must survive minimization for ingestion to work.
var FLAG_CALLED_TRANSPORT_PROPERTY_KEYS = ['token', 'distinct_id', constants_1.COOKIELESS_MODE_FLAG_PROPERTY];
var PRIMARY_INSTANCE_NAME = 'posthog';
/*
 * Dynamic... constants? Is that an oxymoron?
 */
// http://hacks.mozilla.org/2009/07/cross-site-xmlhttprequest-with-cors/
// https://developer.mozilla.org/en-US/docs/DOM/XMLHttpRequest#withCredentials
// IE<10 does not support cross-origin XHR's but script tags
// with defer won't block window.onload; ENQUEUE_REQUESTS
// should only be true for Opera<12
var ENQUEUE_REQUESTS = !request_1.SUPPORTS_REQUEST && (globals_1.userAgent === null || globals_1.userAgent === void 0 ? void 0 : globals_1.userAgent.indexOf('MSIE')) === -1 && (globals_1.userAgent === null || globals_1.userAgent === void 0 ? void 0 : globals_1.userAgent.indexOf('Mozilla')) === -1;
var defaultsThatVaryByConfig = function (defaults) { return ({
    rageclick: defaults && defaults >= '2026-05-30'
        ? { content_ignorelist: autocapture_utils_1.DEFAULT_CONTENT_IGNORELIST_WITH_STEPPERS, ignore_text_selection: true }
        : defaults && defaults >= '2025-11-30'
            ? { content_ignorelist: true }
            : true,
    capture_pageview: defaults && defaults >= '2025-05-24' ? 'history_change' : true,
    session_recording: defaults && defaults >= '2026-06-25'
        ? { strictMinimumDuration: true, canvasCapture: { resolutionScale: 0.6 }, streamNetworkBody: true }
        : defaults && defaults >= '2026-05-30'
            ? { strictMinimumDuration: true, canvasCapture: { resolutionScale: 0.6 } }
            : defaults && defaults >= '2025-11-30'
                ? { strictMinimumDuration: true }
                : {},
    external_scripts_inject_target: defaults && defaults >= '2026-01-30' ? 'head' : 'body',
    internal_or_test_user_hostname: defaults && defaults >= '2026-01-30' ? /^(localhost|127\.0\.0\.1)$/ : undefined,
    persistence_save_debounce_ms: defaults && defaults >= '2026-05-30' ? 250 : 0,
    split_storage: !!(defaults && defaults >= '2026-05-30'),
    detect_google_search_app: !!(defaults && defaults >= '2026-05-30'),
    disable_capture_url_hashes: !!(defaults && defaults >= '2026-06-25'),
}); };
// NOTE: Remember to update `types.ts` when changing a default value
// to guarantee documentation is up to date, make sure to also update our website docs
// NOTE²: This shouldn't ever change because we try very hard to be backwards-compatible
var defaultConfig = function (defaults) {
    var _a;
    return (__assign({ api_host: 'https://us.i.posthog.com', flags_api_host: null, ui_host: null, asset_host: null, token: '', autocapture: true, cross_subdomain_cookie: (0, general_utils_1.isCrossDomainCookie)(globals_1.document === null || globals_1.document === void 0 ? void 0 : globals_1.document.location), persistence: 'localStorage+cookie', persistence_name: '', cookie_persisted_properties: [], loaded: __NOOP, save_campaign_params: true, custom_campaign_params: [], custom_blocked_useragents: [], save_referrer: true, capture_pageleave: 'if_capture_pageview', defaults: defaults !== null && defaults !== void 0 ? defaults : 'unset', __preview_deferred_init_extensions: false, __preview_external_dependency_versioned_paths: false, __preview_cookie_wins_on_conflict: false, debug: (globals_1.location && (0, core_1.isString)(globals_1.location === null || globals_1.location === void 0 ? void 0 : globals_1.location.search) && globals_1.location.search.indexOf('__posthog_debug=true') !== -1) || false, cookie_expiration: 365, upgrade: false, disable_session_recording: false, disable_persistence: false, disable_web_experiments: true, disable_surveys: false, disable_surveys_automatic_display: false, disable_conversations: false, disable_product_tours: false, disableDeviceModel: false, disable_external_dependency_loading: false, strict_script_versioning: false, enable_recording_console_log: undefined, secure_cookie: ((_a = globals_1.window === null || globals_1.window === void 0 ? void 0 : globals_1.window.location) === null || _a === void 0 ? void 0 : _a.protocol) === 'https:', ip: false, opt_out_capturing_by_default: false, opt_out_persistence_by_default: false, opt_out_useragent_filter: false, opt_out_capturing_persistence_type: 'localStorage', consent_persistence_name: null, opt_out_capturing_cookie_prefix: null, opt_in_site_apps: false, property_denylist: [], respect_dnt: false, sanitize_properties: null, request_headers: {}, request_batching: true, properties_string_max_length: 65535, mask_all_element_attributes: false, mask_all_text: false, mask_personal_data_properties: false, custom_personal_data_properties: [], advanced_disable_flags: false, advanced_disable_decide: false, advanced_disable_feature_flags: false, advanced_disable_feature_flags_on_first_load: false, advanced_only_evaluate_survey_feature_flags: false, advanced_feature_flags_dedup_per_session: false, advanced_enable_surveys: false, advanced_disable_toolbar_metrics: false, feature_flag_request_timeout_ms: 3000, surveys_request_timeout_ms: constants_1.SURVEYS_REQUEST_TIMEOUT_MS, on_request_error: function (res) {
            var error = 'Bad HTTP status: ' + res.statusCode + ' ' + res.text;
            logger_1.logger.error(error);
        }, get_device_id: function (uuid) { return uuid; }, capture_performance: undefined, name: 'posthog', bootstrap: {}, disable_compression: false, session_idle_timeout_seconds: 30 * 60, person_profiles: constants_1.PERSON_PROFILES_IDENTIFIED_ONLY, before_send: undefined, get_current_url: undefined, request_queue_config: { flush_interval_ms: request_queue_1.DEFAULT_FLUSH_INTERVAL_MS }, error_tracking: {}, 
        // Used for internal testing
        _onCapture: __NOOP }, defaultsThatVaryByConfig(defaults)));
};
exports.defaultConfig = defaultConfig;
var CONFIG_RENAMES = [
    ['process_person', 'person_profiles'],
    ['xhr_headers', 'request_headers'],
    ['cookie_name', 'persistence_name'],
    ['disable_cookie', 'disable_persistence'],
    ['__preview_disable_beacon', 'disable_beacon'],
    ['store_google', 'save_campaign_params'],
    ['verbose', 'debug'],
];
var configRenames = function (origConfig) {
    var e_1, _a;
    var renames = {};
    try {
        for (var CONFIG_RENAMES_1 = __values(CONFIG_RENAMES), CONFIG_RENAMES_1_1 = CONFIG_RENAMES_1.next(); !CONFIG_RENAMES_1_1.done; CONFIG_RENAMES_1_1 = CONFIG_RENAMES_1.next()) {
            var _b = __read(CONFIG_RENAMES_1_1.value, 2), oldKey = _b[0], newKey = _b[1];
            if (!(0, core_1.isUndefined)(origConfig[oldKey])) {
                ;
                renames[newKey] = origConfig[oldKey];
            }
        }
    }
    catch (e_1_1) { e_1 = { error: e_1_1 }; }
    finally {
        try {
            if (CONFIG_RENAMES_1_1 && !CONFIG_RENAMES_1_1.done && (_a = CONFIG_RENAMES_1.return)) _a.call(CONFIG_RENAMES_1);
        }
        finally { if (e_1) throw e_1.error; }
    }
    // on_xhr_error is not present, as the type is different to on_request_error
    // the original config takes priority over the renames
    var newConfig = (0, general_utils_1.extend)({}, renames, origConfig);
    // Maintain backwards compatibility for the deprecated preview option while
    // letting the supported options take precedence when they are provided.
    var previewExternalDependencyVersionedPaths = origConfig.__preview_external_dependency_versioned_paths;
    if (!(0, core_1.isUndefined)(previewExternalDependencyVersionedPaths)) {
        if ((0, core_1.isUndefined)(origConfig.strict_script_versioning)) {
            newConfig.strict_script_versioning = !!previewExternalDependencyVersionedPaths;
        }
        if ((0, core_1.isString)(previewExternalDependencyVersionedPaths) && (0, core_1.isUndefined)(origConfig.asset_host)) {
            newConfig.asset_host = previewExternalDependencyVersionedPaths;
        }
    }
    // merge property_blacklist into property_denylist
    if ((0, core_1.isArray)(origConfig.property_blacklist)) {
        if ((0, core_1.isUndefined)(origConfig.property_denylist)) {
            newConfig.property_denylist = origConfig.property_blacklist;
        }
        else if ((0, core_1.isArray)(origConfig.property_denylist)) {
            newConfig.property_denylist = __spreadArray(__spreadArray([], __read(origConfig.property_blacklist), false), __read(origConfig.property_denylist), false);
        }
        else {
            logger_1.logger.error(DENYLIST_INVALID + origConfig.property_denylist);
        }
    }
    return newConfig;
};
exports.configRenames = configRenames;
var DeprecatedWebPerformanceObserver = /** @class */ (function () {
    function DeprecatedWebPerformanceObserver() {
        this.__forceAllowLocalhost = false;
    }
    Object.defineProperty(DeprecatedWebPerformanceObserver.prototype, "_forceAllowLocalhost", {
        get: function () {
            return this.__forceAllowLocalhost;
        },
        set: function (value) {
            logger_1.logger.error('WebPerformanceObserver is deprecated and has no impact on network capture. Use `_forceAllowLocalhostNetworkCapture` on `posthog.sessionRecording`');
            this.__forceAllowLocalhost = value;
        },
        enumerable: false,
        configurable: true
    });
    return DeprecatedWebPerformanceObserver;
}());
/**
 *
 * This is the SDK reference for the PostHog JavaScript Web SDK.
 * You can learn more about example usage in the
 * [JavaScript Web SDK documentation](/docs/libraries/js).
 * You can also follow [framework specific guides](/docs/frameworks)
 * to integrate PostHog into your project.
 *
 * This SDK is designed for browser environments.
 * Use the PostHog [Node.js SDK](/docs/libraries/node) for server-side usage.
 *
 * @constructor
 */
var PostHog = /** @class */ (function () {
    /**
     * Creates an uninitialized PostHog instance.
     *
     * @remarks
     * Most browser applications should use the default exported singleton and call `posthog.init()`.
     * Construct a new instance only when you need to manage a separate SDK instance manually.
     *
     * @example
     * ```js
     * const instance = new PostHog()
     * instance.init('<ph_project_api_key>', { api_host: 'https://us.i.posthog.com' })
     * ```
     */
    function PostHog() {
        var _this = this;
        var _a;
        this.webPerformance = new DeprecatedWebPerformanceObserver();
        this._personProcessingSetOncePropertiesSent = false;
        this.version = config_1.default.LIB_VERSION;
        this._internalEventEmitter = new simple_event_emitter_1.SimpleEventEmitter();
        this._extensions = [];
        /** @deprecated - deprecated in 1.241.0, use `calculateEventProperties` instead  */
        this._calculate_event_properties = this.calculateEventProperties.bind(this);
        this.config = (0, exports.defaultConfig)();
        this.SentryIntegration = sentry_integration_1.SentryIntegration;
        this.sentryIntegration = function (options) { return (0, sentry_integration_1.sentryIntegration)(_this, options); };
        this.__request_queue = [];
        this.__loaded = false;
        this.analyticsDefaultEndpoint = '/e/';
        this._initialPageviewCaptured = false;
        this._visibilityStateListener = null;
        this._initialPersonProfilesConfig = null;
        this._cachedPersonProperties = null;
        this.scrollManager = new scroll_manager_1.ScrollManager(this);
        this.pageViewManager = new page_view_1.PageViewManager(this);
        this.rateLimiter = new rate_limiter_1.RateLimiter(this);
        this.requestRouter = new request_router_1.RequestRouter(this);
        this.consent = new consent_1.ConsentManager(this);
        this.externalIntegrations = new external_integration_1.ExternalIntegrations(this);
        // Eagerly construct extensions from default classes so they're available before init().
        // For the slim bundle, these remain undefined until _initExtensions sets them from config.
        var ext = (_a = PostHog.__defaultExtensionClasses) !== null && _a !== void 0 ? _a : {};
        this.featureFlags = ext.featureFlags && new ext.featureFlags(this);
        this.toolbar = ext.toolbar && new ext.toolbar(this);
        this.surveys = ext.surveys && new ext.surveys(this);
        this.conversations = ext.conversations && new ext.conversations(this);
        this.logs = ext.logs && new ext.logs(this);
        this.metrics = ext.metrics && new ext.metrics(this);
        this.experiments = ext.experiments && new ext.experiments(this);
        this.exceptions = ext.exceptions && new ext.exceptions(this);
        // NOTE: See the property definition for deprecation notice
        this.people = {
            set: function (prop, to, callback) {
                var _a;
                var setProps = (0, core_1.isString)(prop) ? (_a = {}, _a[prop] = to, _a) : prop;
                _this.setPersonProperties(setProps);
                callback === null || callback === void 0 ? void 0 : callback({});
            },
            set_once: function (prop, to, callback) {
                var _a;
                var setProps = (0, core_1.isString)(prop) ? (_a = {}, _a[prop] = to, _a) : prop;
                _this.setPersonProperties(undefined, setProps);
                callback === null || callback === void 0 ? void 0 : callback({});
            },
        };
        this.on('eventCaptured', function (data) { return logger_1.logger.info("send \"".concat(data === null || data === void 0 ? void 0 : data.event, "\""), data); });
    }
    PostHog.prototype._replaceExtension = function (oldExt, newExt) {
        var _a;
        if (oldExt) {
            var idx = this._extensions.indexOf(oldExt);
            if (idx !== -1) {
                this._extensions.splice(idx, 1);
            }
        }
        this._extensions.push(newExt);
        (_a = newExt.initialize) === null || _a === void 0 ? void 0 : _a.call(newExt);
        return newExt;
    };
    PostHog.prototype._inCookielessMode = function () {
        return (this.config.cookieless_mode === constants_1.COOKIELESS_ALWAYS ||
            (this.config.cookieless_mode === constants_1.COOKIELESS_ON_REJECT && this.consent.isRejected()));
    };
    Object.defineProperty(PostHog.prototype, "decideEndpointWasHit", {
        // Legacy property to support existing usage - this isn't technically correct but it's what it has always been - a proxy for flags being loaded
        /** @deprecated Use `flagsEndpointWasHit` instead.  We migrated to using a new feature flag endpoint and the new method is more semantically accurate */
        get: function () {
            var _a, _b;
            return (_b = (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.hasLoadedFlags) !== null && _b !== void 0 ? _b : false;
        },
        enumerable: false,
        configurable: true
    });
    Object.defineProperty(PostHog.prototype, "flagsEndpointWasHit", {
        /**
         * Whether feature flags have been initialized for this instance.
         *
         * @remarks
         * Despite the name, this is a proxy for processed feature flag values. It may be true for
         * bootstrapped flags before a network request completes.
         *
         * @returns True once the SDK has processed feature flag values.
         */
        get: function () {
            var _a, _b;
            return (_b = (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.hasLoadedFlags) !== null && _b !== void 0 ? _b : false;
        },
        enumerable: false,
        configurable: true
    });
    // Initialization methods
    /**
     * Initializes a new instance of the PostHog capturing object.
     *
     * @remarks
     * All new instances are added to the main posthog object as sub properties (such as
     * `posthog.library_name`) and also returned by this function. [Learn more about configuration options](https://posthog.com/docs/libraries/js/config)
     *
     * @example
     * ```js
     * // basic initialization
     * posthog.init('<ph_project_api_key>', {
     *     api_host: '<ph_client_api_host>'
     * })
     * ```
     *
     * @example
     * ```js
     * // multiple instances
     * posthog.init('<ph_project_api_key>', {}, 'project1')
     * posthog.init('<ph_project_api_key>', {}, 'project2')
     * ```
     *
     * @public
     *
     * @param token - Your PostHog API token
     * @param config - A dictionary of config options to override
     * @param name - The name for the new posthog instance that you want created
     *
     * {@label Initialization}
     *
     * @returns The newly initialized PostHog instance
     */
    PostHog.prototype.init = function (token, config, name) {
        var _a;
        if (!name || name === PRIMARY_INSTANCE_NAME) {
            // This means we are initializing the primary instance (i.e. this)
            return this._init(token, config, name);
        }
        else {
            var namedPosthog = (_a = instances[name]) !== null && _a !== void 0 ? _a : new PostHog();
            namedPosthog._init(token, config, name);
            instances[name] = namedPosthog;
            instances[PRIMARY_INSTANCE_NAME][name] = namedPosthog;
            return namedPosthog;
        }
    };
    // posthog._init(token:string, config:object, name:string)
    //
    // This function sets up the current instance of the posthog
    // library.  The difference between this method and the init(...)
    // method is this one initializes the actual instance, whereas the
    // init(...) method sets up a new library and calls _init on it.
    //
    // Note that there are operations that can be asynchronous, so we
    // accept a callback that is called when all the asynchronous work
    // is done. Note that we do not use promises because we want to be
    // IE11 compatible. We could use polyfills, which would make the
    // code a bit cleaner, but will add some overhead.
    //
    PostHog.prototype._init = function (token, config, name) {
        var _this = this;
        var _a, _b;
        if (config === void 0) { config = {}; }
        var normalizedToken = (0, core_1.isString)(token) ? token.trim() : '';
        if (!normalizedToken) {
            logger_1.logger.critical('PostHog was initialized without a token. This likely indicates a misconfiguration. Please check the first argument passed to posthog.init()');
            return this;
        }
        if (this.__loaded) {
            // need to be able to log before having processed debug config
            // eslint-disable-next-line no-console
            console.warn('[PostHog.js]', 'You have already initialized PostHog! Re-initializing is a no-op');
            return this;
        }
        this.__loaded = true;
        this.config = (0, exports.defaultConfig)(config.defaults); // fully overwritten by set_config right below
        config.debug = this._checkLocalStorageForDebug(config.debug);
        this._originalUserConfig = config; // Store original user config for migration
        this._triggered_notifs = [];
        if (config.person_profiles) {
            this._initialPersonProfilesConfig = config.person_profiles;
        }
        else if (config.process_person) {
            this._initialPersonProfilesConfig = config.process_person;
        }
        var baseConfig = (0, exports.defaultConfig)(config.defaults);
        var userConfig = (0, exports.configRenames)(config);
        var mergedConfig = (0, general_utils_1.extend)({}, baseConfig, userConfig, {
            name: name,
            token: normalizedToken,
        });
        // a partial user-supplied rageclick object should keep the date-gated defaults
        // (e.g. content_ignorelist, ignore_text_selection) rather than replacing them wholesale
        if ((0, core_1.isObject)(baseConfig.rageclick) && (0, core_1.isObject)(userConfig.rageclick)) {
            mergedConfig.rageclick = (0, general_utils_1.extend)({}, baseConfig.rageclick, userConfig.rageclick);
        }
        // likewise a partial user-supplied session_recording keeps the date-gated top-level
        // defaults (e.g. strictMinimumDuration, canvasCapture) it doesn't set. this is a shallow
        // one-level merge: a user-supplied nested object (e.g. their own canvasCapture) still
        // replaces the default one wholesale rather than merging into it.
        if ((0, core_1.isObject)(baseConfig.session_recording) && (0, core_1.isObject)(userConfig.session_recording)) {
            mergedConfig.session_recording = (0, general_utils_1.extend)({}, baseConfig.session_recording, userConfig.session_recording);
        }
        this.set_config(mergedConfig);
        if (this.config.on_xhr_error) {
            logger_1.logger.error('on_xhr_error is deprecated. Use on_request_error instead');
        }
        this.compression = config.disable_compression ? undefined : types_1.Compression.GZipJS;
        var persistenceDisabled = this._is_persistence_disabled();
        this.persistence = new posthog_persistence_1.PostHogPersistence(this.config, persistenceDisabled);
        this.sessionPersistence =
            this.config.persistence === 'sessionStorage' || this.config.persistence === 'memory'
                ? this.persistence
                : // sessionStorage sibling shares the primary's storage name; it must not own/clean the split group entries
                    new posthog_persistence_1.PostHogPersistence(__assign(__assign({}, this.config), { persistence: 'sessionStorage' }), persistenceDisabled, false);
        // should I store the initial person profiles config in persistence?
        var initialPersistenceProps = __assign({}, this.persistence.props);
        var initialSessionProps = __assign({}, this.sessionPersistence.props);
        this.register({ $initialization_time: new Date().toISOString() });
        this._requestQueue = new request_queue_1.RequestQueue(function (req) { return _this._send_retriable_request(req); }, this.config.request_queue_config);
        this._retryQueue = new retry_queue_1.RetryQueue(this);
        this.__request_queue = [];
        var startInCookielessMode = this._inCookielessMode();
        if (!startInCookielessMode) {
            this.sessionManager = new sessionid_1.SessionIdManager(this);
            this.sessionPropsManager = new session_props_1.SessionPropsManager(this, this.sessionManager, this.persistence);
        }
        // Conditionally defer extension initialization based on config
        if (this.config.__preview_deferred_init_extensions) {
            // EXPERIMENTAL: Defer non-critical extension initialization to next tick
            // This reduces main thread blocking during init
            // while keeping critical path (persistence, sessions, capture) synchronous
            logger_1.logger.info('Deferring extension initialization to improve startup performance');
            setTimeout(function () {
                _this._initExtensions(startInCookielessMode);
            }, 0);
        }
        else {
            // Legacy synchronous initialization (default for now)
            logger_1.logger.info('Initializing extensions synchronously');
            this._initExtensions(startInCookielessMode);
        }
        // if any instance on the page has debug = true, we set the
        // global debug to be true
        config_1.default.DEBUG = config_1.default.DEBUG || this.config.debug;
        if (config_1.default.DEBUG) {
            logger_1.logger.info('Starting in debug mode', {
                this: this,
                config: config,
                thisC: __assign({}, this.config),
                p: initialPersistenceProps,
                s: initialSessionProps,
            });
        }
        // When identity_distinct_id is provided at init time, use it as the
        // bootstrap distinct ID so the Person record is created from the first event.
        if (this.config.identity_distinct_id && !((_a = config.bootstrap) === null || _a === void 0 ? void 0 : _a.distinctID)) {
            config.bootstrap = __assign(__assign({}, config.bootstrap), { distinctID: this.config.identity_distinct_id, isIdentifiedID: true });
        }
        // isUndefined doesn't provide typehint here so wouldn't reduce bundle as we'd need to assign
        // eslint-disable-next-line posthog-js/no-direct-undefined-check
        if (((_b = config.bootstrap) === null || _b === void 0 ? void 0 : _b.distinctID) !== undefined) {
            var bootstrapDistinctId = config.bootstrap.distinctID;
            var existingDistinctId = this.get_distinct_id();
            var existingUserState = this.persistence.get_property(constants_1.USER_STATE);
            if (config.bootstrap.isIdentifiedID &&
                existingDistinctId != null &&
                existingDistinctId !== bootstrapDistinctId &&
                existingUserState === constants_1.USER_STATE_ANONYMOUS) {
                // The server bootstrapped identity for an identified user, but local persistence
                // still has an anonymous ID from a previous session. Calling identify() merges
                // the anonymous user into the identified user, ensuring consistent identity
                // for feature flag evaluation and preventing duplicate $feature_flag_called events.
                //
                // Note: this runs during _init(), before _loaded() enables the request queue.
                // The $identify event is enqueued and flushed once the queue starts. The
                // reloadFeatureFlags() call inside identify() sets _reloadDebouncer, so the
                // subsequent ensureFlagsLoaded() from _onRemoteConfig is a no-op (no double request).
                this.identify(bootstrapDistinctId);
            }
            else if (config.bootstrap.isIdentifiedID &&
                existingDistinctId != null &&
                existingDistinctId !== bootstrapDistinctId &&
                existingUserState === constants_1.USER_STATE_IDENTIFIED) {
                // The existing user is already identified with a different ID. Silently
                // switching identities without an $identify event would corrupt analytics.
                // Preserve the existing identity and log a warning.
                logger_1.logger.warn('Bootstrap distinctID differs from an already-identified user. ' +
                    'The existing identity is preserved. Call reset() before reinitializing ' +
                    'if you intend to switch users.');
            }
            else {
                var uuid = this.config.get_device_id((0, uuidv7_1.uuidv7)());
                var deviceID = config.bootstrap.isIdentifiedID ? uuid : bootstrapDistinctId;
                this.persistence.set_property(constants_1.USER_STATE, config.bootstrap.isIdentifiedID ? constants_1.USER_STATE_IDENTIFIED : constants_1.USER_STATE_ANONYMOUS);
                this.register({
                    distinct_id: bootstrapDistinctId,
                    $device_id: deviceID,
                });
            }
        }
        if (startInCookielessMode) {
            this.register_once({
                distinct_id: constants_1.COOKIELESS_SENTINEL_VALUE,
                $device_id: null,
            }, '');
        }
        else if (!this.get_distinct_id()) {
            // There is no need to set the distinct id
            // or the device id if something was already stored
            // in the persistence
            var uuid = this.config.get_device_id((0, uuidv7_1.uuidv7)());
            this.register_once({
                distinct_id: uuid,
                $device_id: uuid,
            }, '');
            // distinct id == $device_id is a proxy for anonymous user
            this.persistence.set_property(constants_1.USER_STATE, constants_1.USER_STATE_ANONYMOUS);
        }
        // Set up event handler for pageleave
        // Use `onpagehide` if available, see https://calendar.perfplanet.com/2020/beaconing-in-practice/#beaconing-reliability-avoiding-abandons
        //
        // Not making it passive to try and force the browser to handle this before the page is unloaded
        (0, general_utils_1.addEventListener)(globals_1.window, 'onpagehide' in self ? 'pagehide' : 'unload', this._handle_unload.bind(this), {
            passive: false,
        });
        // We want to avoid promises for IE11 compatibility, so we use callbacks here
        if (config.segment) {
            (0, segment_integration_1.setupSegmentIntegration)(this, function () { return _this._loaded(); });
        }
        else {
            this._loaded();
        }
        if ((0, core_1.isFunction)(this.config._onCapture) && this.config._onCapture !== __NOOP) {
            logger_1.logger.warn('onCapture is deprecated. Please use `before_send` instead');
            this.on('eventCaptured', function (data) { return _this.config._onCapture(data.event, data); });
        }
        if (this.config.ip) {
            logger_1.logger.warn('The `ip` config option has NO EFFECT AT ALL and has been deprecated. Use a custom transformation or "Discard IP data" project setting instead. See https://posthog.com/tutorials/web-redact-properties#hiding-customer-ip-address for more information.');
        }
        // Not awaited — the first event may miss $device_model, which is fine for a stable per-device dimension.
        if (!this.config.disableDeviceModel) {
            (0, device_model_utils_1.getDeviceModel)()
                .then(function (model) {
                var _a;
                if (model) {
                    _this.register((_a = {}, _a[constants_1.DEVICE_MODEL] = model, _a));
                }
            })
                .catch(__NOOP);
        }
        return this;
    };
    PostHog.prototype._initExtensions = function (startInCookielessMode) {
        var _this = this;
        var _a, _b, _c, _d, _e, _f, _g, _h;
        // we don't support IE11 anymore, so performance.now is safe
        // eslint-disable-next-line compat/compat
        var initStartTime = performance.now();
        var ext = __assign(__assign({}, PostHog.__defaultExtensionClasses), this.config.__extensionClasses);
        var initTasks = [];
        // Due to name mangling, we can't easily iterate and assign these extensions
        // The assignment needs to also be mangled. Thus, the loop is unrolled.
        if (ext.featureFlags) {
            this._extensions.push((this.featureFlags = (_a = this.featureFlags) !== null && _a !== void 0 ? _a : new ext.featureFlags(this)));
        }
        if (ext.exceptions) {
            this._extensions.push((this.exceptions = (_b = this.exceptions) !== null && _b !== void 0 ? _b : new ext.exceptions(this)));
        }
        if (ext.historyAutocapture) {
            this._extensions.push((this.historyAutocapture = new ext.historyAutocapture(this)));
        }
        if (ext.tracingHeaders) {
            this._extensions.push((this.tracingHeaders = new ext.tracingHeaders(this)));
        }
        if (ext.siteApps) {
            this._extensions.push((this.siteApps = new ext.siteApps(this)));
        }
        if (ext.sessionRecording && !startInCookielessMode) {
            this._extensions.push((this.sessionRecording = new ext.sessionRecording(this)));
        }
        if (!this.config.disable_scroll_properties) {
            initTasks.push(function () {
                _this.scrollManager.startMeasuringScrollPosition();
            });
        }
        if (ext.autocapture) {
            this._extensions.push((this.autocapture = new ext.autocapture(this)));
        }
        if (ext.surveys) {
            this._extensions.push((this.surveys = (_c = this.surveys) !== null && _c !== void 0 ? _c : new ext.surveys(this)));
        }
        if (ext.logs) {
            this._extensions.push((this.logs = (_d = this.logs) !== null && _d !== void 0 ? _d : new ext.logs(this)));
        }
        if (ext.metrics) {
            this._extensions.push((this.metrics = (_e = this.metrics) !== null && _e !== void 0 ? _e : new ext.metrics(this)));
        }
        if (ext.conversations) {
            this._extensions.push((this.conversations = (_f = this.conversations) !== null && _f !== void 0 ? _f : new ext.conversations(this)));
        }
        if (ext.productTours) {
            this._extensions.push((this.productTours = new ext.productTours(this)));
        }
        if (ext.heatmaps) {
            this._extensions.push((this.heatmaps = new ext.heatmaps(this)));
        }
        if (ext.webVitalsAutocapture) {
            this._extensions.push((this.webVitalsAutocapture = new ext.webVitalsAutocapture(this)));
        }
        if (ext.exceptionObserver) {
            this._extensions.push((this.exceptionObserver = new ext.exceptionObserver(this)));
        }
        if (ext.deadClicksAutocapture) {
            this._extensions.push((this.deadClicksAutocapture = new ext.deadClicksAutocapture(this, dead_clicks_autocapture_1.isDeadClicksEnabledForAutocapture)));
        }
        if (ext.toolbar) {
            this._extensions.push((this.toolbar = (_g = this.toolbar) !== null && _g !== void 0 ? _g : new ext.toolbar(this)));
        }
        if (ext.experiments) {
            this._extensions.push((this.experiments = (_h = this.experiments) !== null && _h !== void 0 ? _h : new ext.experiments(this)));
        }
        this._extensions.forEach(function (extension) {
            if (!extension.initialize)
                return;
            initTasks.push(function () {
                var _a;
                (_a = extension.initialize) === null || _a === void 0 ? void 0 : _a.call(extension);
            });
        });
        // Replay any pending remote config that arrived before extensions were ready
        initTasks.push(function () {
            if (_this._pendingRemoteConfig) {
                var result = _this._pendingRemoteConfig;
                _this._pendingRemoteConfig = undefined; // Clear before replaying to avoid re-storing
                _this._onRemoteConfig(result);
            }
        });
        // Process tasks with time-slicing to avoid blocking
        this._processInitTaskQueue(initTasks, initStartTime);
    };
    PostHog.prototype._processInitTaskQueue = function (queue, initStartTime) {
        var _a;
        var _this = this;
        var TIME_BUDGET_MS = 30; // Respect frame budget (~60fps = 16ms, but we're already deferred)
        while (queue.length > 0) {
            // Only time-slice if deferred init is enabled, otherwise run synchronously
            if (this.config.__preview_deferred_init_extensions) {
                // we don't support IE11 anymore, so performance.now is safe
                var elapsed = performance.now() - initStartTime;
                // Check if we've exceeded our time budget
                if (elapsed >= TIME_BUDGET_MS && queue.length > 0) {
                    // Yield to browser, then continue processing
                    setTimeout(function () {
                        _this._processInitTaskQueue(queue, initStartTime);
                    }, 0);
                    return;
                }
            }
            // Process next task
            var task = queue.shift();
            if (task) {
                try {
                    task();
                }
                catch (error) {
                    logger_1.logger.error('Error initializing extension:', error);
                }
            }
        }
        // All tasks complete - record timing for both sync and deferred modes
        // we don't support IE11 anymore, so performance.now is safe
        // eslint-disable-next-line compat/compat
        var taskInitTiming = Math.round(performance.now() - initStartTime);
        this.register_for_session((_a = {},
            _a[constants_1.SDK_DEBUG_EXTENSIONS_INIT_METHOD] = this.config.__preview_deferred_init_extensions
                ? 'deferred'
                : 'synchronous',
            _a[constants_1.SDK_DEBUG_EXTENSIONS_INIT_TIME_MS] = taskInitTiming,
            _a));
        if (this.config.__preview_deferred_init_extensions) {
            logger_1.logger.info("PostHog extensions initialized (".concat(taskInitTiming, "ms)"));
        }
    };
    PostHog.prototype._onRemoteConfig = function (result) {
        var _this = this;
        var _a;
        if (!(globals_1.document && globals_1.document.body)) {
            logger_1.logger.info('document not ready yet, trying again in 500 milliseconds...');
            setTimeout(function () {
                _this._onRemoteConfig(result);
            }, 500);
            return;
        }
        // Store config in case extensions aren't initialized yet (only needed for deferred init)
        if (this.config.__preview_deferred_init_extensions) {
            this._pendingRemoteConfig = result;
        }
        // Cache the latest remote config result so extensions that are created later
        // (e.g. sessionRecording after opt_in_capturing from cookieless mode) can
        // replay it and pick up server-side settings like recording enable flags.
        // Storing the result (not just a config) means a replayed failure is
        // distinguishable from a successful empty config.
        this._lastRemoteConfig = result;
        this.compression = undefined;
        if (result.ok) {
            var config = result.config;
            if (config.supportedCompression && !this.config.disable_compression) {
                this.compression = (0, core_1.includes)(config['supportedCompression'], types_1.Compression.GZipJS)
                    ? types_1.Compression.GZipJS
                    : (0, core_1.includes)(config['supportedCompression'], types_1.Compression.Base64)
                        ? types_1.Compression.Base64
                        : undefined;
            }
            if ((_a = config.analytics) === null || _a === void 0 ? void 0 : _a.endpoint) {
                this.analyticsDefaultEndpoint = config.analytics.endpoint;
            }
        }
        // Runs on failure too: the person_profiles default must be applied even when
        // the remote config could not be fetched.
        this.set_config({
            person_profiles: this._initialPersonProfilesConfig
                ? this._initialPersonProfilesConfig
                : constants_1.PERSON_PROFILES_IDENTIFIED_ONLY,
        });
        // Every extension receives the full result and handles the failure case itself.
        this._extensions.forEach(function (ext) { var _a; return (_a = ext.onRemoteConfig) === null || _a === void 0 ? void 0 : _a.call(ext, result); });
    };
    PostHog.prototype._loaded = function () {
        var _this = this;
        try {
            this.config.loaded(this);
        }
        catch (err) {
            logger_1.logger.critical('`loaded` function failed', err);
        }
        this._start_queue_if_opted_in();
        // Check if current hostname matches internal_or_test_user_hostname pattern and mark as test user before any events
        if (this.config.internal_or_test_user_hostname && (globals_1.location === null || globals_1.location === void 0 ? void 0 : globals_1.location.hostname)) {
            var hostname = globals_1.location.hostname;
            var pattern = this.config.internal_or_test_user_hostname;
            var matches = typeof pattern === 'string' ? hostname === pattern : pattern.test(hostname);
            if (matches) {
                this.setInternalOrTestUser();
            }
        }
        // this happens after "loaded" so a user can call identify or any other things before the pageview fires
        if (this.config.capture_pageview) {
            // NOTE: We want to fire this on the next tick as the previous implementation had this side effect
            // and some clients may rely on it
            setTimeout(function () {
                if (_this.consent.isOptedIn() || _this._inCookielessMode()) {
                    _this._captureInitialPageview();
                }
            }, 1);
        }
        this._remoteConfigLoader = new remote_config_1.RemoteConfigLoader(this);
        this._remoteConfigLoader.load();
    };
    PostHog.prototype._start_queue_if_opted_in = function () {
        var _a;
        if (this.is_capturing()) {
            if (this.config.request_batching) {
                (_a = this._requestQueue) === null || _a === void 0 ? void 0 : _a.enable();
            }
        }
    };
    PostHog.prototype._dom_loaded = function () {
        var _this = this;
        if (this.is_capturing()) {
            (0, general_utils_1.eachArray)(this.__request_queue, function (item) { return _this._send_retriable_request(item); });
        }
        this.__request_queue = [];
        this._start_queue_if_opted_in();
    };
    PostHog.prototype._handle_unload = function () {
        var _a, _b, _c, _d, _e, _f;
        // Optional-call the method, not just the receiver: after a deploy a cached older
        // lazy-loaded surveys chunk can yield an instance whose prototype lacks handlePageUnload,
        // and `this.surveys?.handlePageUnload()` would still throw "handlePageUnload is not a function".
        (_b = (_a = this.surveys) === null || _a === void 0 ? void 0 : _a.handlePageUnload) === null || _b === void 0 ? void 0 : _b.call(_a);
        // Metrics are pre-aggregated client-side and don't ride the request
        // queue, so the drain must not depend on `request_batching`.
        void ((_c = this.metrics) === null || _c === void 0 ? void 0 : _c.flush('sendBeacon'));
        if (!this.config.request_batching) {
            if (this._shouldCapturePageleave()) {
                this.capture(constants_1.EVENT_PAGELEAVE, null, { transport: 'sendBeacon' });
            }
            return;
        }
        if (this._shouldCapturePageleave()) {
            this.capture(constants_1.EVENT_PAGELEAVE);
        }
        (_d = this.logs) === null || _d === void 0 ? void 0 : _d.flushLogs('sendBeacon');
        (_e = this._requestQueue) === null || _e === void 0 ? void 0 : _e.unload();
        (_f = this._retryQueue) === null || _f === void 0 ? void 0 : _f.unload();
    };
    PostHog.prototype._send_request = function (options) {
        var _this = this;
        var _a, _b;
        if (!this.__loaded) {
            if (options.fireCallbackOnDrop) {
                (_a = options.callback) === null || _a === void 0 ? void 0 : _a.call(options, { statusCode: 0 });
            }
            return;
        }
        if (ENQUEUE_REQUESTS) {
            this.__request_queue.push(options);
            return;
        }
        if (this.rateLimiter.isServerRateLimited(options.batchKey)) {
            if (options.fireCallbackOnDrop) {
                (_b = options.callback) === null || _b === void 0 ? void 0 : _b.call(options, { statusCode: 429 });
            }
            return;
        }
        options.transport = options.transport || this.config.api_transport;
        options.headers = __assign(__assign({}, this.config.request_headers), options.headers);
        options.compression = options.compression === 'best-available' ? this.compression : options.compression;
        var disableBeacon = (0, core_1.isUndefined)(this.config.disable_beacon)
            ? this.config.__preview_disable_beacon
            : this.config.disable_beacon;
        if (disableBeacon) {
            options.disableTransport = ['sendBeacon'];
        }
        // Specially useful if you're doing SSR with NextJS
        // Users must be careful when tweaking `cache` because they might get out-of-date feature flags
        options.fetchOptions = options.fetchOptions || this.config.fetch_options;
        (0, request_1.request)(__assign(__assign({}, options), { callback: function (response) {
                var _a, _b, _c;
                _this.rateLimiter.checkForLimiting(response);
                if (response.statusCode >= 400) {
                    (_b = (_a = _this.config).on_request_error) === null || _b === void 0 ? void 0 : _b.call(_a, response);
                }
                (_c = options.callback) === null || _c === void 0 ? void 0 : _c.call(options, response);
            } }));
    };
    PostHog.prototype._send_retriable_request = function (options) {
        if (this._retryQueue) {
            this._retryQueue.retriableRequest(options);
        }
        else {
            this._send_request(options);
        }
    };
    /**
     * _execute_array() deals with processing any posthog function
     * calls that were called before the PostHog library were loaded
     * (and are thus stored in an array so they can be called later)
     *
     * Note: we fire off all the posthog function calls && user defined
     * functions BEFORE we fire off posthog capturing calls. This is so
     * identify/register/set_config calls can properly modify early
     * capturing calls.
     *
     * @param {Array} array
     */
    PostHog.prototype._execute_array = function (array) {
        var _this = this;
        _executeArrayDepth++;
        try {
            var fn_name_1;
            var alias_calls_1 = [];
            var other_calls_1 = [];
            var capturing_calls_1 = [];
            (0, general_utils_1.eachArray)(array, function (item) {
                if (item) {
                    fn_name_1 = item[0];
                    if ((0, core_1.isArray)(fn_name_1)) {
                        capturing_calls_1.push(item); // chained call e.g. posthog.get_group().set()
                    }
                    else if ((0, core_1.isFunction)(item)) {
                        try {
                            ;
                            item.call(_this);
                        }
                        catch (e) {
                            logger_1.logger.error('Error executing queued PostHog call', item, e);
                        }
                    }
                    else if ((0, core_1.isArray)(item) && fn_name_1 === 'alias') {
                        alias_calls_1.push(item);
                    }
                    else if ((0, core_1.isArray)(item) &&
                        fn_name_1.indexOf('capture') !== -1 &&
                        (0, core_1.isFunction)(_this[fn_name_1])) {
                        capturing_calls_1.push(item);
                    }
                    else {
                        other_calls_1.push(item);
                    }
                }
            });
            var execute = function (calls, thisArg) {
                (0, general_utils_1.eachArray)(calls, function (item) {
                    try {
                        if ((0, core_1.isArray)(item[0])) {
                            // chained call
                            var caller_1 = thisArg;
                            (0, general_utils_1.each)(item, function (call) {
                                caller_1 = caller_1[call[0]].apply(caller_1, call.slice(1));
                            });
                        }
                        else {
                            thisArg[item[0]].apply(thisArg, item.slice(1));
                        }
                    }
                    catch (e) {
                        logger_1.logger.error('Error executing queued PostHog call', item, e);
                    }
                });
            };
            execute(alias_calls_1, this);
            execute(other_calls_1, this);
            execute(capturing_calls_1, this);
        }
        finally {
            _executeArrayDepth--;
        }
    };
    /**
     * push() keeps the standard async-array-push
     * behavior around after the lib is loaded.
     * This is only useful for external integrations that
     * do not wish to rely on our convenience methods
     * (created in the snippet).
     *
     * @example
     * ```js
     * posthog.push(['register', { a: 'b' }]);
     * ```
     *
     * @param {SnippetArrayItem} item A `[function_name, ...args]` array to be executed.
     */
    PostHog.prototype.push = function (item) {
        if (_executeArrayDepth > 0 && (0, core_1.isArray)(item) && (0, core_1.isString)(item[0])) {
            // push() is being called while _execute_array is already running.
            // This happens when a third-party Proxy (e.g., TikTok's in-app browser)
            // wraps window.posthog and converts method calls into push() calls,
            // creating an infinite loop: _execute_array -> this[method] -> Proxy ->
            // push -> _execute_array -> ...
            // Dispatch directly from the prototype to break the cycle.
            var fn = PostHog.prototype[item[0]];
            if ((0, core_1.isFunction)(fn)) {
                fn.apply(this, item.slice(1));
            }
            return;
        }
        this._execute_array([item]);
    };
    /**
     * Captures an event with optional properties and configuration.
     *
     * @remarks
     * You can capture arbitrary object-like values as events. [Learn about capture best practices](/docs/product-analytics/capture-events)
     *
     * @example
     * ```js
     * // basic event capture
     * posthog.capture('cta-button-clicked', {
     *     button_name: 'Get Started',
     *     page: 'homepage'
     * })
     * ```
     *
     * {@label Capture}
     *
     * @public
     *
     * @param event_name - The name of the event (e.g., 'Sign Up', 'Button Click', 'Purchase')
     * @param properties - Properties to include with the event describing the user or event details
     * @param options - Optional configuration for the capture request
     *
     * @returns The capture result containing event data, or undefined if capture failed
     */
    PostHog.prototype.capture = function (event_name, properties, options) {
        var _a, _b, _c;
        var _d;
        // While developing, a developer might purposefully _not_ call init(),
        // in this case, we would like capture to be a noop.
        if (!this.__loaded || !this.persistence || !this.sessionPersistence || !this._requestQueue) {
            logger_1.logger.uninitializedWarning('posthog.capture');
            return;
        }
        if (!this.is_capturing()) {
            return;
        }
        // typing doesn't prevent interesting data
        if ((0, core_1.isUndefined)(event_name) || !(0, core_1.isString)(event_name)) {
            logger_1.logger.error('No event name provided to posthog.capture');
            return;
        }
        var isBot = !this.config.opt_out_useragent_filter && this._is_bot();
        var shouldDropBotEvent = isBot && !this.config.__preview_capture_bot_pageviews;
        // We drop bot events unless the preview flag to send bot pageviews is enabled
        // or the user has explicitly opted out of useragent filtering
        if (shouldDropBotEvent) {
            return;
        }
        var clientRateLimitContext = !(options === null || options === void 0 ? void 0 : options.skip_client_rate_limiting)
            ? this.rateLimiter.clientRateLimitContext()
            : undefined;
        if (clientRateLimitContext === null || clientRateLimitContext === void 0 ? void 0 : clientRateLimitContext.isRateLimited) {
            logger_1.logger.critical('This capture call is ignored due to client rate limiting.');
            return;
        }
        if ((properties === null || properties === void 0 ? void 0 : properties.$current_url) && !(0, core_1.isString)(properties === null || properties === void 0 ? void 0 : properties.$current_url)) {
            logger_1.logger.error('Invalid `$current_url` property provided to `posthog.capture`. Input must be a string. Ignoring provided value.');
            properties === null || properties === void 0 ? true : delete properties.$current_url;
        }
        if (event_name === '$exception' && !(options === null || options === void 0 ? void 0 : options._originatedFromCaptureException)) {
            logger_1.logger.warn("Using `posthog.capture('$exception')` is unreliable because it does not attach required metadata. Use `posthog.captureException(error)` instead, which attaches required metadata automatically.");
        }
        // update persistence
        this.sessionPersistence.update_search_keyword();
        // The initial campaign/referrer props need to be stored in the regular persistence, as they are there to mimic
        // the person-initial props. The non-initial versions are stored in the sessionPersistence, as they are sent
        // with every event and used by the session table to create session-initial props.
        if (this.config.save_campaign_params) {
            this.sessionPersistence.update_campaign_params();
        }
        if (this.config.save_referrer) {
            this.sessionPersistence.update_referrer_info();
        }
        if (this.config.save_campaign_params || this.config.save_referrer) {
            this.persistence.set_initial_person_info();
        }
        var systemTime = new Date();
        var timestamp = (options === null || options === void 0 ? void 0 : options.timestamp) || systemTime;
        // codeql[js/insecure-randomness] Event UUIDs are identifiers for deduplication, not secrets.
        var uuid = (0, core_1.getEventUuid)(options === null || options === void 0 ? void 0 : options.uuid, uuidv7_1.uuidv7);
        var data = {
            uuid: uuid,
            event: event_name,
            properties: this.calculateEventProperties(event_name, properties || {}, timestamp, uuid),
        };
        // Route pageviews to $bot_pageview when bot detected and preview flag enabled
        if (event_name === constants_1.EVENT_PAGEVIEW && this.config.__preview_capture_bot_pageviews && isBot) {
            data.event = '$bot_pageview';
            // While it's obvious that a $bot_pageview is (likely) from a bot, we explicitly set $browser_type
            // to make it easy to filter and test bot pageviews in the product
            data.properties.$browser_type = 'bot';
        }
        if (clientRateLimitContext) {
            data.properties['$lib_rate_limit_remaining_tokens'] = clientRateLimitContext.remainingTokens;
        }
        // When the server gates this project into minimal $feature_flag_called events and the
        // evaluated flag is not linked to an experiment, the fully merged properties are rebuilt
        // from the strict allowlist below (after the last SDK-added property) so super properties
        // and the context envelope are structurally excluded. Any missing signal falls back to
        // the full event.
        var shouldSendMinimalFlagCalledEvent = event_name === '$feature_flag_called' &&
            data.properties['$feature_flag_has_experiment'] === false &&
            this.get_property(constants_1.PERSISTENCE_MINIMAL_FLAG_CALLED_EVENTS) === true;
        var setProperties = options === null || options === void 0 ? void 0 : options.$set;
        if (setProperties && !shouldSendMinimalFlagCalledEvent) {
            data.$set = options === null || options === void 0 ? void 0 : options.$set;
        }
        var unsetProperties = options === null || options === void 0 ? void 0 : options.$unset;
        if (unsetProperties) {
            data.$unset = unsetProperties;
        }
        // $groupidentify doesn't process person $set_once on the server, so don't mark
        // initial person props as sent. This ensures they're included with subsequent
        // $identify calls.
        var markSetOnceAsSent = event_name !== constants_1.EVENT_GROUPIDENTIFY;
        // $identify should always include initial props because it creates/merges persons
        // and may be processed before earlier anonymous events on the server
        var forceIncludeInitialProps = event_name === constants_1.EVENT_IDENTIFY;
        // Minimal flag-called events must not carry $set_once. Skipping the calculation (rather
        // than dropping its result) avoids marking the initial person props as sent, so they
        // still go out with the next full event.
        var setOnceProperties = shouldSendMinimalFlagCalledEvent
            ? undefined
            : this._calculate_set_once_properties(options === null || options === void 0 ? void 0 : options.$set_once, markSetOnceAsSent, forceIncludeInitialProps);
        if (setOnceProperties) {
            data.$set_once = setOnceProperties;
        }
        if (!(options === null || options === void 0 ? void 0 : options._noTruncate)) {
            data = (0, general_utils_1._copyAndTruncateStrings)(data, this.config.properties_string_max_length);
        }
        data.timestamp = timestamp;
        if (!(0, core_1.isUndefined)(options === null || options === void 0 ? void 0 : options.timestamp)) {
            data.properties['$event_time_override_provided'] = true;
            data.properties['$event_time_override_system_time'] = systemTime;
        }
        // before_send runs after this filter and may deliberately re-add stripped properties;
        // the SDK itself must not enrich beyond allowlisted keys past this point.
        if (shouldSendMinimalFlagCalledEvent) {
            data.properties = (0, core_1.minimizeFlagCalledEventProperties)(data.properties, FLAG_CALLED_TRANSPORT_PROPERTY_KEYS);
        }
        if (event_name === posthog_surveys_types_1.SurveyEventName.DISMISSED || event_name === posthog_surveys_types_1.SurveyEventName.SENT) {
            var surveyId = properties === null || properties === void 0 ? void 0 : properties[posthog_surveys_types_1.SurveyEventProperties.SURVEY_ID];
            var surveyIteration = properties === null || properties === void 0 ? void 0 : properties[posthog_surveys_types_1.SurveyEventProperties.SURVEY_ITERATION];
            (0, survey_utils_1.setSurveySeenOnLocalStorage)({ id: surveyId, current_iteration: surveyIteration });
            data.$set = __assign(__assign({}, data.$set), (_a = {}, _a[(0, survey_utils_1.getSurveyInteractionProperty)({ id: surveyId, current_iteration: surveyIteration }, event_name === posthog_surveys_types_1.SurveyEventName.SENT ? 'responded' : 'dismissed')] = true, _a));
        }
        else if (event_name === posthog_surveys_types_1.SurveyEventName.SHOWN) {
            data.$set = __assign(__assign({}, data.$set), (_b = {}, _b[posthog_surveys_types_1.SurveyEventProperties.SURVEY_LAST_SEEN_DATE] = new Date().toISOString(), _b));
        }
        if (event_name === posthog_product_tours_types_1.ProductTourEventName.SHOWN) {
            var tourType = properties === null || properties === void 0 ? void 0 : properties[posthog_product_tours_types_1.ProductTourEventProperties.TOUR_TYPE];
            if (tourType) {
                data.$set = __assign(__assign({}, data.$set), (_c = {}, _c["".concat(posthog_product_tours_types_1.ProductTourEventProperties.TOUR_LAST_SEEN_DATE, "/").concat(tourType)] = new Date().toISOString(), _c));
            }
        }
        // Top-level $set overriding values from the one from properties is taken from the plugin-server normalizeEvent
        // This doesn't handle $set_once, because posthog-people doesn't either
        var finalSet = __assign(__assign({}, data.properties['$set']), data['$set']);
        if (!(0, core_1.isEmptyObject)(finalSet)) {
            this.setPersonPropertiesForFlags(finalSet);
        }
        if (!(0, core_1.isNullish)(this.config.before_send)) {
            var beforeSendResult = this._runBeforeSend(data);
            if (!beforeSendResult) {
                return;
            }
            else {
                data = beforeSendResult;
                // codeql[js/insecure-randomness] Event UUIDs are identifiers for deduplication, not secrets.
                data.uuid = (0, core_1.getEventUuid)(data.uuid, uuidv7_1.uuidv7);
            }
        }
        this._internalEventEmitter.emit('eventCaptured', data);
        var url = (_d = options === null || options === void 0 ? void 0 : options._url) !== null && _d !== void 0 ? _d : this.requestRouter.endpointFor('api', this.analyticsDefaultEndpoint);
        var isSessionRecording = (options === null || options === void 0 ? void 0 : options._batchKey) === 'recordings' || /\/s\/(?:\?|$)/.test(url);
        var requestOptions = {
            method: 'POST',
            url: url,
            data: data,
            compression: 'best-available',
            timestampMode: isSessionRecording ? 'body' : 'capture-body',
            batchKey: options === null || options === void 0 ? void 0 : options._batchKey,
            transport: options === null || options === void 0 ? void 0 : options.transport,
        };
        if (this.config.request_batching && (!options || (options === null || options === void 0 ? void 0 : options._batchKey)) && !(options === null || options === void 0 ? void 0 : options.send_instantly)) {
            this._requestQueue.enqueue(requestOptions);
        }
        else {
            this._send_retriable_request(requestOptions);
        }
        return data;
    };
    PostHog.prototype._addCaptureHook = function (callback) {
        return this.on('eventCaptured', function (data) { return callback(data.event, data); });
    };
    /**
     * This method is used internally to calculate the event properties before sending it to PostHog. It can also be
     * used by integrations (e.g. Segment) to enrich events with PostHog properties before sending them to Segment,
     * which is required for some PostHog products to work correctly. (e.g. to have a correct $session_id property).
     *
     * @param {String} eventName The name of the event. This can be anything the user does - 'Button Click', 'Sign Up', '$pageview', etc.
     * @param {Object} eventProperties The properties to include with the event.
     * @param {Date} [timestamp] The timestamp of the event, e.g. for calculating time on page. If not set, it'll automatically be set to the current time.
     * @param {String} [uuid] The uuid of the event, e.g. for storing the $pageview ID.
     * @param {Boolean} [readOnly] Set this if you do not intend to actually send the event, and therefore do not want to update internal state e.g. session timeout
     *
     * @internal
     */
    PostHog.prototype.calculateEventProperties = function (eventName, eventProperties, timestamp, uuid, readOnly) {
        var _a;
        timestamp = timestamp || new Date();
        if (!this.persistence || !this.sessionPersistence) {
            return eventProperties;
        }
        // set defaults
        var startTimestamp = readOnly ? undefined : this.persistence.remove_event_timer(eventName);
        var properties = __assign({}, eventProperties);
        properties['token'] = this.config.token;
        properties['$config_defaults'] = this.config.defaults;
        if (this._inCookielessMode()) {
            // Set a flag to tell the plugin server to use cookieless server hash mode
            properties[constants_1.COOKIELESS_MODE_FLAG_PROPERTY] = true;
        }
        if (eventName === '$snapshot') {
            var persistenceProps = __assign(__assign({}, this.persistence.properties()), this.sessionPersistence.properties());
            properties['distinct_id'] = persistenceProps.distinct_id;
            if (
            // we spotted one customer that was managing to send `false` for ~9k events a day
            !((0, core_1.isString)(properties['distinct_id']) || (0, core_1.isNumber)(properties['distinct_id'])) ||
                (0, core_1.isEmptyString)(properties['distinct_id'])) {
                logger_1.logger.error('Invalid distinct_id for replay event. This indicates a bug in your implementation');
            }
            return properties;
        }
        var infoProperties = (0, event_utils_1.getEventProperties)(this.config.mask_personal_data_properties, this.config.custom_personal_data_properties, this.config.detect_google_search_app, this.config.disable_capture_url_hashes);
        if (this.sessionManager) {
            var _b = this.sessionManager.checkAndGetSessionAndWindowId(readOnly, timestamp.getTime()), sessionId = _b.sessionId, windowId = _b.windowId;
            properties['$session_id'] = sessionId;
            properties['$window_id'] = windowId;
        }
        if (this.sessionPropsManager) {
            (0, general_utils_1.extend)(properties, this.sessionPropsManager.getSessionProps());
        }
        try {
            if (this.sessionRecording) {
                (0, general_utils_1.extend)(properties, this.sessionRecording.sdkDebugProperties);
            }
            properties['$sdk_debug_retry_queue_size'] = (_a = this._retryQueue) === null || _a === void 0 ? void 0 : _a.length;
        }
        catch (e) {
            properties['$sdk_debug_error_capturing_properties'] = String(e);
        }
        if (this.requestRouter.region === request_router_1.RequestRouterRegion.CUSTOM) {
            properties['$lib_custom_api_host'] = this.config.api_host;
        }
        var pageviewProperties;
        if (eventName === constants_1.EVENT_PAGEVIEW && !readOnly) {
            pageviewProperties = this.pageViewManager.doPageView(timestamp, uuid);
        }
        else if (eventName === constants_1.EVENT_PAGELEAVE && !readOnly) {
            pageviewProperties = this.pageViewManager.doPageLeave(timestamp);
        }
        else {
            pageviewProperties = this.pageViewManager.doEvent();
        }
        properties = (0, general_utils_1.extend)(properties, pageviewProperties);
        if (eventName === constants_1.EVENT_PAGEVIEW && globals_1.document) {
            properties['title'] = globals_1.document.title;
        }
        // set $duration if time_event was previously called for this event
        if (!(0, core_1.isUndefined)(startTimestamp)) {
            var duration_in_ms = timestamp.getTime() - startTimestamp;
            properties['$duration'] = parseFloat((duration_in_ms / 1000).toFixed(3));
        }
        // this is only added when this.config.opt_out_useragent_filter is true,
        // or it would always add "browser"
        if (globals_1.userAgent && this.config.opt_out_useragent_filter) {
            properties['$browser_type'] = this._is_bot() ? 'bot' : 'browser';
        }
        // note: extend writes to the first object, so lets make sure we
        // don't write to the persistence properties object and info
        // properties object by passing in a new object
        // $referrer / $referring_domain are written to the sessionPersistence from document.referrer
        // on every capture, and sessionPersistence is merged after the regular persistence below, so
        // they normally win. When the user has explicitly set one of these via posthog.register()
        // (which writes to the regular persistence), let that value win instead. Resolve the
        // precedence per property so a single registered key does not suppress the other, and a
        // stale session value cannot shadow the registered one. Without this an SPA or iframe
        // reports document.referrer (the iframe's own origin) rather than the registered value.
        var persistenceProperties = this.persistence.properties();
        var sessionPersistenceProperties = this.sessionPersistence.properties();
        (0, general_utils_1.each)(['$referrer', '$referring_domain'], function (referrerKey) {
            if (referrerKey in persistenceProperties) {
                delete sessionPersistenceProperties[referrerKey];
            }
        });
        // update properties with pageview info and super-properties
        properties = (0, general_utils_1.extend)({}, infoProperties, persistenceProperties, sessionPersistenceProperties, properties);
        properties['$is_identified'] = this._isIdentified();
        if ((0, core_1.isArray)(this.config.property_denylist)) {
            (0, general_utils_1.each)(this.config.property_denylist, function (denylisted_prop) {
                delete properties[denylisted_prop];
            });
        }
        else {
            logger_1.logger.error(DENYLIST_INVALID +
                this.config.property_denylist +
                ' or property_blacklist config: ' +
                this.config.property_blacklist);
        }
        var sanitize_properties = this.config.sanitize_properties;
        if (sanitize_properties) {
            logger_1.logger.error(SANITIZE_DEPRECATED);
            properties = sanitize_properties(properties, eventName);
        }
        // add person processing flag as very last step, so it cannot be overridden
        var hasPersonProcessing = this._hasPersonProcessing();
        properties['$process_person_profile'] = hasPersonProcessing;
        // if the event has person processing, ensure that all future events will too, even if the setting changes
        if (hasPersonProcessing && !readOnly) {
            this._requirePersonProcessing('_calculate_event_properties');
        }
        return properties;
    };
    /**
     * Add additional set_once properties to the event when creating a person profile. This allows us to create the
     * profile with mostly-accurate properties, despite earlier events not setting them. We do this by storing them in
     * persistence.
     * @param dataSetOnce
     * @param markAsSent - if true, marks the properties as sent so they won't be included in future events.
     *                     Set to false for events like $groupidentify where the server doesn't process person props.
     * @param forceIncludeInitialProps - if true, include initial person props even if they've already been sent.
     *                                   Used for $identify which creates/merges persons and may be processed out of order.
     */
    PostHog.prototype._calculate_set_once_properties = function (dataSetOnce, markAsSent, forceIncludeInitialProps) {
        var _a;
        if (markAsSent === void 0) { markAsSent = true; }
        if (forceIncludeInitialProps === void 0) { forceIncludeInitialProps = false; }
        if (!this.persistence || !this._hasPersonProcessing()) {
            return dataSetOnce;
        }
        if (this._personProcessingSetOncePropertiesSent && !forceIncludeInitialProps) {
            // We only need to send these properties once. Sending them with later events would be redundant and would
            // just require extra work on the server to process them.
            return dataSetOnce;
        }
        // if we're an identified person, send initial params with every event
        var initialProps = this.persistence.get_initial_props();
        var sessionProps = (_a = this.sessionPropsManager) === null || _a === void 0 ? void 0 : _a.getSetOnceProps();
        var setOnceProperties = (0, general_utils_1.extend)({}, initialProps, sessionProps || {}, dataSetOnce || {});
        var sanitize_properties = this.config.sanitize_properties;
        if (sanitize_properties) {
            logger_1.logger.error(SANITIZE_DEPRECATED);
            setOnceProperties = sanitize_properties(setOnceProperties, '$set_once');
        }
        if (markAsSent) {
            this._personProcessingSetOncePropertiesSent = true;
        }
        if ((0, core_1.isEmptyObject)(setOnceProperties)) {
            return undefined;
        }
        return setOnceProperties;
    };
    /**
     * Registers super properties that are included with all events.
     *
     * @remarks
     * Super properties are stored in persistence and automatically added to every event you capture.
     * These values will overwrite any existing super properties with the same keys.
     *
     * @example
     * ```js
     * // register a single property
     * posthog.register({ plan: 'premium' })
     * ```
     *
     * {@label Capture}
     *
     * @example
     * ```js
     * // register multiple properties
     * posthog.register({
     *     email: 'user@example.com',
     *     account_type: 'business',
     *     signup_date: '2023-01-15'
     * })
     * ```
     *
     * @example
     * ```js
     * // register with custom expiration
     * posthog.register({ campaign: 'summer_sale' }, 7) // expires in 7 days
     * ```
     *
     * @public
     *
     * @param {Object} properties properties to store about the user
     * @param {Number} [days] How many days since the user's last visit to store the super properties
     */
    PostHog.prototype.register = function (properties, days) {
        var _a;
        (_a = this.persistence) === null || _a === void 0 ? void 0 : _a.register(properties, days);
    };
    /**
     * Registers super properties only if they haven't been set before.
     *
     * @remarks
     * Unlike `register()`, this method will not overwrite existing super properties.
     * Use this for properties that should only be set once, like signup date or initial referrer.
     *
     * {@label Capture}
     *
     * @example
     * ```js
     * // register once-only properties
     * posthog.register_once({
     *     first_login_date: new Date().toISOString(),
     *     initial_referrer: document.referrer
     * })
     * ```
     *
     * @example
     * ```js
     * // override existing value if it matches default
     * posthog.register_once(
     *     { user_type: 'premium' },
     *     'unknown'  // overwrite if current value is 'unknown'
     * )
     * ```
     *
     * @public
     *
     * @param {Object} properties An associative array of properties to store about the user
     * @param {*} [default_value] Value to override if already set in super properties (ex: 'False') Default: 'None'
     * @param {Number} [days] How many days since the users last visit to store the super properties
     */
    PostHog.prototype.register_once = function (properties, default_value, days) {
        var _a;
        (_a = this.persistence) === null || _a === void 0 ? void 0 : _a.register_once(properties, default_value, days);
    };
    /**
     * Registers super properties for the current session only.
     *
     * @remarks
     * Session super properties are automatically added to all events during the current browser session.
     * Unlike regular super properties, these are cleared when the session ends and are stored in sessionStorage.
     *
     * {@label Capture}
     *
     * @example
     * ```js
     * // register session-specific properties
     * posthog.register_for_session({
     *     current_page_type: 'checkout',
     *     ab_test_variant: 'control'
     * })
     * ```
     *
     * @example
     * ```js
     * // register properties for user flow tracking
     * posthog.register_for_session({
     *     selected_plan: 'pro',
     *     completed_steps: 3,
     *     flow_id: 'signup_flow_v2'
     * })
     * ```
     *
     * @public
     *
     * @param {Object} properties An associative array of properties to store about the user
     */
    PostHog.prototype.register_for_session = function (properties) {
        var _a;
        (_a = this.sessionPersistence) === null || _a === void 0 ? void 0 : _a.register(properties);
    };
    /**
     * Removes a super property from persistent storage.
     *
     * @remarks
     * This will stop the property from being automatically included in future events.
     * The property will be permanently removed from the user's profile.
     *
     * {@label Capture}
     *
     * @example
     * ```js
     * // remove a super property
     * posthog.unregister('plan_type')
     * ```
     *
     * @public
     *
     * @param {String} property The name of the super property to remove
     */
    PostHog.prototype.unregister = function (property) {
        var _a;
        (_a = this.persistence) === null || _a === void 0 ? void 0 : _a.unregister(property);
    };
    /**
     * Removes a session super property from the current session.
     *
     * @remarks
     * This will stop the property from being automatically included in future events for this session.
     * The property is removed from sessionStorage.
     *
     * {@label Capture}
     *
     * @example
     * ```js
     * // remove a session property
     * posthog.unregister_for_session('current_flow')
     * ```
     *
     * @public
     *
     * @param {String} property The name of the session super property to remove
     */
    PostHog.prototype.unregister_for_session = function (property) {
        var _a;
        (_a = this.sessionPersistence) === null || _a === void 0 ? void 0 : _a.unregister(property);
    };
    PostHog.prototype._register_single = function (prop, value) {
        var _a;
        this.register((_a = {}, _a[prop] = value, _a));
    };
    /**
     * Gets the value of a feature flag for the current user.
     *
     * @remarks
     * Returns the feature flag value which can be a boolean, string, or undefined.
     * Supports multivariate flags that can return custom string values.
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * // check boolean flag
     * if (posthog.getFeatureFlag('new-feature')) {
     *     // show new feature
     * }
     * ```
     *
     * @example
     * ```js
     * // check multivariate flag
     * const variant = posthog.getFeatureFlag('button-color')
     * if (variant === 'red') {
     *     // show red button
     * }
     * ```
     *
     * @public
     *
     * @param {string} key Key of the feature flag.
     * @param {FeatureFlagOptions} [options] Optional lookup settings. If `{ send_event: false }`, we won't send a `$feature_flag_called` event to PostHog. If `{ fresh: true }`, we won't return cached values from localStorage - only values loaded from the server.
     * @returns {boolean | string | undefined} The feature flag value, or undefined if the flag is unavailable.
     */
    PostHog.prototype.getFeatureFlag = function (key, options) {
        var _a;
        return (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.getFeatureFlag(key, options);
    };
    /**
     * Get feature flag payload value matching key for user (supports multivariate flags).
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * const betaFeature = posthog.getFeatureFlagResult('beta-feature')
     * if (betaFeature?.variant === 'some-value') {
     *      const someValue = betaFeature?.payload
     *      // do something
     * }
     * ```
     *
     * @public
     *
     * @deprecated Use `getFeatureFlagResult()` instead
     *
     * @param {string} key Key of the feature flag.
     * @returns {JsonType} The feature flag payload.
     */
    PostHog.prototype.getFeatureFlagPayload = function (key) {
        var _a;
        return (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.getFeatureFlagPayload(key);
    };
    /**
     * Get a feature flag evaluation result including both the flag value and payload.
     *
     * By default, this method emits the `$feature_flag_called` event.
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * const result = posthog.getFeatureFlagResult('my-flag')
     * if (result?.enabled) {
     *     console.log('Flag is enabled with payload:', result.payload)
     * }
     * ```
     *
     * @example
     * ```js
     * // multivariate flag
     * const result = posthog.getFeatureFlagResult('button-color')
     * if (result?.variant === 'red') {
     *     showRedButton(result.payload)
     * }
     * ```
     *
     * @public
     *
     * @param {string} key Key of the feature flag.
     * @param {Object} [options] Options for the feature flag lookup.
     * @param {boolean} [options.send_event=true] If false, won't send the $feature_flag_called event.
     * @param {boolean} [options.fresh=false] If true, won't return cached values from localStorage - only values loaded from the server.
     * @returns {FeatureFlagResult | undefined} The feature flag result including key, enabled, variant, and payload.
     */
    PostHog.prototype.getFeatureFlagResult = function (key, options) {
        var _a;
        return (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.getFeatureFlagResult(key, options);
    };
    /**
     * Returns all currently cached feature flags as `FeatureFlagResult`s. This is a synchronous read of
     * the flags from the last load (no network request); call `reloadFeatureFlags()` first to refresh.
     * Unlike `getFeatureFlag()`, it does not send a `$feature_flag_called` event.
     *
     * @returns {FeatureFlagResult[]} All loaded flags, or an empty array if none are loaded.
     */
    PostHog.prototype.getAllFeatureFlags = function () {
        var _a, _b;
        return (_b = (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.getAllFeatureFlags()) !== null && _b !== void 0 ? _b : [];
    };
    PostHog.prototype.isFeatureEnabled = function (key, options) {
        var _a, _b;
        return (_b = (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.isFeatureEnabled(key, options)) !== null && _b !== void 0 ? _b : options === null || options === void 0 ? void 0 : options.defaultValue;
    };
    /**
     * Feature flag values are cached. If something has changed with your user and you'd like to refetch their flag values, call this method.
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * posthog.reloadFeatureFlags()
     * ```
     *
     * @public
     */
    PostHog.prototype.reloadFeatureFlags = function () {
        var _a;
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.reloadFeatureFlags();
    };
    /**
     * Manually update feature flag values without making a network request.
     *
     * This is useful when you have feature flag values from an external source
     * (e.g., server-side evaluation, edge middleware) and want to inject them
     * into the client SDK.
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * // Replace all flags with server-evaluated values
     * posthog.updateFlags({
     *   'my-flag': true,
     *   'my-experiment': 'variant-a'
     * })
     *
     * // Merge with existing flags (update only specified flags)
     * posthog.updateFlags(
     *   { 'my-flag': true },
     *   undefined,
     *   { merge: true }
     * )
     *
     * // With payloads
     * posthog.updateFlags(
     *   { 'my-flag': true },
     *   { 'my-flag': { some: 'data' } }
     * )
     * ```
     *
     * @param flags - An object mapping flag keys to their values (boolean or string variant)
     * @param payloads - Optional object mapping flag keys to their JSON payloads
     * @param options - Optional settings. Use `{ merge: true }` to merge with existing flags instead of replacing.
     * @public
     */
    PostHog.prototype.updateFlags = function (flags, payloads, options) {
        var _a;
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.updateFlags(flags, payloads, options);
    };
    /**
     * Opt the user in or out of an early access feature. [Learn more in the docs](/docs/feature-flags/early-access-feature-management#option-2-custom-implementation)
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * const toggleBeta = (betaKey) => {
     *   if (activeBetas.some(
     *     beta => beta.flagKey === betaKey
     *   )) {
     *     posthog.updateEarlyAccessFeatureEnrollment(
     *       betaKey,
     *       false
     *     )
     *     setActiveBetas(
     *       prevActiveBetas => prevActiveBetas.filter(
     *         item => item.flagKey !== betaKey
     *       )
     *     );
     *     return
     *   }
     *
     *   posthog.updateEarlyAccessFeatureEnrollment(
     *     betaKey,
     *     true
     *   )
     *   setInactiveBetas(
     *     prevInactiveBetas => prevInactiveBetas.filter(
     *       item => item.flagKey !== betaKey
     *     )
     *   );
     * }
     *
     * const registerInterest = (featureKey) => {
     *   posthog.updateEarlyAccessFeatureEnrollment(
     *     featureKey,
     *     true
     *   )
     *   // Update UI to show user has registered
     * }
     * ```
     *
     * @public
     *
     * @param {String} key The key of the feature flag to update.
     * @param {Boolean} isEnrolled Whether the user is enrolled in the feature.
     * @param {String} [stage] The stage of the feature flag to update.
     */
    PostHog.prototype.updateEarlyAccessFeatureEnrollment = function (key, isEnrolled, stage) {
        var _a;
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.updateEarlyAccessFeatureEnrollment(key, isEnrolled, stage);
    };
    /**
     * Get the list of early access features. To check enrollment status, use `isFeatureEnabled`. [Learn more in the docs](/docs/feature-flags/early-access-feature-management#option-2-custom-implementation)
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * const posthog = usePostHog()
     * const activeFlags = useActiveFeatureFlags()
     *
     * const [activeBetas, setActiveBetas] = useState([])
     * const [inactiveBetas, setInactiveBetas] = useState([])
     * const [comingSoonFeatures, setComingSoonFeatures] = useState([])
     *
     * useEffect(() => {
     *   posthog.getEarlyAccessFeatures((features) => {
     *     // Filter features by stage
     *     const betaFeatures = features.filter(feature => feature.stage === 'beta')
     *     const conceptFeatures = features.filter(feature => feature.stage === 'concept')
     *
     *     setComingSoonFeatures(conceptFeatures)
     *
     *     if (!activeFlags || activeFlags.length === 0) {
     *       setInactiveBetas(betaFeatures)
     *       return
     *     }
     *
     *     const activeBetas = betaFeatures.filter(
     *             beta => activeFlags.includes(beta.flagKey)
     *         );
     *     const inactiveBetas = betaFeatures.filter(
     *             beta => !activeFlags.includes(beta.flagKey)
     *         );
     *     setActiveBetas(activeBetas)
     *     setInactiveBetas(inactiveBetas)
     *   }, true, ['concept', 'beta'])
     * }, [activeFlags])
     * ```
     *
     * @public
     *
     * @param {Function} callback The callback function will be called when the early access features are loaded.
     * @param {Boolean} [force_reload] Whether to force a reload of the early access features.
     * @param {String[]} [stages] The stages of the early access features to load.
     */
    PostHog.prototype.getEarlyAccessFeatures = function (callback, force_reload, stages) {
        var _a;
        if (force_reload === void 0) { force_reload = false; }
        return (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.getEarlyAccessFeatures(callback, force_reload, stages);
    };
    /**
     * Exposes a set of events that PostHog will emit.
     * e.g. `eventCaptured` is emitted immediately before trying to send an event
     *
     * Unlike  `onFeatureFlags` and `onSessionId` these are not called when the
     * listener is registered, the first callback will be the next event
     * _after_ registering a listener
     *
     * Available events:
     * - `eventCaptured`: Emitted immediately before trying to send an event
     * - `featureFlagsReloading`: Emitted when feature flags are being reloaded (e.g. after `identify()`, `group()`, or `reloadFeatureFlags()`)
     *
     * {@label Capture}
     *
     * @example
     * ```js
     * posthog.on('eventCaptured', (event) => {
     *   console.log(event)
     * })
     * ```
     *
     * @example
     * ```js
     * // Track when feature flags are reloading to show a loading state
     * posthog.on('featureFlagsReloading', () => {
     *   console.log('Feature flags are being reloaded...')
     * })
     * ```
     *
     * @public
     *
     * @param {String} event The event to listen for.
     * @param {Function} cb The callback function to call when the event is emitted.
     * @returns {Function} A function that can be called to unsubscribe the listener.
     */
    PostHog.prototype.on = function (event, cb) {
        return this._internalEventEmitter.on(event, cb);
    };
    /**
     * Register an event listener that runs when feature flags become available or when they change.
     * If there are flags, the listener is called immediately in addition to being called on future changes.
     * Note that this is not called only when we fetch feature flags from the server, but also when they change in the browser.
     *
     * {@label Feature flags}
     *
     * @public
     *
     * @example
     * ```js
     * posthog.onFeatureFlags(function(featureFlags, featureFlagsVariants, { errorsLoading }) {
     *     // do something
     * })
     * ```
     *
     * @param callback - The callback function will be called once the feature flags are ready or when they are updated.
     *                   It'll return a list of feature flags enabled for the user, the variants,
     *                   and also a context object indicating whether we succeeded to fetch the flags or not.
     * @returns A function that can be called to unsubscribe the listener. Used by `useEffect` when the component unmounts.
     */
    PostHog.prototype.onFeatureFlags = function (callback) {
        if (!this.featureFlags) {
            callback([], {}, { errorsLoading: true });
            return function () { };
        }
        return this.featureFlags.onFeatureFlags(callback);
    };
    /**
     * Register an event listener that runs when surveys are loaded.
     *
     * Callback parameters:
     * - surveys: Survey[]: An array containing all survey objects fetched from PostHog using the getSurveys method
     * - context: { isLoaded: boolean, error?: string }: An object indicating if the surveys were loaded successfully
     *
     * {@label Surveys}
     *
     * @example
     * ```js
     * posthog.onSurveysLoaded((surveys, context) => { // do something })
     * ```
     *
     * @param {SurveyCallback} callback The callback function will be called when surveys are loaded or updated.
     * @returns A function that can be called to unsubscribe the listener.
     */
    PostHog.prototype.onSurveysLoaded = function (callback) {
        if (!this.surveys) {
            callback([], { isLoaded: false, error: SURVEYS_NOT_AVAILABLE });
            return function () { };
        }
        return this.surveys.onSurveysLoaded(callback);
    };
    /**
     * Register an event listener that runs whenever the session id or window id change.
     * If there is already a session id, the listener is called immediately in addition to being called on future changes.
     *
     * Can be used, for example, to sync the PostHog session id with a backend session.
     *
     * {@label Identification}
     *
     * @public
     *
     * @example
     * ```js
     * posthog.onSessionId(function(sessionId, windowId) { // do something })
     * ```
     *
     * @param {Function} [callback] The callback function will be called once a session id is present or when it or the window id are updated.
     * @returns {Function} A function that can be called to unsubscribe the listener. E.g. Used by `useEffect` when the component unmounts.
     */
    PostHog.prototype.onSessionId = function (callback) {
        var _a, _b;
        return (_b = (_a = this.sessionManager) === null || _a === void 0 ? void 0 : _a.onSessionId(callback)) !== null && _b !== void 0 ? _b : (function () { });
    };
    /**
     * Get list of all surveys.
     *
     * {@label Surveys}
     *
     * @example
     * ```js
     * function callback(surveys, context) {
     *   // do something
     * }
     *
     * posthog.getSurveys(callback, false)
     * ```
     *
     * @public
     *
     * @param {SurveyCallback} callback Function that receives the array of surveys.
     * @param {boolean} [forceReload] Optional boolean to force an API call for updated surveys.
     */
    PostHog.prototype.getSurveys = function (callback, forceReload) {
        if (forceReload === void 0) { forceReload = false; }
        this.surveys
            ? this.surveys.getSurveys(callback, forceReload)
            : callback([], { isLoaded: false, error: SURVEYS_NOT_AVAILABLE });
    };
    /**
     * Get surveys that should be enabled for the current user. See [fetching surveys documentation](/docs/surveys/implementing-custom-surveys#fetching-surveys-manually) for more details.
     *
     * {@label Surveys}
     *
     * @example
     * ```js
     * posthog.getActiveMatchingSurveys((surveys) => {
     *      // do something
     * })
     * ```
     *
     * @public
     *
     * @param {SurveyCallback} callback The callback function will be called when the surveys are loaded or updated.
     * @param {boolean} [forceReload] Whether to force a reload of the surveys.
     */
    PostHog.prototype.getActiveMatchingSurveys = function (callback, forceReload) {
        if (forceReload === void 0) { forceReload = false; }
        this.surveys
            ? this.surveys.getActiveMatchingSurveys(callback, forceReload)
            : callback([], { isLoaded: false, error: SURVEYS_NOT_AVAILABLE });
    };
    /**
     * Although we recommend using popover surveys and display conditions,
     * if you want to show surveys programmatically without setting up all
     * the extra logic needed for API surveys, you can render surveys
     * programmatically with the renderSurvey method.
     *
     * This takes a survey ID and an HTML selector to render an unstyled survey.
     *
     * {@label Surveys}
     *
     * @example
     * ```js
     * posthog.renderSurvey(coolSurveyID, '#survey-container')
     * ```
     *
     * @deprecated Use displaySurvey instead - it's more complete and also supports popover surveys.
     *
     * @public
     *
     * @param {String} surveyId The ID of the survey to render.
     * @param {String} selector The selector of the HTML element to render the survey on.
     */
    PostHog.prototype.renderSurvey = function (surveyId, selector) {
        var _a;
        (_a = this.surveys) === null || _a === void 0 ? void 0 : _a.renderSurvey(surveyId, selector);
    };
    /**
     * Display a survey programmatically as either a popover or inline element.
     *
     * @param {string} surveyId The survey ID to display.
     * @param {DisplaySurveyOptions} [options] Display configuration. Defaults to a popover that respects dashboard conditions and delays.
     *
     * @example
     * ```js
     * // Display as popover (respects all conditions defined in the dashboard)
     * posthog.displaySurvey('survey-id-123')
     * ```
     *
     * @example
     * ```js
     * // Display inline in a specific element
     * posthog.displaySurvey('survey-id-123', {
     *   displayType: DisplaySurveyType.Inline,
     *   ignoreConditions: false,
     *   ignoreDelay: false,
     *   selector: '#survey-container'
     * })
     * ```
     *
     * @example
     * ```js
     * // Force display ignoring conditions and delays
     * posthog.displaySurvey('survey-id-123', {
     *   displayType: DisplaySurveyType.Popover,
     *   ignoreConditions: true,
     *   ignoreDelay: true
     * })
     * ```
     *
     * {@label Surveys}
     */
    PostHog.prototype.displaySurvey = function (surveyId, options) {
        var _a;
        if (options === void 0) { options = survey_utils_1.DEFAULT_DISPLAY_SURVEY_OPTIONS; }
        (_a = this.surveys) === null || _a === void 0 ? void 0 : _a.displaySurvey(surveyId, options);
    };
    /**
     * Cancels a pending survey that is waiting to be displayed (e.g., due to a popup delay).
     *
     * {@label Surveys}
     *
     * @param {string} surveyId The survey ID whose pending display should be cancelled.
     */
    PostHog.prototype.cancelPendingSurvey = function (surveyId) {
        var _a;
        (_a = this.surveys) === null || _a === void 0 ? void 0 : _a.cancelPendingSurvey(surveyId);
    };
    /**
     * Checks the feature flags associated with this Survey to see if the survey can be rendered.
     * This method is deprecated because it's synchronous and won't return the correct result if surveys are not loaded.
     * Use `canRenderSurveyAsync` instead.
     *
     * {@label Surveys}
     *
     * @public
     * @deprecated
     *
     * @param surveyId The ID of the survey to check.
     * @returns A SurveyRenderReason object indicating if the survey can be rendered.
     */
    PostHog.prototype.canRenderSurvey = function (surveyId) {
        var _a, _b;
        return ((_b = (_a = this.surveys) === null || _a === void 0 ? void 0 : _a.canRenderSurvey(surveyId)) !== null && _b !== void 0 ? _b : {
            visible: false,
            disabledReason: SURVEYS_NOT_AVAILABLE,
        });
    };
    /**
     * Checks the feature flags associated with this Survey to see if the survey can be rendered.
     *
     * {@label Surveys}
     *
     * @example
     * ```js
     * posthog.canRenderSurveyAsync(surveyId).then((result) => {
     *     if (result.visible) {
     *         // Survey can be rendered
     *         console.log('Survey can be rendered')
     *     } else {
     *         // Survey cannot be rendered
     *         console.log('Survey cannot be rendered:', result.disabledReason)
     *     }
     * })
     * ```
     *
     * @public
     *
     * @param surveyId The ID of the survey to check.
     * @param forceReload If true, the survey will be reloaded from the server, Default: false
     * @returns A SurveyRenderReason object indicating if the survey can be rendered.
     */
    PostHog.prototype.canRenderSurveyAsync = function (surveyId, forceReload) {
        var _a, _b;
        if (forceReload === void 0) { forceReload = false; }
        return ((_b = (_a = this.surveys) === null || _a === void 0 ? void 0 : _a.canRenderSurveyAsync(surveyId, forceReload)) !== null && _b !== void 0 ? _b : 
        // eslint-disable-next-line compat/compat
        Promise.resolve({ visible: false, disabledReason: SURVEYS_NOT_AVAILABLE }));
    };
    PostHog.prototype._validateIdentifyId = function (id) {
        if (!id || (0, core_1.isEmptyString)(id)) {
            logger_1.logger.critical('Unique user id has not been set in posthog.identify');
            return false;
        }
        if (id === constants_1.COOKIELESS_SENTINEL_VALUE) {
            logger_1.logger.critical("The string \"".concat(id, "\" was set in posthog.identify which indicates an error. This ID is only used as a sentinel value."));
            return false;
        }
        if ((0, core_1.isDistinctIdStringLike)(id) || ['undefined', 'null'].includes(id.toLowerCase())) {
            logger_1.logger.critical("The string \"".concat(id, "\" was set in posthog.identify which indicates an error. This ID should be unique to the user and not a hardcoded string."));
            return false;
        }
        return true;
    };
    /**
     * Associates a user with a unique identifier instead of an auto-generated ID.
     * Learn more about [identifying users](/docs/product-analytics/identify)
     *
     * {@label Identification}
     *
     * @remarks
     * By default, PostHog assigns each user a randomly generated `distinct_id`. Use this method to
     * replace that ID with your own unique identifier (like a user ID from your database).
     *
     * @example
     * ```js
     * // basic identification
     * posthog.identify('user_12345')
     * ```
     *
     * @example
     * ```js
     * // identify with user properties
     * posthog.identify('user_12345', {
     *     email: 'user@example.com',
     *     plan: 'premium'
     * })
     * ```
     *
     * @example
     * ```js
     * // identify with set and set_once properties
     * posthog.identify('user_12345',
     *     { last_login: new Date() },  // updates every time
     *     { signup_date: new Date() }  // sets only once
     * )
     * ```
     *
     * @public
     *
     * @param {String} [new_distinct_id] A string that uniquely identifies a user. If not provided, the distinct_id currently in the persistent store (cookie or localStorage) will be used.
     * @param {Object} [userPropertiesToSet] Optional: An associative array of properties to store about the user. Note: For feature flag evaluations, if the same key is present in the userPropertiesToSetOnce,
     *  it will be overwritten by the value in userPropertiesToSet.
     * @param {Object} [userPropertiesToSetOnce] Optional: An associative array of properties to store about the user. If property is previously set, this does not override that value.
     */
    PostHog.prototype.identify = function (new_distinct_id, userPropertiesToSet, userPropertiesToSetOnce) {
        var _a;
        if (!this.__loaded || !this.persistence) {
            return logger_1.logger.uninitializedWarning('posthog.identify');
        }
        if ((0, core_1.isNumber)(new_distinct_id)) {
            new_distinct_id = new_distinct_id.toString();
            logger_1.logger.warn('The first argument to posthog.identify was a number, but it should be a string. It has been converted to a string.');
        }
        if (!this._validateIdentifyId(new_distinct_id)) {
            return;
        }
        if (!this._requirePersonProcessing('posthog.identify')) {
            return;
        }
        var previous_distinct_id = this.get_distinct_id();
        this.register({ $user_id: new_distinct_id });
        if (!this.get_property(constants_1.DEVICE_ID)) {
            // The persisted distinct id might not actually be a device id at all
            // it might be a distinct id of the user from before
            var device_id = previous_distinct_id;
            this.register_once({
                $had_persisted_distinct_id: true,
                $device_id: device_id,
            }, '');
        }
        // if the previous distinct id had an alias stored, then we clear it
        if (new_distinct_id !== previous_distinct_id && new_distinct_id !== this.get_property(constants_1.ALIAS_ID_KEY)) {
            this.unregister(constants_1.ALIAS_ID_KEY);
            this.register({ distinct_id: new_distinct_id });
        }
        var isKnownAnonymous = (this.persistence.get_property(constants_1.USER_STATE) || constants_1.USER_STATE_ANONYMOUS) === constants_1.USER_STATE_ANONYMOUS;
        // send an $identify event any time the distinct_id is changing and the old ID is an anonymous ID
        // - logic on the server will determine whether or not to do anything with it.
        if (new_distinct_id !== previous_distinct_id && isKnownAnonymous) {
            this.persistence.set_property(constants_1.USER_STATE, constants_1.USER_STATE_IDENTIFIED);
            // Update current user properties
            this.setPersonPropertiesForFlags({ $set: userPropertiesToSet || {}, $set_once: userPropertiesToSetOnce || {} }, false);
            this.capture(constants_1.EVENT_IDENTIFY, {
                distinct_id: new_distinct_id,
                $anon_distinct_id: previous_distinct_id,
            }, { $set: userPropertiesToSet || {}, $set_once: userPropertiesToSetOnce || {} });
            this._cachedPersonProperties = (0, property_utils_1.getPersonPropertiesHash)(new_distinct_id, userPropertiesToSet, userPropertiesToSetOnce);
            // let the reload feature flag request know to send this previous distinct id
            // for flag consistency
            (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.setAnonymousDistinctId(previous_distinct_id);
        }
        else if (userPropertiesToSet || userPropertiesToSetOnce) {
            // If the distinct_id is not changing, but we have user properties to set, we can check if they have changed
            // and if so, send a $set event
            this.setPersonProperties(userPropertiesToSet, userPropertiesToSetOnce);
        }
        // Reload active feature flags if the user identity changes.
        // Note we don't reload this on property changes as these get processed async
        if (new_distinct_id !== previous_distinct_id) {
            this.reloadFeatureFlags();
            // also clear any stored flag calls
            this.unregister(constants_1.FLAG_CALL_REPORTED);
        }
    };
    /**
     * Sets properties on the person profile associated with the current `distinct_id`.
     * Learn more about [identifying users](/docs/product-analytics/identify)
     *
     * {@label Identification}
     *
     * @remarks
     * Updates user properties that are stored with the person profile in PostHog.
     * If `person_profiles` is set to `identified_only` and no profile exists, this will create one.
     *
     * @example
     * ```js
     * // set user properties
     * posthog.setPersonProperties({
     *     email: 'user@example.com',
     *     plan: 'premium'
     * })
     * ```
     *
     * @example
     * ```js
     * // set properties
     * posthog.setPersonProperties(
     *     { name: 'Max Hedgehog' },  // $set properties
     *     { initial_url: '/blog' }   // $set_once properties
     * )
     * ```
     *
     * @public
     *
     * @param {Object} [userPropertiesToSet] Optional: An associative array of properties to store about the user. Note: For feature flag evaluations, if the same key is present in the userPropertiesToSetOnce,
     *  it will be overwritten by the value in userPropertiesToSet.
     * @param {Object} [userPropertiesToSetOnce] Optional: An associative array of properties to store about the user. If property is previously set, this does not override that value.
     */
    PostHog.prototype.setPersonProperties = function (userPropertiesToSet, userPropertiesToSetOnce) {
        if (!userPropertiesToSet && !userPropertiesToSetOnce) {
            return;
        }
        if (!this._requirePersonProcessing('posthog.setPersonProperties')) {
            return;
        }
        var hash = (0, property_utils_1.getPersonPropertiesHash)(this.get_distinct_id(), userPropertiesToSet, userPropertiesToSetOnce);
        // if exactly this $set call has been sent before, don't send it again - determine based on hash of properties
        if (this._cachedPersonProperties === hash) {
            logger_1.logger.info('A duplicate setPersonProperties call was made with the same properties. It has been ignored.');
            return;
        }
        // Update current user properties
        this.setPersonPropertiesForFlags({ $set: userPropertiesToSet || {}, $set_once: userPropertiesToSetOnce || {} }, true);
        this.capture('$set', { $set: userPropertiesToSet || {}, $set_once: userPropertiesToSetOnce || {} });
        this._cachedPersonProperties = hash;
    };
    /**
     * Removes properties from the person profile associated with the current `distinct_id`.
     * Learn more about [identifying users](/docs/product-analytics/identify)
     *
     * {@label Identification}
     *
     * @remarks
     * Deletes the given person properties from the person profile in PostHog. This is the
     * counterpart to {@link setPersonProperties} — instead of hand-passing `$unset` inside a
     * `capture()` call, you can remove properties with a dedicated method.
     * If `person_profiles` is set to `never`, this call is ignored.
     *
     * @example
     * ```js
     * // remove a single property
     * posthog.unsetPersonProperties('plan')
     * ```
     *
     * @example
     * ```js
     * // remove multiple properties
     * posthog.unsetPersonProperties(['plan', 'email'])
     * ```
     *
     * @public
     *
     * @param {String|String[]} propertyNames The name (or names) of the person properties to remove.
     */
    PostHog.prototype.unsetPersonProperties = function (propertyNames) {
        var _a;
        var names = ((0, core_1.isArray)(propertyNames) ? propertyNames : [propertyNames]).filter(function (name) { return (0, core_1.isString)(name) && name.length > 0; });
        if (names.length === 0) {
            return;
        }
        if (!this._requirePersonProcessing('posthog.unsetPersonProperties')) {
            return;
        }
        // Remove the properties from the locally-stored person properties used for flag evaluation
        // so flags re-evaluate without the unset values, mirroring setPersonPropertiesForFlags.
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.unsetPersonPropertiesForFlags(names, true);
        this.capture('$set', { $unset: names });
        // Invalidate the setPersonProperties dedupe cache so a subsequent set of an unset
        // property is not skipped as a duplicate.
        this._cachedPersonProperties = null;
    };
    /**
     * Associates the user with a group for group-based analytics.
     * Learn more about [groups](/docs/product-analytics/group-analytics)
     *
     * {@label Identification}
     *
     * @remarks
     * Groups allow you to analyze users collectively (e.g., by organization, team, or account).
     * This sets the group association for all subsequent events and reloads feature flags.
     *
     * @example
     * ```js
     * // associate user with an organization
     * posthog.group('organization', 'org_12345', {
     *     name: 'Acme Corp',
     *     plan: 'enterprise'
     * })
     * ```
     *
     * @example
     * ```js
     * // associate with multiple group types
     * posthog.group('organization', 'org_12345')
     * posthog.group('team', 'team_67890')
     * ```
     *
     * @public
     *
     * @param {String} groupType Group type (example: 'organization')
     * @param {String} groupKey Group key (example: 'org::5')
     * @param {Object} groupPropertiesToSet Optional properties to set for group
     */
    PostHog.prototype.group = function (groupType, groupKey, groupPropertiesToSet) {
        var _a, _b;
        if (!groupType || !groupKey) {
            logger_1.logger.error('posthog.group requires a group type and group key');
            return;
        }
        var existingGroups = this.getGroups();
        var isNewGroup = existingGroups[groupType] !== groupKey;
        // if group key changes, remove stored group properties
        if (isNewGroup) {
            this.resetGroupPropertiesForFlags(groupType);
        }
        this.register({ $groups: __assign(__assign({}, existingGroups), (_a = {}, _a[groupType] = groupKey, _a)) });
        // Send $groupidentify when the group is new/changed OR when properties
        // are provided. Skip only when the group already exists with the same
        // key and no new properties are being set.
        if (isNewGroup || groupPropertiesToSet) {
            var groupIdentifyProperties = {
                $group_type: groupType,
                $group_key: groupKey,
            };
            if (groupPropertiesToSet) {
                groupIdentifyProperties.$group_set = groupPropertiesToSet;
            }
            this.capture(constants_1.EVENT_GROUPIDENTIFY, groupIdentifyProperties);
        }
        if (groupPropertiesToSet) {
            this.setGroupPropertiesForFlags((_b = {}, _b[groupType] = groupPropertiesToSet, _b));
        }
        // If groups change and no properties change, reload feature flags.
        // The property change reload case is handled in setGroupPropertiesForFlags.
        if (isNewGroup && !groupPropertiesToSet) {
            this.reloadFeatureFlags();
        }
    };
    /**
     * Resets only the group properties of the user currently logged in.
     * Learn more about [groups](/docs/product-analytics/group-analytics)
     *
     * {@label Identification}
     *
     * @example
     * ```js
     * posthog.resetGroups()
     * ```
     *
     * @public
     */
    PostHog.prototype.resetGroups = function () {
        this.register({ $groups: {} });
        this.resetGroupPropertiesForFlags();
        // If groups changed, reload feature flags.
        this.reloadFeatureFlags();
    };
    /**
     * Sometimes, you might want to evaluate feature flags using properties that haven't been ingested yet,
     * or were set incorrectly earlier. You can do so by setting properties the flag depends on with these calls:
     *
     * {@label Feature flags}
     *
     * @example
     * ```js
     * // Set properties
     * posthog.setPersonPropertiesForFlags({'property1': 'value', property2: 'value2'})
     * ```
     *
     * @example
     * ```js
     * // Set properties without reloading
     * posthog.setPersonPropertiesForFlags({'property1': 'value', property2: 'value2'}, false)
     * ```
     *
     * @public
     *
     * @param {Object} properties The properties to override.
     * @param {Boolean} [reloadFeatureFlags] Whether to reload feature flags.
     */
    PostHog.prototype.setPersonPropertiesForFlags = function (properties, reloadFeatureFlags) {
        var _a;
        if (reloadFeatureFlags === void 0) { reloadFeatureFlags = true; }
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.setPersonPropertiesForFlags(properties, reloadFeatureFlags);
    };
    /**
     * Resets the person properties for feature flags.
     *
     * {@label Feature flags}
     *
     * @public
     *
     * @example
     * ```js
     * posthog.resetPersonPropertiesForFlags()
     * ```
     *
     * @example
     * ```js
     * // Reset properties without reloading
     * posthog.resetPersonPropertiesForFlags(false)
     * ```
     *
     * @param {Boolean} [reloadFeatureFlags=true] Whether to reload feature flags.
     */
    PostHog.prototype.resetPersonPropertiesForFlags = function (reloadFeatureFlags) {
        var _a;
        if (reloadFeatureFlags === void 0) { reloadFeatureFlags = true; }
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.resetPersonPropertiesForFlags(reloadFeatureFlags);
    };
    /**
     * Set override group properties for feature flags.
     * This is used when dealing with new groups / where you don't want to wait for ingestion
     * to update properties.
     * Takes in an object, the key of which is the group type.
     *
     * {@label Feature flags}
     *
     * @public
     *
     * @example
     * ```js
     * // Set properties with reload
     * posthog.setGroupPropertiesForFlags({'organization': { name: 'CYZ', employees: '11' } })
     * ```
     *
     * @example
     * ```js
     * // Set properties without reload
     * posthog.setGroupPropertiesForFlags({'organization': { name: 'CYZ', employees: '11' } }, false)
     * ```
     *
     * @param {Object} properties The properties to override, the key of which is the group type.
     * @param {Boolean} [reloadFeatureFlags] Whether to reload feature flags.
     */
    PostHog.prototype.setGroupPropertiesForFlags = function (properties, reloadFeatureFlags) {
        var _a;
        if (reloadFeatureFlags === void 0) { reloadFeatureFlags = true; }
        if (!this._requirePersonProcessing('posthog.setGroupPropertiesForFlags')) {
            return;
        }
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.setGroupPropertiesForFlags(properties, reloadFeatureFlags);
    };
    /**
     * Resets the group properties for feature flags.
     *
     * {@label Feature flags}
     *
     * @public
     *
     * @example
     * ```js
     * posthog.resetGroupPropertiesForFlags()
     * ```
     *
     * @param {string} [group_type] Optional group type to reset. If omitted, all group properties are reset.
     */
    PostHog.prototype.resetGroupPropertiesForFlags = function (group_type) {
        var _a;
        (_a = this.featureFlags) === null || _a === void 0 ? void 0 : _a.resetGroupPropertiesForFlags(group_type);
    };
    /**
     * Resets all user data and starts a fresh session.
     *
     * ⚠️ **Warning**: Only call this when a user logs out. Calling at the wrong time can cause split sessions.
     *
     * This clears:
     * - Session ID and super properties
     * - User identification (sets new random distinct_id)
     * - Cached data and consent settings
     *
     * {@label Identification}
     * @example
     * ```js
     * // reset on user logout
     * function logout() {
     *     posthog.reset()
     *     // redirect to login page
     * }
     * ```
     *
     * @example
     * ```js
     * // reset and generate new device ID
     * posthog.reset(true)  // also resets device_id
     * ```
     *
     * @public
     *
     * @param {boolean} [reset_device_id] Whether to generate a new device ID as well as a new distinct ID.
     */
    PostHog.prototype.reset = function (reset_device_id) {
        var _a, _b;
        var _c, _d, _e, _f, _g, _h, _j, _k, _l, _m, _o;
        logger_1.logger.info('reset');
        if (!this.__loaded) {
            return logger_1.logger.uninitializedWarning('posthog.reset');
        }
        var device_id = this.get_property(constants_1.DEVICE_ID);
        // $device_model describes the physical device, not the user, so preserve it across reset()
        // the same way $device_id is — it is only ever re-resolved at init.
        var device_model = this.get_property(constants_1.DEVICE_MODEL);
        // Snapshot the session-recording remote config before clearing persistence.
        // It's server-defined config (sample rate, masking, canvas, triggers, …),
        // not user state, and must survive reset(). Otherwise start('session_id_changed')
        // bails on the next session rotation: rrweb is torn down with no replacement
        // and the new session opens with no FullSnapshot until the next periodic
        // checkout (~5 min later).
        var recordingRemoteConfig = this.get_property(constants_1.SESSION_RECORDING_REMOTE_CONFIG);
        this.consent.reset();
        (_c = this.persistence) === null || _c === void 0 ? void 0 : _c.clear();
        (_d = this.sessionPersistence) === null || _d === void 0 ? void 0 : _d.clear();
        if (!(0, core_1.isUndefined)(recordingRemoteConfig)) {
            (_e = this.persistence) === null || _e === void 0 ? void 0 : _e.register((_a = {}, _a[constants_1.SESSION_RECORDING_REMOTE_CONFIG] = recordingRemoteConfig, _a));
        }
        (_f = this.surveys) === null || _f === void 0 ? void 0 : _f.reset();
        // Stop the refresh interval before resetting flags — featureFlags.reset() clears
        // the debouncer, so if the order were reversed a pending refresh could fire after reset.
        (_g = this._remoteConfigLoader) === null || _g === void 0 ? void 0 : _g.stop();
        (_h = this.featureFlags) === null || _h === void 0 ? void 0 : _h.reset();
        (_j = this.conversations) === null || _j === void 0 ? void 0 : _j.reset();
        (_k = this.logs) === null || _k === void 0 ? void 0 : _k.reset();
        (_l = this.metrics) === null || _l === void 0 ? void 0 : _l.reset();
        (_m = this.persistence) === null || _m === void 0 ? void 0 : _m.set_property(constants_1.USER_STATE, constants_1.USER_STATE_ANONYMOUS);
        (_o = this.sessionManager) === null || _o === void 0 ? void 0 : _o.resetSessionId();
        this._cachedPersonProperties = null;
        if (this.config.cookieless_mode === constants_1.COOKIELESS_ALWAYS) {
            this.register_once({
                distinct_id: constants_1.COOKIELESS_SENTINEL_VALUE,
                $device_id: null,
            }, '');
        }
        else {
            var uuid = this.config.get_device_id((0, uuidv7_1.uuidv7)());
            this.register_once({
                distinct_id: uuid,
                $device_id: reset_device_id ? uuid : device_id,
            }, '');
            if (!reset_device_id && !(0, core_1.isUndefined)(device_model)) {
                this.register((_b = {}, _b[constants_1.DEVICE_MODEL] = device_model, _b));
            }
        }
        this.register({
            $last_posthog_reset: new Date().toISOString(),
        }, 1);
        // Clear HMAC identity verification fields
        delete this.config.identity_distinct_id;
        delete this.config.identity_hash;
        // Reload feature flags for the new anonymous user, just like identify()
        // does when the distinct_id changes.
        this.reloadFeatureFlags();
    };
    /**
     * Flushes any queued events and resolves once teardown is complete.
     *
     * @remarks
     * This exists primarily for parity with the server-side
     * [Node.js SDK](/docs/libraries/node), whose `shutdown()` you call once before a
     * process exits. In the browser there is no process to exit — the SDK already
     * flushes pending events on `pagehide`/`unload` — so this method is mostly a
     * graceful no-op that best-effort flushes the request queues and always resolves.
     *
     * It is safe to call in isomorphic teardown code (for example a Nuxt/Next module
     * that calls `shutdown()` on both the server and the client) so the same
     * symmetric cleanup works in either environment without throwing.
     *
     * {@label Lifecycle}
     * @example
     * ```js
     * // symmetric teardown that runs on both server and client
     * await posthog.shutdown()
     * ```
     *
     * @public
     *
     * @param {number} [_shutdownTimeoutMs] Accepted for call-site parity with the Node.js SDK. The browser flush is synchronous, so this is ignored.
     * @returns {Promise<void>} A promise that resolves once the queues have been flushed.
     */
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    PostHog.prototype.shutdown = function (_shutdownTimeoutMs) {
        return __awaiter(this, void 0, void 0, function () {
            var _a, _b, _c, _d, _e;
            return __generator(this, function (_f) {
                if (!this.__loaded) {
                    logger_1.logger.uninitializedWarning('posthog.shutdown');
                    return [2 /*return*/];
                }
                // Best-effort flush of anything still queued, mirroring page-unload teardown
                // so no buffered events are silently dropped when teardown is explicit.
                (_a = this.logs) === null || _a === void 0 ? void 0 : _a.flushLogs('sendBeacon');
                void ((_b = this.metrics) === null || _b === void 0 ? void 0 : _b.flush('sendBeacon'));
                (_c = this._requestQueue) === null || _c === void 0 ? void 0 : _c.unload();
                (_d = this._retryQueue) === null || _d === void 0 ? void 0 : _d.unload();
                (_e = this.featureFlags) === null || _e === void 0 ? void 0 : _e.destroy();
                return [2 /*return*/];
            });
        });
    };
    /**
     * Set HMAC-based identity verification.
     *
     * @remarks
     * When set, products like conversations use server-verified identity
     * (distinct_id + HMAC hash) instead of anonymous session identifiers.
     * The hash should be computed server-side as HMAC-SHA256 of the
     * distinct_id using the project's API secret.
     *
     * @param distinctId - The verified user distinct_id
     * @param hash - HMAC-SHA256 of distinctId using the project API secret
     *
     * @example
     * ```js
     * posthog.setIdentity('user_123', 'a1b2c3d4e5f6...')
     * ```
     *
     * @public
     */
    PostHog.prototype.setIdentity = function (distinctId, hash) {
        var _a;
        this.config.identity_distinct_id = distinctId;
        this.config.identity_hash = hash;
        this.alias(distinctId);
        (_a = this.conversations) === null || _a === void 0 ? void 0 : _a._onIdentityChanged();
    };
    /**
     * Clear HMAC-based identity verification, reverting to anonymous mode.
     *
     * @example
     * ```js
     * posthog.clearIdentity()
     * ```
     *
     * @public
     */
    PostHog.prototype.clearIdentity = function () {
        var _a;
        delete this.config.identity_distinct_id;
        delete this.config.identity_hash;
        (_a = this.conversations) === null || _a === void 0 ? void 0 : _a._onIdentityCleared();
    };
    /**
     * Returns the current distinct ID for the user.
     *
     * @remarks
     * This is either the auto-generated ID or the ID set via `identify()`.
     * The distinct ID is used to associate events with users in PostHog.
     *
     * {@label Identification}
     *
     * @example
     * ```js
     * // get the current user ID
     * const userId = posthog.get_distinct_id()
     * console.log('Current user:', userId)
     * ```
     *
     * @example
     * ```js
     * // use in loaded callback
     * posthog.init('token', {
     *     loaded: (posthog) => {
     *         const id = posthog.get_distinct_id()
     *         // use the ID
     *     }
     * })
     * ```
     *
     * @public
     *
     * @returns The current distinct ID
     */
    PostHog.prototype.get_distinct_id = function () {
        return this.get_property('distinct_id');
    };
    /**
     * Returns the current groups.
     *
     * {@label Identification}
     *
     * @public
     *
     * @returns The current groups
     */
    PostHog.prototype.getGroups = function () {
        return this.get_property('$groups') || {};
    };
    /**
     * Returns the current session_id.
     *
     * @remarks
     * This should only be used for informative purposes.
     * Any actual internal use case for the session_id should be handled by the sessionManager.
     *
     * @public
     *
     * @returns The stored session ID for the current session. This may be an empty string if the client is not yet fully initialized.
     */
    PostHog.prototype.get_session_id = function () {
        var _a, _b;
        return (_b = (_a = this.sessionManager) === null || _a === void 0 ? void 0 : _a.checkAndGetSessionAndWindowId(true).sessionId) !== null && _b !== void 0 ? _b : '';
    };
    /**
     * Returns the Replay url for the current session.
     *
     * {@label Session replay}
     *
     * @public
     *
     * @example
     * ```js
     * // basic usage
     * posthog.get_session_replay_url()
     * ```
     *
     * @example
     * ```js
     * // timestamp
     * posthog.get_session_replay_url({ withTimestamp: true })
     * ```
     *
     * @example
     * ```js
     * // timestamp and lookback
     * posthog.get_session_replay_url({
     *   withTimestamp: true,
     *   timestampLookBack: 30 // look back 30 seconds
     * })
     * ```
     *
     * @param {Object} [options] Options for the URL.
     * @param {boolean} [options.withTimestamp] Whether to include the timestamp in the URL.
     * @param {number} [options.timestampLookBack] How many seconds to look back for the timestamp.
     * @returns The URL for the current session replay, or an empty string if sessions are unavailable.
     */
    PostHog.prototype.get_session_replay_url = function (options) {
        var _a;
        if (!this.sessionManager) {
            return '';
        }
        var _b = this.sessionManager.checkAndGetSessionAndWindowId(true), sessionId = _b.sessionId, sessionStartTimestamp = _b.sessionStartTimestamp;
        var url = this.requestRouter.endpointFor('ui', "/project/".concat(this.config.token, "/replay/").concat(sessionId));
        if ((options === null || options === void 0 ? void 0 : options.withTimestamp) && sessionStartTimestamp) {
            var LOOK_BACK = (_a = options.timestampLookBack) !== null && _a !== void 0 ? _a : 10;
            if (!sessionStartTimestamp) {
                return url;
            }
            var recordingStartTime = Math.max(Math.floor((new Date().getTime() - sessionStartTimestamp) / 1000) - LOOK_BACK, 0);
            url += "?t=".concat(recordingStartTime);
        }
        return url;
    };
    /**
     * Creates an alias linking two distinct user identifiers. Learn more about [identifying users](/docs/product-analytics/identify)
     *
     * {@label Identification}
     *
     * @remarks
     * PostHog will use this to link two distinct_ids going forward (not retroactively).
     * Call this when a user signs up to connect their anonymous session with their account.
     *
     *
     * @example
     * ```js
     * // link anonymous user to account on signup
     * posthog.alias('user_12345')
     * ```
     *
     * @example
     * ```js
     * // explicit alias with original ID
     * posthog.alias('user_12345', 'anonymous_abc123')
     * ```
     *
     * @public
     *
     * @param {String} alias A unique identifier that you want to use for this user in the future.
     * @param {String} [original] The current identifier being used for this user.
     * @returns The `$create_alias` capture result, `-1` if alias matches the current distinct ID, `-2` if aliasing would duplicate a People user, or void if person processing is disabled.
     */
    PostHog.prototype.alias = function (alias, original) {
        // If the $people_distinct_id key exists in persistence, there has been a previous
        // posthog.people.identify() call made for this user. It is VERY BAD to make an alias with
        // this ID, as it will duplicate users.
        if (alias === this.get_property(constants_1.PEOPLE_DISTINCT_ID_KEY)) {
            logger_1.logger.critical('Attempting to create alias for existing People user - aborting.');
            return -2;
        }
        if (!this._requirePersonProcessing('posthog.alias')) {
            return;
        }
        if ((0, core_1.isUndefined)(original)) {
            original = this.get_distinct_id();
        }
        if (alias !== original) {
            this._register_single(constants_1.ALIAS_ID_KEY, alias);
            return this.capture('$create_alias', { alias: alias, distinct_id: original });
        }
        else {
            logger_1.logger.warn('alias matches current distinct_id - skipping api call.');
            this.identify(alias);
            return -1;
        }
    };
    /**
     * Updates the configuration of the PostHog instance.
     *
     * {@label Initialization}
     *
     * @public
     *
     * @param {Partial<PostHogConfig>} config A dictionary of new configuration values to update
     */
    PostHog.prototype.set_config = function (config) {
        var _a, _b, _c, _d, _e, _f, _g, _h, _j, _k, _l;
        var oldConfig = __assign({}, this.config);
        if ((0, core_1.isObject)(config)) {
            (0, general_utils_1.extend)(this.config, (0, exports.configRenames)(config));
            var isPersistenceDisabled = this._is_persistence_disabled();
            (_a = this.persistence) === null || _a === void 0 ? void 0 : _a.update_config(this.config, oldConfig, isPersistenceDisabled);
            this.sessionPersistence =
                this.config.persistence === 'sessionStorage' || this.config.persistence === 'memory'
                    ? this.persistence
                    : // sessionStorage sibling shares the primary's storage name; it must not own/clean the split group entries
                        new posthog_persistence_1.PostHogPersistence(__assign(__assign({}, this.config), { persistence: 'sessionStorage' }), isPersistenceDisabled, false);
            var debugConfigFromLocalStorage = this._checkLocalStorageForDebug(this.config.debug);
            if ((0, core_1.isBoolean)(debugConfigFromLocalStorage)) {
                this.config.debug = debugConfigFromLocalStorage;
            }
            if ((0, core_1.isBoolean)(this.config.debug)) {
                if (this.config.debug) {
                    config_1.default.DEBUG = true;
                    storage_1.localStore._is_supported() && storage_1.localStore._set('ph_debug', true);
                    logger_1.logger.info('set_config', {
                        config: config,
                        oldConfig: oldConfig,
                        newConfig: __assign({}, this.config),
                    });
                }
                else {
                    config_1.default.DEBUG = false;
                    storage_1.localStore._is_supported() && storage_1.localStore._remove('ph_debug');
                }
            }
            (_b = this.exceptionObserver) === null || _b === void 0 ? void 0 : _b.onConfigChange();
            (_c = this.exceptions) === null || _c === void 0 ? void 0 : _c.onConfigChange();
            (_d = this.sessionRecording) === null || _d === void 0 ? void 0 : _d.startIfEnabledOrStop();
            (_e = this.tracingHeaders) === null || _e === void 0 ? void 0 : _e.startIfEnabledOrStop();
            (_f = this.autocapture) === null || _f === void 0 ? void 0 : _f.startIfEnabled();
            (_g = this.heatmaps) === null || _g === void 0 ? void 0 : _g.startIfEnabled();
            (_h = this.exceptionObserver) === null || _h === void 0 ? void 0 : _h.startIfEnabledOrStop();
            (_j = this.deadClicksAutocapture) === null || _j === void 0 ? void 0 : _j.startIfEnabledOrStop();
            (_k = this.surveys) === null || _k === void 0 ? void 0 : _k.loadIfEnabled();
            this._sync_opt_out_with_persistence();
            (_l = this.externalIntegrations) === null || _l === void 0 ? void 0 : _l.startIfEnabledOrStop();
        }
    };
    /**
     * @internal
     * Allows wrapper SDKs (e.g. posthog-flutter, posthog-react-native) to override the
     * `$lib` and `$lib_version` properties sent with every event.
     *
     * This is not a public API and may change without notice.
     */
    PostHog.prototype._overrideSDKInfo = function (sdkName, sdkVersion) {
        config_1.default.LIB_NAME = sdkName;
        config_1.default.LIB_VERSION = sdkVersion;
    };
    /**
     * turns session recording on, and updates the config option `disable_session_recording` to false
     *
     * {@label Session replay}
     *
     * @public
     *
     * @example
     * ```js
     * // Start and ignore controls
     * posthog.startSessionRecording(true)
     * ```
     *
     * @example
     * ```js
     * // Start and override controls
     * posthog.startSessionRecording({
     *   // you don't have to send all of these
     *   sampling: true || false,
     *   linked_flag: true || false,
     *   url_trigger: true || false,
     *   event_trigger: true || false
     * })
     * ```
     *
     * @param override.sampling - optional boolean to override the default sampling behavior - ensures the next session recording to start will not be skipped by sampling config.
     * @param override.linked_flag - optional boolean to override the default linked_flag behavior - ensures the next session recording to start will not be skipped by linked_flag config.
     * @param override.url_trigger - optional boolean to override the default url_trigger behavior - ensures the next session recording to start will not be skipped by url_trigger config.
     * @param override.event_trigger - optional boolean to override the default event_trigger behavior - ensures the next session recording to start will not be skipped by event_trigger config.
     * @param override - optional boolean to override the default sampling behavior - ensures the next session recording to start will not be skipped by sampling or linked_flag config. `true` is shorthand for { sampling: true, linked_flag: true }
     */
    PostHog.prototype.startSessionRecording = function (override) {
        var _a, _b, _c, _d, _e;
        var overrideAll = override === true;
        var overrideConfig = {
            sampling: overrideAll || !!(override === null || override === void 0 ? void 0 : override.sampling),
            linked_flag: overrideAll || !!(override === null || override === void 0 ? void 0 : override.linked_flag),
            url_trigger: overrideAll || !!(override === null || override === void 0 ? void 0 : override.url_trigger),
            event_trigger: overrideAll || !!(override === null || override === void 0 ? void 0 : override.event_trigger),
        };
        if (Object.values(overrideConfig).some(Boolean)) {
            // allow the session id check to rotate session id if necessary
            (_a = this.sessionManager) === null || _a === void 0 ? void 0 : _a.checkAndGetSessionAndWindowId();
            if (overrideConfig.sampling) {
                (_b = this.sessionRecording) === null || _b === void 0 ? void 0 : _b.overrideSampling();
            }
            if (overrideConfig.linked_flag) {
                (_c = this.sessionRecording) === null || _c === void 0 ? void 0 : _c.overrideLinkedFlag();
            }
            if (overrideConfig.url_trigger) {
                (_d = this.sessionRecording) === null || _d === void 0 ? void 0 : _d.overrideTrigger('url');
            }
            if (overrideConfig.event_trigger) {
                (_e = this.sessionRecording) === null || _e === void 0 ? void 0 : _e.overrideTrigger('event');
            }
        }
        this.set_config({ disable_session_recording: false });
    };
    /**
     * turns session recording off, and updates the config option
     * disable_session_recording to true
     *
     * {@label Session replay}
     *
     * @public
     *
     * @example
     * ```js
     * // Stop session recording
     * posthog.stopSessionRecording()
     * ```
     */
    PostHog.prototype.stopSessionRecording = function () {
        this.set_config({ disable_session_recording: true });
    };
    /**
     * returns a boolean indicating whether session recording
     * is currently running
     *
     * {@label Session replay}
     *
     * @public
     *
     * @example
     * ```js
     * // Stop session recording if it's running
     * if (posthog.sessionRecordingStarted()) {
     *   posthog.stopSessionRecording()
     * }
     * ```
     *
     * @returns Whether session recording is currently running.
     */
    PostHog.prototype.sessionRecordingStarted = function () {
        var _a;
        return !!((_a = this.sessionRecording) === null || _a === void 0 ? void 0 : _a.started);
    };
    /**
     * Capture a caught exception manually
     *
     * {@label Error tracking}
     *
     * @public
     *
     * @example
     * ```js
     * // Capture a caught exception
     * try {
     *   // something that might throw
     * } catch (error) {
     *   posthog.captureException(error)
     * }
     * ```
     *
     * @example
     * ```js
     * // With additional properties
     * posthog.captureException(error, {
     *   customProperty: 'value',
     *   anotherProperty: ['I', 'can be a list'],
     *   ...
     * })
     * ```
     *
     * @param {unknown} error The error or exception-like value to capture.
     * @param {Properties} [additionalProperties] Any additional properties to add to the error event.
     * @returns The result of the capture, or undefined if exception capture is unavailable.
     */
    PostHog.prototype.captureException = function (error, additionalProperties) {
        if (!this.exceptions)
            return;
        var syntheticException = new Error('PostHog syntheticException');
        var errorToProperties = this.exceptions.buildProperties(error, {
            handled: true,
            syntheticException: syntheticException,
        });
        return this.exceptions.sendExceptionEvent(__assign(__assign({}, errorToProperties), additionalProperties));
    };
    /**
     * Add a breadcrumb-like step that will be attached to the next captured exception.
     *
     * {@label Error tracking}
     *
     * @public
     *
     * @example
     * ```js
     * posthog.addExceptionStep('Checkout button clicked', {
     *   checkout_id: 'ch_123',
     * })
     * ```
     *
     * @param {string} message The step message.
     * @param {Properties} [properties] Additional context for this step.
     */
    PostHog.prototype.addExceptionStep = function (message, properties) {
        var _a;
        (_a = this.exceptions) === null || _a === void 0 ? void 0 : _a.addExceptionStep(message, properties);
    };
    /**
     * Capture a log entry and send it to the PostHog logs endpoint.
     *
     * {@label Logs}
     *
     * @public
     *
     * @example
     * ```js
     * posthog.captureLog({
     *   body: 'checkout completed',
     *   level: 'info',
     *   attributes: { order_id: 'ord_789', amount_cents: 4999 },
     * })
     * ```
     *
     * @param {CaptureLogOptions} options The log entry options
     */
    PostHog.prototype.captureLog = function (options) {
        var _a;
        (_a = this.logs) === null || _a === void 0 ? void 0 : _a.captureLog(options);
    };
    Object.defineProperty(PostHog.prototype, "logger", {
        /**
         * Logger with convenience methods for each severity level.
         *
         * @example
         * ```js
         * posthog.logger.info('checkout completed', { order_id: 'ord_789' })
         * posthog.logger.error('payment failed', { error_code: 'E001' })
         * ```
         */
        get: function () {
            var _a, _b;
            return (_b = (_a = this.logs) === null || _a === void 0 ? void 0 : _a.logger) !== null && _b !== void 0 ? _b : PostHog._noopLogger;
        },
        enumerable: false,
        configurable: true
    });
    /**
     * turns exception autocapture on, and updates the config option `capture_exceptions` to the provided config (or `true`)
     *
     * {@label Error tracking}
     *
     * @public
     *
     * @example
     * ```js
     * // Start with default exception autocapture rules. No-op if already enabled
     * posthog.startExceptionAutocapture()
     * ```
     *
     * @example
     * ```js
     * // Start and override controls
     * posthog.startExceptionAutocapture({
     *   // you don't have to send all of these (unincluded values will use the default)
     *   capture_unhandled_errors: true || false,
     *   capture_unhandled_rejections: true || false,
     *   capture_console_errors: true || false
     * })
     * ```
     *
     * @param config - optional configuration option to control the exception autocapture behavior
     */
    PostHog.prototype.startExceptionAutocapture = function (config) {
        this.set_config({ capture_exceptions: config !== null && config !== void 0 ? config : true });
    };
    /**
     * turns exception autocapture off by updating the config option `capture_exceptions` to `false`
     *
     * {@label Error tracking}
     *
     * @public
     *
     * @example
     * ```js
     * // Stop capturing exceptions automatically
     * posthog.stopExceptionAutocapture()
     * ```
     */
    PostHog.prototype.stopExceptionAutocapture = function () {
        this.set_config({ capture_exceptions: false });
    };
    /**
     * returns a boolean indicating whether the [toolbar](/docs/toolbar) loaded
     *
     * {@label Toolbar}
     *
     * @public
     *
     * @param {ToolbarParams} params Toolbar parameters.
     * @returns Whether the toolbar loaded.
     */
    PostHog.prototype.loadToolbar = function (params) {
        var _a, _b;
        return (_b = (_a = this.toolbar) === null || _a === void 0 ? void 0 : _a.loadToolbar(params)) !== null && _b !== void 0 ? _b : false;
    };
    /**
     * Returns the value of a super property. Returns undefined if the property doesn't exist.
     *
     * {@label Identification}
     *
     * @remarks
     * get_property() can only be called after the PostHog library has finished loading.
     * init() has a loaded function available to handle this automatically.
     *
     * @example
     * ```js
     * // grab value for '$user_id' after the posthog library has loaded
     * posthog.init('<YOUR PROJECT TOKEN>', {
     *     loaded: function(posthog) {
     *         user_id = posthog.get_property('$user_id');
     *     }
     * });
     * ```
     * @public
     *
     * @param {String} property_name The name of the super property you want to retrieve
     * @returns The stored property value, or undefined if it is not set.
     */
    PostHog.prototype.get_property = function (property_name) {
        var _a;
        return (_a = this.persistence) === null || _a === void 0 ? void 0 : _a.props[property_name];
    };
    /**
     * Returns the value of the session super property named property_name. If no such
     * property is set, getSessionProperty() will return the undefined value.
     *
     * {@label Identification}
     *
     * @public
     *
     * @remarks
     * This is based on browser-level `sessionStorage`, NOT the PostHog session.
     * getSessionProperty() can only be called after the PostHog library has finished loading.
     * init() has a loaded function available to handle this automatically.
     *
     * @example
     * ```js
     * // grab value for 'user_id' after the posthog library has loaded
     * posthog.init('YOUR PROJECT TOKEN', {
     *     loaded: function(posthog) {
     *         user_id = posthog.getSessionProperty('user_id');
     *     }
     * });
     * ```
     *
     * @param {String} property_name The name of the session super property you want to retrieve
     * @returns The stored session property value, or undefined if it is not set.
     */
    PostHog.prototype.getSessionProperty = function (property_name) {
        var _a;
        return (_a = this.sessionPersistence) === null || _a === void 0 ? void 0 : _a.props[property_name];
    };
    /**
     * Returns a string representation of the PostHog instance.
     *
     * {@label Initialization}
     *
     * @internal
     */
    PostHog.prototype.toString = function () {
        var _a;
        var name = (_a = this.config.name) !== null && _a !== void 0 ? _a : PRIMARY_INSTANCE_NAME;
        if (name !== PRIMARY_INSTANCE_NAME) {
            name = PRIMARY_INSTANCE_NAME + '.' + name;
        }
        return name;
    };
    PostHog.prototype._isIdentified = function () {
        var _a, _b;
        return (((_a = this.persistence) === null || _a === void 0 ? void 0 : _a.get_property(constants_1.USER_STATE)) === constants_1.USER_STATE_IDENTIFIED ||
            ((_b = this.sessionPersistence) === null || _b === void 0 ? void 0 : _b.get_property(constants_1.USER_STATE)) === constants_1.USER_STATE_IDENTIFIED);
    };
    PostHog.prototype._hasPersonProcessing = function () {
        var _a, _b, _c, _d;
        return !(this.config.person_profiles === 'never' ||
            (this.config.person_profiles === constants_1.PERSON_PROFILES_IDENTIFIED_ONLY &&
                !this._isIdentified() &&
                (0, core_1.isEmptyObject)(this.getGroups()) &&
                !((_b = (_a = this.persistence) === null || _a === void 0 ? void 0 : _a.props) === null || _b === void 0 ? void 0 : _b[constants_1.ALIAS_ID_KEY]) &&
                !((_d = (_c = this.persistence) === null || _c === void 0 ? void 0 : _c.props) === null || _d === void 0 ? void 0 : _d[constants_1.ENABLE_PERSON_PROCESSING])));
    };
    PostHog.prototype._shouldCapturePageleave = function () {
        return (this.config.capture_pageleave === true ||
            (this.config.capture_pageleave === 'if_capture_pageview' &&
                (this.config.capture_pageview === true || this.config.capture_pageview === 'history_change')));
    };
    /**
     *  Creates a person profile for the current user, if they don't already have one and config.person_profiles is set
     *  to 'identified_only'. Produces a warning and does not create a profile if config.person_profiles is set to
     *  'never'. Learn more about [person profiles](/docs/product-analytics/identify)
     *
     * {@label Identification}
     *
     * @public
     *
     * @example
     * ```js
     * posthog.createPersonProfile()
     * ```
     */
    PostHog.prototype.createPersonProfile = function () {
        if (this._hasPersonProcessing()) {
            // if a person profile already exists, don't send an event when we don't need to
            return;
        }
        if (!this._requirePersonProcessing('posthog.createPersonProfile')) {
            return;
        }
        // sent a $set event. We don't set any properties here, but attribution props will be added later
        this.setPersonProperties({}, {});
    };
    /**
     * Marks the current user as a test user by setting the `$internal_or_test_user` person property to `true`.
     * This also enables person processing for the current user.
     *
     * This is useful for using in a cohort your internal/test filters for your posthog org.
     * @see https://posthog.com/tutorials/filter-internal-users
     * Create a cohort with `$internal_or_test_user` IS SET, and set your internal test filters to be NOT IN that cohort.
     *
     * {@label Identification}
     *
     * @example
     * ```js
     * // Manually mark as test user
     * posthog.setInternalOrTestUser()
     *
     * // Or use internal_or_test_user_hostname config for automatic detection
     * posthog.init('token', { internal_or_test_user_hostname: 'localhost' })
     * ```
     *
     * @public
     */
    PostHog.prototype.setInternalOrTestUser = function () {
        if (!this._requirePersonProcessing('posthog.setInternalOrTestUser')) {
            return;
        }
        this.setPersonProperties({ $internal_or_test_user: true });
    };
    /**
     * Enables person processing if possible, returns true if it does so or already enabled, false otherwise
     *
     * @param function_name
     */
    PostHog.prototype._requirePersonProcessing = function (function_name) {
        if (this.config.person_profiles === 'never') {
            logger_1.logger.error(function_name + ' was called, but process_person is set to "never". This call will be ignored.');
            return false;
        }
        this._register_single(constants_1.ENABLE_PERSON_PROCESSING, true);
        return true;
    };
    PostHog.prototype._is_persistence_disabled = function () {
        if (this.config.cookieless_mode === 'always') {
            return true;
        }
        var isOptedOut = this.consent.isOptedOut();
        var defaultPersistenceDisabled = this.config.opt_out_persistence_by_default || this.config.cookieless_mode === constants_1.COOKIELESS_ON_REJECT;
        // TRICKY: We want a deterministic state for persistence so that a new pageload has the same persistence
        return this.config.disable_persistence || (isOptedOut && !!defaultPersistenceDisabled);
    };
    PostHog.prototype._sync_opt_out_with_persistence = function () {
        var _a, _b, _c, _d;
        var persistenceDisabled = this._is_persistence_disabled();
        if (((_a = this.persistence) === null || _a === void 0 ? void 0 : _a._disabled) !== persistenceDisabled) {
            (_b = this.persistence) === null || _b === void 0 ? void 0 : _b.set_disabled(persistenceDisabled);
        }
        if (((_c = this.sessionPersistence) === null || _c === void 0 ? void 0 : _c._disabled) !== persistenceDisabled) {
            (_d = this.sessionPersistence) === null || _d === void 0 ? void 0 : _d.set_disabled(persistenceDisabled);
        }
        return persistenceDisabled;
    };
    /**
     * Opts the user into data capturing and persistence.
     *
     * {@label Privacy}
     *
     * @remarks
     * Enables event tracking and data persistence (cookies/localStorage) for this PostHog instance.
     * By default, captures an `$opt_in` event unless disabled.
     *
     * @example
     * ```js
     * // simple opt-in
     * posthog.opt_in_capturing()
     * ```
     *
     * @example
     * ```js
     * // opt-in with custom event and properties
     * posthog.opt_in_capturing({
     *     captureEventName: 'Privacy Accepted',
     *     captureProperties: { source: 'banner' }
     * })
     * ```
     *
     * @example
     * ```js
     * // opt-in without capturing event
     * posthog.opt_in_capturing({
     *     captureEventName: false
     * })
     * ```
     *
     * @public
     *
     * @param {Object} [options] A dictionary of opt-in options.
     * @param {EventName | null | false} [options.captureEventName] Event name to use for capturing the opt-in action. Set to `null` or `false` to skip capturing the opt-in event. Defaults to `$opt_in`.
     * @param {Properties} [options.captureProperties] Set of properties to capture along with the opt-in action.
     */
    PostHog.prototype.opt_in_capturing = function (options) {
        var _a, _b, _c, _d, _e, _f, _g, _h, _j, _k;
        if (this.config.cookieless_mode === constants_1.COOKIELESS_ALWAYS) {
            logger_1.logger.warn(CONSENT_COOKIELESS_WARN);
            return;
        }
        if (this._inCookielessMode()) {
            // If the user was being treated as rejected in on_reject mode (either explicitly opted out, or opted out by default via opt_out_capturing_by_default), then before we can start sending regular non-cookieless events
            // we need to reset the instance to ensure that there is no leaking of state or data between the cookieless and regular events
            this.reset(true);
            (_a = this.sessionManager) === null || _a === void 0 ? void 0 : _a.destroy();
            (_b = this.pageViewManager) === null || _b === void 0 ? void 0 : _b.destroy();
            this.sessionManager = new sessionid_1.SessionIdManager(this);
            this.pageViewManager = new page_view_1.PageViewManager(this);
            if (this.persistence) {
                this.sessionPropsManager = new session_props_1.SessionPropsManager(this, this.sessionManager, this.persistence);
            }
            var SessionRecordingClass = (_d = (_c = this.config.__extensionClasses) === null || _c === void 0 ? void 0 : _c.sessionRecording) !== null && _d !== void 0 ? _d : (_e = PostHog.__defaultExtensionClasses) === null || _e === void 0 ? void 0 : _e.sessionRecording;
            if (SessionRecordingClass) {
                this.sessionRecording = this._replaceExtension(this.sessionRecording, new SessionRecordingClass(this));
                // Replay the cached remote config result so the new recorder picks up
                // server-side settings (enable flag, endpoint, sampling) that arrived while
                // we were still in cookieless mode and sessionRecording didn't yet exist.
                if (this._lastRemoteConfig) {
                    (_g = (_f = this.sessionRecording) === null || _f === void 0 ? void 0 : _f.onRemoteConfig) === null || _g === void 0 ? void 0 : _g.call(_f, this._lastRemoteConfig);
                }
            }
        }
        this.consent.optInOut(true);
        this._sync_opt_out_with_persistence();
        // Start queue after opting in
        this._start_queue_if_opted_in();
        // Restart session recording if it should now be enabled
        // (this handles the case where opt_out_capturing_by_default or cookieless_mode prevented it from starting)
        (_h = this.sessionRecording) === null || _h === void 0 ? void 0 : _h.startIfEnabledOrStop();
        // Reinitialize surveys if we're in cookieless mode and just opted in
        if (this.config.cookieless_mode == constants_1.COOKIELESS_ON_REJECT) {
            (_j = this.surveys) === null || _j === void 0 ? void 0 : _j.loadIfEnabled();
        }
        // Don't capture if captureEventName is null or false
        if ((0, core_1.isUndefined)(options === null || options === void 0 ? void 0 : options.captureEventName) || (options === null || options === void 0 ? void 0 : options.captureEventName)) {
            this.capture((_k = options === null || options === void 0 ? void 0 : options.captureEventName) !== null && _k !== void 0 ? _k : '$opt_in', options === null || options === void 0 ? void 0 : options.captureProperties, { send_instantly: true });
        }
        if (this.config.capture_pageview) {
            this._captureInitialPageview();
        }
    };
    /**
     * Opts the user out of data capturing and persistence.
     *
     * {@label Privacy}
     *
     * @remarks
     * Disables event tracking and data persistence (cookies/localStorage) for this PostHog instance.
     * If `opt_out_persistence_by_default` is true, SDK persistence will also be disabled.
     *
     * @example
     * ```js
     * // opt user out (e.g., on privacy settings page)
     * posthog.opt_out_capturing()
     * ```
     *
     * @public
     */
    PostHog.prototype.opt_out_capturing = function () {
        var _a, _b, _c;
        if (this.config.cookieless_mode === constants_1.COOKIELESS_ALWAYS) {
            logger_1.logger.warn(CONSENT_COOKIELESS_WARN);
            return;
        }
        if (this.config.cookieless_mode === constants_1.COOKIELESS_ON_REJECT && this.consent.isOptedIn()) {
            // If the user has opted in, we need to reset the instance to ensure that there is no leaking of state or data between the cookieless and regular events
            this.reset(true);
        }
        this.consent.optInOut(false);
        this._sync_opt_out_with_persistence();
        if (this.config.cookieless_mode === constants_1.COOKIELESS_ON_REJECT) {
            // If cookieless_mode is COOKIELESS_ON_REJECT, we start capturing events in cookieless mode
            this.register({
                distinct_id: constants_1.COOKIELESS_SENTINEL_VALUE,
                $device_id: null,
            });
            // tear down rrweb observers before sessionManager goes away — late events would throw
            (_a = this.sessionRecording) === null || _a === void 0 ? void 0 : _a.stopRecording();
            this.sessionRecording = undefined;
            (_b = this.sessionManager) === null || _b === void 0 ? void 0 : _b.destroy();
            (_c = this.pageViewManager) === null || _c === void 0 ? void 0 : _c.destroy();
            this.sessionManager = undefined;
            this.sessionPropsManager = undefined;
            if (this.config.capture_pageview) {
                this._captureInitialPageview();
            }
            // At init time, consent was PENDING so is_capturing() was false and _start_queue_if_opted_in() was a no-op.
            // Now that rejection has been recorded, capturing is active — enable the queue so batched events are flushed.
            this._start_queue_if_opted_in();
        }
    };
    /**
     * Checks if the user has opted into data capturing.
     *
     * {@label Privacy}
     *
     * @remarks
     * Returns the current consent status for event tracking and data persistence.
     *
     * @example
     * ```js
     * if (posthog.has_opted_in_capturing()) {
     *     // show analytics features
     * }
     * ```
     *
     * @public
     *
     * @returns {boolean} current opt-in status
     */
    PostHog.prototype.has_opted_in_capturing = function () {
        return this.consent.isOptedIn();
    };
    /**
     * Checks if the user has opted out of data capturing.
     *
     * {@label Privacy}
     *
     * @remarks
     * Returns the current consent status for event tracking and data persistence.
     *
     * @example
     * ```js
     * if (posthog.has_opted_out_capturing()) {
     *     // disable analytics features
     * }
     * ```
     *
     * @public
     *
     * @returns {boolean} current opt-out status
     */
    PostHog.prototype.has_opted_out_capturing = function () {
        return this.consent.isOptedOut();
    };
    /**
     * Returns the explicit consent status of the user.
     *
     * @remarks
     * This can be used to check if the user has explicitly opted in or out of data capturing, or neither. This does not
     * take the default config options into account, only whether the user has made an explicit choice, so this can be
     * used to determine whether to show an initial cookie banner or not.
     *
     * @example
     * ```js
     * const consentStatus = posthog.get_explicit_consent_status()
     * if (consentStatus === "granted") {
     *     // user has explicitly opted in
     * } else if (consentStatus === "denied") {
     *     // user has explicitly opted out
     * } else if (consentStatus === "pending"){
     *     // user has not made a choice, show consent banner
     * }
     * ```
     *
     * @public
     *
     * @returns The current explicit consent status.
     */
    PostHog.prototype.get_explicit_consent_status = function () {
        var consent = this.consent.consent;
        return consent === consent_1.ConsentStatus.GRANTED ? 'granted' : consent === consent_1.ConsentStatus.DENIED ? 'denied' : 'pending';
    };
    /**
     * Checks whether the PostHog library is currently capturing events.
     *
     * Usually this means that the user has not opted out of capturing, but the exact behaviour can be controlled by
     * some config options.
     *
     * Additionally, if the cookieless_mode is set to `'on_reject'`, we will capture events in cookieless mode if the
     * user has opted out or been defaulted to opt-out.
     *
     * {@label Privacy}
     *
     * @see {PostHogConfig.cookieless_mode}
     * @see {PostHogConfig.opt_out_persistence_by_default}
     * @see {PostHogConfig.respect_dnt}
     *
     * @returns {boolean} whether the posthog library is capturing events
     */
    PostHog.prototype.is_capturing = function () {
        if (this.config.cookieless_mode === constants_1.COOKIELESS_ALWAYS) {
            return true;
        }
        if (this.config.cookieless_mode === constants_1.COOKIELESS_ON_REJECT) {
            return this.consent.isRejected() || this.consent.isOptedIn();
        }
        else {
            return !this.has_opted_out_capturing();
        }
    };
    /**
     * Clear the user's opt in/out status of data capturing and cookies/localstorage for this PostHog instance
     *
     * {@label Privacy}
     *
     * @public
     *
     */
    PostHog.prototype.clear_opt_in_out_capturing = function () {
        this.consent.reset();
        this._sync_opt_out_with_persistence();
    };
    PostHog.prototype._is_bot = function () {
        if (globals_1.navigator) {
            return (0, blocked_uas_1.isLikelyBot)(globals_1.navigator, this.config.custom_blocked_useragents);
        }
        else {
            return undefined;
        }
    };
    PostHog.prototype._captureInitialPageview = function () {
        if (!globals_1.document) {
            return;
        }
        // If page is not visible, add a listener to detect when the page becomes visible
        // and trigger the pageview only then
        // This is useful to avoid `prerender` calls from Chrome/Wordpress/SPAs
        // that are not visible to the user
        if (globals_1.document.visibilityState !== 'visible') {
            if (!this._visibilityStateListener) {
                this._visibilityStateListener = this._captureInitialPageview.bind(this);
                (0, general_utils_1.addEventListener)(globals_1.document, constants_1.DOM_EVENT_VISIBILITYCHANGE, this._visibilityStateListener);
            }
            return;
        }
        // Extra check here to guarantee we only ever trigger a single `$pageview` event
        if (!this._initialPageviewCaptured) {
            this._initialPageviewCaptured = true;
            this.capture(constants_1.EVENT_PAGEVIEW, { title: globals_1.document.title }, { send_instantly: true });
            // After we've captured the initial pageview, we can remove the listener
            if (this._visibilityStateListener) {
                globals_1.document.removeEventListener(constants_1.DOM_EVENT_VISIBILITYCHANGE, this._visibilityStateListener);
                this._visibilityStateListener = null;
            }
        }
    };
    /**
     * Enables or disables debug mode for detailed logging.
     *
     * @remarks
     * Debug mode logs all PostHog calls to the browser console for troubleshooting.
     * Can also be enabled by adding `?__posthog_debug=true` to the URL.
     *
     * {@label Initialization}
     *
     * @example
     * ```js
     * // enable debug mode
     * posthog.debug(true)
     * ```
     *
     * @example
     * ```js
     * // disable debug mode
     * posthog.debug(false)
     * ```
     *
     * @public
     *
     * @param {boolean} [debug] If true, will enable debug mode.
     */
    PostHog.prototype.debug = function (debug) {
        if (debug === false) {
            globals_1.window === null || globals_1.window === void 0 ? void 0 : globals_1.window.console.log("You've disabled debug mode.");
            this.set_config({ debug: false });
        }
        else {
            globals_1.window === null || globals_1.window === void 0 ? void 0 : globals_1.window.console.log("You're now in debug mode. All calls to PostHog will be logged in your console.\nYou can disable this with `posthog.debug(false)`.");
            this.set_config({ debug: true });
        }
    };
    /**
     * Helper method to check if external API calls (flags/decide) should be disabled
     * Handles migration from old `advanced_disable_decide` to new `advanced_disable_flags`
     */
    PostHog.prototype._shouldDisableFlags = function () {
        // Check if advanced_disable_flags was explicitly set in original config
        var originalConfig = this._originalUserConfig || {};
        if ('advanced_disable_flags' in originalConfig) {
            return !!originalConfig.advanced_disable_flags;
        }
        // Check if advanced_disable_flags was set post-init (different from default false)
        if (this.config.advanced_disable_flags !== false) {
            return !!this.config.advanced_disable_flags;
        }
        // Check for post-init changes to advanced_disable_decide
        if (this.config.advanced_disable_decide === true) {
            logger_1.logger.warn("Config field 'advanced_disable_decide' is deprecated. Please use 'advanced_disable_flags' instead. " +
                'The old field will be removed in a future major version.');
            return true;
        }
        // Fall back to migration logic for original user config
        return (0, general_utils_1.migrateConfigField)(originalConfig, 'advanced_disable_flags', 'advanced_disable_decide', false, logger_1.logger);
    };
    PostHog.prototype._runBeforeSend = function (data) {
        var e_2, _a, e_3, _b;
        var _c;
        if ((0, core_1.isNullish)(this.config.before_send)) {
            return data;
        }
        // Some properties are required for the event to be ingested - e.g. `token`
        // carries the project api_key, and ingest rejects any event that arrives
        // without it (a 401). Snapshot which of these were present before the hooks
        // run, so we can tell if a hook removed one (the names are captured here, so
        // a hook that mutates `data.properties` in place can't hide the removal).
        var requiredProperties = Object.keys((_c = data.properties) !== null && _c !== void 0 ? _c : {}).filter(core_1.isKnownUnsafeEditableEventProperty);
        var fns = (0, core_1.isArray)(this.config.before_send) ? this.config.before_send : [this.config.before_send];
        var beforeSendResult = data;
        try {
            for (var fns_1 = __values(fns), fns_1_1 = fns_1.next(); !fns_1_1.done; fns_1_1 = fns_1.next()) {
                var fn = fns_1_1.value;
                beforeSendResult = fn(beforeSendResult);
                if ((0, core_1.isNullish)(beforeSendResult)) {
                    var logMessage = "Event '".concat(data.event, "' was rejected in beforeSend function");
                    if ((0, core_1.isKnownUnsafeEditableEvent)(data.event)) {
                        logger_1.logger.warn("".concat(logMessage, ". This can cause unexpected behavior."));
                    }
                    else {
                        logger_1.logger.info(logMessage);
                    }
                    return null;
                }
                if (!beforeSendResult.properties || (0, core_1.isEmptyObject)(beforeSendResult.properties)) {
                    logger_1.logger.warn("Event '".concat(data.event, "' has no properties after beforeSend function, this is likely an error."));
                }
            }
        }
        catch (e_2_1) { e_2 = { error: e_2_1 }; }
        finally {
            try {
                if (fns_1_1 && !fns_1_1.done && (_a = fns_1.return)) _a.call(fns_1);
            }
            finally { if (e_2) throw e_2.error; }
        }
        try {
            // If a beforeSend hook removed a property the event needs to be ingested
            // (e.g. a generic token/PII scrubber matching /token/i dropping `token`),
            // ingest would silently reject every such event with a 401. Drop the event
            // here and warn instead - we don't restore the property, since the removal
            // may have been intentional.
            for (var requiredProperties_1 = __values(requiredProperties), requiredProperties_1_1 = requiredProperties_1.next(); !requiredProperties_1_1.done; requiredProperties_1_1 = requiredProperties_1.next()) {
                var property = requiredProperties_1_1.value;
                if (beforeSendResult.properties && (0, core_1.isNullish)(beforeSendResult.properties[property])) {
                    logger_1.logger.warn("Event '".concat(data.event, "' had its '").concat(property, "' property removed in a beforeSend function. This property is required for ingestion, so the event will be dropped."));
                    return null;
                }
            }
        }
        catch (e_3_1) { e_3 = { error: e_3_1 }; }
        finally {
            try {
                if (requiredProperties_1_1 && !requiredProperties_1_1.done && (_b = requiredProperties_1.return)) _b.call(requiredProperties_1);
            }
            finally { if (e_3) throw e_3.error; }
        }
        return beforeSendResult;
    };
    /**
     * Returns the current page view ID.
     *
     * {@label Initialization}
     *
     * @public
     *
     * @returns The current page view ID, or undefined if no page view has been captured.
     */
    PostHog.prototype.getPageViewId = function () {
        var _a;
        return (_a = this.pageViewManager._currentPageview) === null || _a === void 0 ? void 0 : _a.pageViewId;
    };
    /**
     * Capture written user feedback for a LLM trace. Numeric values are converted to strings.
     *
     * {@label LLM analytics}
     *
     * @public
     *
     * @param traceId The trace ID to capture feedback for.
     * @param userFeedback The feedback to capture.
     */
    PostHog.prototype.captureTraceFeedback = function (traceId, userFeedback) {
        this.capture('$ai_feedback', {
            $ai_trace_id: String(traceId),
            $ai_feedback_text: userFeedback,
        });
    };
    /**
     * Capture a metric for a LLM trace. Numeric values are converted to strings.
     *
     * {@label LLM analytics}
     *
     * @public
     *
     * @param traceId The trace ID to capture the metric for.
     * @param metricName The name of the metric to capture.
     * @param metricValue The value of the metric to capture.
     */
    PostHog.prototype.captureTraceMetric = function (traceId, metricName, metricValue) {
        this.capture('$ai_metric', {
            $ai_trace_id: String(traceId),
            $ai_metric_name: metricName,
            $ai_metric_value: String(metricValue),
        });
    };
    PostHog.prototype._checkLocalStorageForDebug = function (debugConfig) {
        var explicitlyFalse = (0, core_1.isBoolean)(debugConfig) && !debugConfig;
        var isTrueInLocalStorage = storage_1.localStore._is_supported() && storage_1.localStore._get('ph_debug') === 'true';
        return explicitlyFalse ? false : isTrueInLocalStorage ? true : debugConfig;
    };
    PostHog.__defaultExtensionClasses = {};
    PostHog._noopLogger = (function () {
        var noop = function () { };
        return { trace: noop, debug: noop, info: noop, warn: noop, error: noop, fatal: noop };
    })();
    return PostHog;
}());
exports.PostHog = PostHog;
(0, general_utils_1.safewrapClass)(PostHog, ['identify']);
var add_dom_loaded_handler = function () {
    // Cross browser DOM Loaded support
    function dom_loaded_handler() {
        // function flag since we only want to execute this once
        if (dom_loaded_handler.done) {
            return;
        }
        ;
        dom_loaded_handler.done = true;
        ENQUEUE_REQUESTS = false;
        (0, general_utils_1.each)(instances, function (inst) {
            inst._dom_loaded();
        });
    }
    if (globals_1.document === null || globals_1.document === void 0 ? void 0 : globals_1.document.addEventListener) {
        if (globals_1.document.readyState === 'complete') {
            // safari 4 can fire the DOMContentLoaded event before loading all
            // external JS (including this file). you will see some copypasta
            // on the internet that checks for 'complete' and 'loaded', but
            // 'loaded' is an IE thing
            dom_loaded_handler();
        }
        else {
            (0, general_utils_1.addEventListener)(globals_1.document, 'DOMContentLoaded', dom_loaded_handler, { capture: false });
        }
        return;
    }
    // Only IE6-8 don't support `document.addEventListener` and we don't support them
    // so let's simply log an error stating PostHog couldn't be initialized
    // We're checking for `window` to avoid erroring out on a SSR context
    if (globals_1.window) {
        logger_1.logger.error("Browser doesn't support `document.addEventListener` so PostHog couldn't be initialized");
    }
};
function init_from_snippet() {
    config_1.default.SDK_DIST_CHANNEL = 'cdn';
    var posthogMain = (instances[PRIMARY_INSTANCE_NAME] = new PostHog());
    var snippetPostHog = globals_2.assignableWindow['posthog'];
    if (snippetPostHog) {
        /**
         * The snippet uses some clever tricks to allow deferred loading of array.js (this code)
         *
         * window.posthog is an array which the queue of calls made before the lib is loaded
         * It has methods attached to it to simulate the posthog object so for instance
         *
         * window.posthog.init("TOKEN", {api_host: "foo" })
         * window.posthog.capture("my-event", {foo: "bar" })
         *
         * ... will mean that window.posthog will look like this:
         * window.posthog == [
         *  ["my-event", {foo: "bar"}]
         * ]
         *
         * window.posthog[_i] == [
         *   ["TOKEN", {api_host: "foo" }, "posthog"]
         * ]
         *
         * If a name is given to the init function then the same as above is true but as a sub-property on the object:
         *
         * window.posthog.init("TOKEN", {}, "ph2")
         * window.posthog.ph2.people.set({foo: "bar"})
         *
         * window.posthog.ph2 == []
         * window.posthog.people == [
         *  ["set", {foo: "bar"}]
         * ]
         *
         */
        // Call all pre-loaded init calls properly
        (0, general_utils_1.each)(snippetPostHog['_i'], function (item) {
            if (item && (0, core_1.isArray)(item)) {
                var instance = posthogMain.init(item[0], item[1], item[2]);
                var instanceSnippet = snippetPostHog[item[2]] || snippetPostHog;
                if (instance) {
                    // Crunch through the people queue first - we queue this data up &
                    // flush on identify, so it's better to do all these operations first
                    instance._execute_array.call(instance.people, instanceSnippet.people);
                    instance._execute_array(instanceSnippet);
                }
            }
        });
    }
    globals_2.assignableWindow['posthog'] = posthogMain;
    add_dom_loaded_handler();
}
function init_as_module() {
    config_1.default.SDK_DIST_CHANNEL = 'npm';
    var posthogMain = (instances[PRIMARY_INSTANCE_NAME] = new PostHog());
    add_dom_loaded_handler();
    return posthogMain;
}
//# sourceMappingURL=posthog-core.js.map