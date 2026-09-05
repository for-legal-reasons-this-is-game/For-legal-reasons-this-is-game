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
Object.defineProperty(exports, "__esModule", { value: true });
exports.PostHogSurveys = void 0;
var constants_1 = require("./constants");
var SURVEY_NOT_LOADED = 'SDK is not enabled or survey functionality is not yet loaded';
var SURVEY_DISABLED = 'Disabled. Not loading surveys.';
var posthog_surveys_types_1 = require("./posthog-surveys-types");
var globals_1 = require("@posthog/browser-common/utils/globals");
var globals_2 = require("./utils/globals");
var survey_event_receiver_1 = require("./utils/survey-event-receiver");
var survey_utils_1 = require("./utils/survey-utils");
var core_1 = require("@posthog/core");
var PostHogSurveys = /** @class */ (function () {
    function PostHogSurveys(_instance) {
        this._instance = _instance;
        // this is set to undefined until the remote config is loaded
        // then it's set to true if there are surveys to load
        // or false if there are no surveys to load
        // or false if the surveys feature is disabled in the project settings
        this._isSurveysEnabled = undefined;
        this._surveyManager = null;
        this._isInitializingSurveys = false;
        this._surveyCallbacks = [];
        // Promise for in-flight survey fetch - allows multiple callers to await the same request
        this._getSurveysInFlightPromise = null;
        // Backs off the stale-cache refresh for one TTL after a failure, so a surveys-API outage can't
        // turn the ~1s display poll into a per-poll request storm.
        this._lastSurveyRefreshFailedAt = null;
        // we set this to undefined here because we need the persistence storage for this type
        // but that's not initialized until loadIfEnabled is called.
        this._surveyEventReceiver = null;
    }
    Object.defineProperty(PostHogSurveys.prototype, "_config", {
        get: function () {
            return this._instance.config;
        },
        enumerable: false,
        configurable: true
    });
    PostHogSurveys.prototype.initialize = function () {
        this.loadIfEnabled();
    };
    PostHogSurveys.prototype.onRemoteConfig = function (result) {
        // only load surveys if they are enabled and there are surveys to load
        if (this._config.disable_surveys) {
            return;
        }
        if (!result.ok) {
            // Failure behaves like a response without a surveys key: not loaded.
            return survey_utils_1.SURVEY_LOGGER.warn('Remote config unavailable. Not loading surveys.');
        }
        var surveys = result.config['surveys'];
        if ((0, core_1.isNullish)(surveys)) {
            return survey_utils_1.SURVEY_LOGGER.warn('Flags not loaded yet. Not loading surveys.');
        }
        var isArrayResponse = (0, core_1.isArray)(surveys);
        this._isSurveysEnabled = isArrayResponse ? surveys.length > 0 : surveys;
        survey_utils_1.SURVEY_LOGGER.info("flags response received, isSurveysEnabled: ".concat(this._isSurveysEnabled));
        this.loadIfEnabled();
    };
    PostHogSurveys.prototype.reset = function () {
        var _a;
        try {
            // Drop in-memory event/action activations too; they aren't in persistence (which
            // reset() has already cleared), so without this an armed-but-unshown survey would
            // survive a logout/account switch that doesn't reload the page.
            (_a = this._surveyEventReceiver) === null || _a === void 0 ? void 0 : _a.reset();
            localStorage.removeItem('lastSeenSurveyDate');
            var surveyKeys = [];
            for (var i = 0; i < localStorage.length; i++) {
                var key = localStorage.key(i);
                if ((key === null || key === void 0 ? void 0 : key.startsWith(survey_utils_1.SURVEY_SEEN_PREFIX)) || (key === null || key === void 0 ? void 0 : key.startsWith(survey_utils_1.SURVEY_IN_PROGRESS_PREFIX))) {
                    surveyKeys.push(key);
                }
            }
            surveyKeys.forEach(function (key) { return localStorage.removeItem(key); });
        }
        catch (_b) {
            // localStorage is not always available (e.g. in cross-origin iframes); resetting survey state is best-effort.
        }
    };
    PostHogSurveys.prototype.loadIfEnabled = function () {
        var _this = this;
        // Initial guard clauses
        if (this._surveyManager) {
            return;
        } // Already loaded
        if (this._isInitializingSurveys) {
            survey_utils_1.SURVEY_LOGGER.info('Already initializing surveys, skipping...');
            return;
        }
        if (this._config.disable_surveys) {
            survey_utils_1.SURVEY_LOGGER.info(SURVEY_DISABLED);
            return;
        }
        if (this._config.cookieless_mode && this._instance.consent.isOptedOut()) {
            survey_utils_1.SURVEY_LOGGER.info('Not loading surveys in cookieless mode without consent.');
            return;
        }
        var phExtensions = globals_2.assignableWindow === null || globals_2.assignableWindow === void 0 ? void 0 : globals_2.assignableWindow.__PosthogExtensions__;
        if (!phExtensions) {
            survey_utils_1.SURVEY_LOGGER.error('PostHog Extensions not found.');
            return;
        }
        // waiting for remote config to load
        // if surveys is forced enable (like external surveys), ignore the remote config and load surveys
        if ((0, core_1.isUndefined)(this._isSurveysEnabled) && !this._config.advanced_enable_surveys) {
            return;
        }
        var isSurveysEnabled = this._isSurveysEnabled || this._config.advanced_enable_surveys;
        this._isInitializingSurveys = true;
        try {
            var generateSurveys = phExtensions.generateSurveys;
            if (generateSurveys) {
                // Surveys code is already loaded
                this._completeSurveyInitialization(generateSurveys, isSurveysEnabled);
                return;
            }
            // If we reach here, surveys code is not loaded yet
            var loadExternalDependency = phExtensions.loadExternalDependency;
            if (!loadExternalDependency) {
                // Cannot load surveys code
                this._handleSurveyLoadError(constants_1.LOAD_EXT_NOT_FOUND);
                return;
            }
            // If we reach here, we need to load the dependency
            loadExternalDependency(this._instance, 'surveys', function (err) {
                if (err || !phExtensions.generateSurveys) {
                    _this._handleSurveyLoadError('Could not load surveys script', err);
                }
                else {
                    // Need to get the function reference again inside the callback
                    _this._completeSurveyInitialization(phExtensions.generateSurveys, isSurveysEnabled);
                }
            });
        }
        catch (e) {
            this._handleSurveyLoadError('Error initializing surveys', e);
            throw e;
        }
        finally {
            // Ensure the flag is always reset
            this._isInitializingSurveys = false;
        }
    };
    /** Helper to finalize survey initialization */
    PostHogSurveys.prototype._completeSurveyInitialization = function (generateSurveysFn, isSurveysEnabled) {
        this._surveyManager = generateSurveysFn(this._instance, isSurveysEnabled);
        this._surveyEventReceiver = new survey_event_receiver_1.SurveyEventReceiver(this._instance);
        survey_utils_1.SURVEY_LOGGER.info('Surveys loaded successfully');
        this._notifySurveyCallbacks({ isLoaded: true });
    };
    /** Helper to handle errors during survey loading */
    PostHogSurveys.prototype._handleSurveyLoadError = function (message, error) {
        survey_utils_1.SURVEY_LOGGER.error(message, error);
        this._notifySurveyCallbacks({ isLoaded: false, error: message });
    };
    /**
     * Register a callback that runs when surveys are initialized.
     * ### Usage:
     *
     *     posthog.onSurveysLoaded((surveys) => {
     *         // You can work with all surveys
     *         console.log('All available surveys:', surveys)
     *
     *         // Or get active matching surveys
     *         posthog.getActiveMatchingSurveys((activeMatchingSurveys) => {
     *             if (activeMatchingSurveys.length > 0) {
     *                 posthog.renderSurvey(activeMatchingSurveys[0].id, '#survey-container')
     *             }
     *         })
     *     })
     *
     * @param {Function} callback The callback function will be called when surveys are loaded or updated.
     *                           It receives the array of all surveys and a context object with error status.
     * @returns {Function} A function that can be called to unsubscribe the listener.
     */
    PostHogSurveys.prototype.onSurveysLoaded = function (callback) {
        var _this = this;
        this._surveyCallbacks.push(callback);
        if (this._surveyManager) {
            this._notifySurveyCallbacks({
                isLoaded: true,
            });
        }
        // Return unsubscribe function
        return function () {
            _this._surveyCallbacks = _this._surveyCallbacks.filter(function (cb) { return cb !== callback; });
        };
    };
    PostHogSurveys.prototype.getSurveys = function (callback, forceReload) {
        var _this = this;
        if (forceReload === void 0) { forceReload = false; }
        // In case we manage to load the surveys script, but config says not to load surveys
        // then we shouldn't return survey data
        if (this._config.disable_surveys) {
            survey_utils_1.SURVEY_LOGGER.info(SURVEY_DISABLED);
            return callback([]);
        }
        var existingSurveys = this._instance.get_property(constants_1.SURVEYS);
        if (existingSurveys && !forceReload) {
            // Serve the cached definitions synchronously so callers that rely on a synchronous
            // callback (e.g. _getSurveyById) keep working.
            callback(existingSurveys, {
                isLoaded: true,
            });
            // If the cache has aged past its TTL, kick off a background refresh so server-side
            // changes (e.g. a survey switched from popover to API) reach a long-lived tab. The
            // next poll then evaluates the refreshed definitions.
            if (this._shouldBackgroundRefreshSurveys()) {
                this.getSurveys(function () { }, true);
            }
            return;
        }
        // If a fetch is already in progress and Promise is available, reuse that promise
        // In browsers without Promise (IE11), we skip this optimization and just make concurrent requests
        if (typeof Promise !== 'undefined' && this._getSurveysInFlightPromise) {
            this._getSurveysInFlightPromise.then(function (_a) {
                var surveys = _a.surveys, context = _a.context;
                return callback(surveys, context);
            });
            return;
        }
        // Create a new promise for this fetch that other callers can reuse
        // We need to assign the promise before starting the request, because
        // in tests (and potentially in some edge cases) the callback may fire synchronously
        var resolvePromise;
        if (typeof Promise !== 'undefined') {
            this._getSurveysInFlightPromise = new Promise(function (resolve) {
                resolvePromise = resolve;
            });
        }
        this._instance._send_request({
            url: this._instance.requestRouter.endpointFor('api', "/api/surveys/?token=".concat(this._config.token)),
            method: 'GET',
            timestampMode: 'query',
            timeout: this._config.surveys_request_timeout_ms,
            callback: function (response) {
                var _a;
                var _b, _c;
                _this._getSurveysInFlightPromise = null;
                var statusCode = response.statusCode;
                if (statusCode !== 200 || !response.json) {
                    var error = "Surveys API could not be loaded, status: ".concat(statusCode);
                    survey_utils_1.SURVEY_LOGGER.error(error);
                    _this._lastSurveyRefreshFailedAt = Date.now();
                    var context_1 = { isLoaded: false, error: error };
                    callback([], context_1);
                    resolvePromise === null || resolvePromise === void 0 ? void 0 : resolvePromise({ surveys: [], context: context_1 });
                    return;
                }
                _this._lastSurveyRefreshFailedAt = null;
                var surveys = response.json.surveys || [];
                var eventOrActionBasedSurveys = surveys.filter(function (survey) {
                    return (0, survey_utils_1.isSurveyRunning)(survey) &&
                        ((0, survey_utils_1.doesSurveyActivateByEvent)(survey) || (0, survey_utils_1.doesSurveyActivateByAction)(survey));
                });
                if (eventOrActionBasedSurveys.length > 0) {
                    (_b = _this._surveyEventReceiver) === null || _b === void 0 ? void 0 : _b.register(eventOrActionBasedSurveys);
                }
                // Stamp when these definitions were fetched so the split-storage
                // loader can tell a fresher main-blob write-back from a stale
                // `__surveys` entry (the survey analogue of $feature_flag_evaluated_at).
                (_c = _this._instance.persistence) === null || _c === void 0 ? void 0 : _c.register((_a = {}, _a[constants_1.SURVEYS] = surveys, _a[constants_1.SURVEYS_LOADED_AT] = Date.now(), _a));
                var context = { isLoaded: true };
                callback(surveys, context);
                resolvePromise === null || resolvePromise === void 0 ? void 0 : resolvePromise({ surveys: surveys, context: context });
            },
        });
    };
    /**
     * Whether to kick off a background refresh of the cached definitions: the cache is stale, no
     * fetch is already in flight, and we're not backing off after a recent failure.
     */
    PostHogSurveys.prototype._shouldBackgroundRefreshSurveys = function () {
        return this._isSurveyCacheStale() && !this._getSurveysInFlightPromise && !this._isSurveyRefreshBackingOff();
    };
    /**
     * Whether the cached `$surveys` definitions have aged past their TTL. Returns false when no
     * timestamp is recorded (e.g. surveys injected directly in tests) so the cache stays valid.
     */
    PostHogSurveys.prototype._isSurveyCacheStale = function () {
        var surveysLoadedAt = this._instance.get_property(constants_1.SURVEYS_LOADED_AT);
        return (0, core_1.isNumber)(surveysLoadedAt) && Date.now() - surveysLoadedAt > constants_1.SURVEYS_CACHE_TTL_MS;
    };
    PostHogSurveys.prototype._isSurveyRefreshBackingOff = function () {
        return ((0, core_1.isNumber)(this._lastSurveyRefreshFailedAt) &&
            Date.now() - this._lastSurveyRefreshFailedAt < constants_1.SURVEYS_REFRESH_BACKOFF_MS);
    };
    /**
     * Marks a survey as seen for the current device, mirroring the local state the SDK records
     * when it shows or sends a survey itself.
     *
     * Use this when you display surveys through your own backend/integration (so the SDK never
     * captures the `survey shown`/`sent`/`dismissed` events) and still want PostHog's display
     * logic to honour the "already seen" and wait-period checks on subsequent page loads.
     *
     * Note: surveys configured to repeat (`schedule: 'always'` or event `repeatedActivation`)
     * intentionally bypass the seen check, so marking them as seen will not stop them showing.
     *
     * @param surveyId The ID of the survey to mark as seen.
     * @param options Optional settings. `iteration` is the survey's current iteration number, if any.
     */
    PostHogSurveys.prototype.markSurveyAsSeen = function (surveyId, options) {
        var _a;
        var survey = { id: surveyId, current_iteration: (_a = options === null || options === void 0 ? void 0 : options.iteration) !== null && _a !== void 0 ? _a : null };
        (0, survey_utils_1.setSurveySeenOnLocalStorage)(survey);
        try {
            localStorage.setItem('lastSeenSurveyDate', new Date().toISOString());
        }
        catch (_b) {
            // localStorage is not always available (e.g. in cross-origin iframes); best-effort only.
        }
    };
    /** Helper method to notify all registered callbacks */
    PostHogSurveys.prototype._notifySurveyCallbacks = function (context) {
        var e_1, _a;
        try {
            for (var _b = __values(this._surveyCallbacks), _c = _b.next(); !_c.done; _c = _b.next()) {
                var callback = _c.value;
                try {
                    if (!context.isLoaded) {
                        return callback([], context);
                    }
                    this.getSurveys(callback);
                }
                catch (error) {
                    survey_utils_1.SURVEY_LOGGER.error('Error in survey callback', error);
                }
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
    };
    PostHogSurveys.prototype.getActiveMatchingSurveys = function (callback, forceReload) {
        if (forceReload === void 0) { forceReload = false; }
        if ((0, core_1.isNullish)(this._surveyManager)) {
            survey_utils_1.SURVEY_LOGGER.warn('init was not called');
            return;
        }
        return this._surveyManager.getActiveMatchingSurveys(callback, forceReload);
    };
    PostHogSurveys.prototype._getSurveyById = function (surveyId) {
        var survey = null;
        this.getSurveys(function (surveys) {
            var _a;
            survey = (_a = surveys.find(function (x) { return x.id === surveyId; })) !== null && _a !== void 0 ? _a : null;
        });
        return survey;
    };
    PostHogSurveys.prototype._checkSurveyEligibility = function (surveyId) {
        if ((0, core_1.isNullish)(this._surveyManager)) {
            return { eligible: false, reason: SURVEY_NOT_LOADED };
        }
        var survey = typeof surveyId === 'string' ? this._getSurveyById(surveyId) : surveyId;
        if (!survey) {
            return { eligible: false, reason: 'Survey not found' };
        }
        return this._surveyManager.checkSurveyEligibility(survey);
    };
    PostHogSurveys.prototype._checkSurveyRenderability = function (surveyId) {
        if ((0, core_1.isNullish)(this._surveyManager)) {
            return { eligible: false, reason: SURVEY_NOT_LOADED };
        }
        var survey = typeof surveyId === 'string' ? this._getSurveyById(surveyId) : surveyId;
        if (!survey) {
            return { eligible: false, reason: 'Survey not found' };
        }
        return this._surveyManager.checkSurveyRenderability(survey);
    };
    PostHogSurveys.prototype.canRenderSurvey = function (surveyId) {
        if ((0, core_1.isNullish)(this._surveyManager)) {
            survey_utils_1.SURVEY_LOGGER.warn('init was not called');
            return { visible: false, disabledReason: SURVEY_NOT_LOADED };
        }
        var eligibility = this._checkSurveyRenderability(surveyId);
        return { visible: eligibility.eligible, disabledReason: eligibility.reason };
    };
    PostHogSurveys.prototype.canRenderSurveyAsync = function (surveyId, forceReload) {
        var _this = this;
        // Ensure surveys are loaded before checking
        // Using Promise to wrap the callback-based getSurveys method
        if ((0, core_1.isNullish)(this._surveyManager)) {
            survey_utils_1.SURVEY_LOGGER.warn('init was not called');
            return Promise.resolve({
                visible: false,
                disabledReason: SURVEY_NOT_LOADED,
            });
        }
        // eslint-disable-next-line compat/compat
        return new Promise(function (resolve) {
            _this.getSurveys(function (surveys) {
                var _a;
                var survey = (_a = surveys.find(function (x) { return x.id === surveyId; })) !== null && _a !== void 0 ? _a : null;
                if (!survey) {
                    resolve({ visible: false, disabledReason: 'Survey not found' });
                }
                else {
                    var eligibility = _this._checkSurveyRenderability(survey);
                    resolve({ visible: eligibility.eligible, disabledReason: eligibility.reason });
                }
            }, forceReload);
        });
    };
    PostHogSurveys.prototype.renderSurvey = function (surveyId, selector, properties) {
        var _this = this;
        var _a;
        if ((0, core_1.isNullish)(this._surveyManager)) {
            survey_utils_1.SURVEY_LOGGER.warn('init was not called');
            return;
        }
        var survey = typeof surveyId === 'string' ? this._getSurveyById(surveyId) : surveyId;
        if (!(survey === null || survey === void 0 ? void 0 : survey.id)) {
            survey_utils_1.SURVEY_LOGGER.warn('Survey not found');
            return;
        }
        if (!survey_utils_1.IN_APP_SURVEY_TYPES.includes(survey.type)) {
            survey_utils_1.SURVEY_LOGGER.warn("Surveys of type ".concat(survey.type, " cannot be rendered in the app"));
            return;
        }
        var elem = globals_1.document === null || globals_1.document === void 0 ? void 0 : globals_1.document.querySelector(selector);
        if (!elem) {
            survey_utils_1.SURVEY_LOGGER.warn('Survey element not found');
            return;
        }
        if ((_a = survey.appearance) === null || _a === void 0 ? void 0 : _a.surveyPopupDelaySeconds) {
            survey_utils_1.SURVEY_LOGGER.info("Rendering survey ".concat(survey.id, " with delay of ").concat(survey.appearance.surveyPopupDelaySeconds, " seconds"));
            setTimeout(function () {
                var _a, _b;
                survey_utils_1.SURVEY_LOGGER.info("Rendering survey ".concat(survey.id, " with delay of ").concat((_a = survey.appearance) === null || _a === void 0 ? void 0 : _a.surveyPopupDelaySeconds, " seconds"));
                (_b = _this._surveyManager) === null || _b === void 0 ? void 0 : _b.renderSurvey(survey, elem, properties);
                survey_utils_1.SURVEY_LOGGER.info("Survey ".concat(survey.id, " rendered"));
            }, survey.appearance.surveyPopupDelaySeconds * 1000);
            return;
        }
        this._surveyManager.renderSurvey(survey, elem, properties);
    };
    PostHogSurveys.prototype.displaySurvey = function (surveyId, options) {
        var _a;
        if ((0, core_1.isNullish)(this._surveyManager)) {
            survey_utils_1.SURVEY_LOGGER.warn('init was not called');
            return;
        }
        var survey = this._getSurveyById(surveyId);
        if (!survey) {
            survey_utils_1.SURVEY_LOGGER.warn('Survey not found');
            return;
        }
        var surveyToDisplay = survey;
        if (((_a = survey.appearance) === null || _a === void 0 ? void 0 : _a.surveyPopupDelaySeconds) && options.ignoreDelay) {
            surveyToDisplay = __assign(__assign({}, survey), { appearance: __assign(__assign({}, survey.appearance), { surveyPopupDelaySeconds: 0 }) });
        }
        if (options.displayType !== posthog_surveys_types_1.DisplaySurveyType.Popover && options.initialResponses) {
            survey_utils_1.SURVEY_LOGGER.warn('initialResponses is only supported for popover surveys. prefill will not be applied.');
        }
        if (options.ignoreConditions === false) {
            // Explicit display goes through eligibility only, not renderability: the event/action
            // trigger state lives in memory and is irrelevant when the caller asks to show the survey.
            var eligibility = this._checkSurveyEligibility(survey);
            if (!eligibility.eligible) {
                survey_utils_1.SURVEY_LOGGER.warn('Survey is not eligible to be displayed: ', eligibility.reason);
                return;
            }
        }
        if (options.displayType === posthog_surveys_types_1.DisplaySurveyType.Inline) {
            this.renderSurvey(surveyToDisplay, options.selector, options.properties);
            return;
        }
        this._surveyManager.handlePopoverSurvey(surveyToDisplay, options);
    };
    PostHogSurveys.prototype.cancelPendingSurvey = function (surveyId) {
        if ((0, core_1.isNullish)(this._surveyManager)) {
            survey_utils_1.SURVEY_LOGGER.warn('init was not called');
            return;
        }
        this._surveyManager.cancelSurvey(surveyId);
    };
    PostHogSurveys.prototype.handlePageUnload = function () {
        var _a, _b;
        (_b = (_a = this._surveyManager) === null || _a === void 0 ? void 0 : _a.handlePageUnload) === null || _b === void 0 ? void 0 : _b.call(_a);
    };
    return PostHogSurveys;
}());
exports.PostHogSurveys = PostHogSurveys;
//# sourceMappingURL=posthog-surveys.js.map