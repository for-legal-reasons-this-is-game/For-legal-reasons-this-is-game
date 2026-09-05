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
var globals_1 = require("@posthog/browser-common/utils/globals");
var globals_2 = require("../utils/globals");
var core_1 = require("@posthog/core");
var autocapture_utils_1 = require("@posthog/browser-common/utils/autocapture-utils");
var autocapture_1 = require("../autocapture");
var element_utils_1 = require("@posthog/browser-common/utils/element-utils");
var prototype_utils_1 = require("@posthog/browser-common/utils/prototype-utils");
var general_utils_1 = require("@posthog/browser-common/utils/general-utils");
function asCandidate(event, extra) {
    var eventTarget = (0, autocapture_utils_1.getEventTarget)(event);
    if (eventTarget) {
        return __assign({ node: eventTarget, originalEvent: event, timestamp: Date.now() }, extra);
    }
    return null;
}
function swipeDirection(dx, dy) {
    if (Math.abs(dx) >= Math.abs(dy)) {
        return dx >= 0 ? 'right' : 'left';
    }
    return dy >= 0 ? 'down' : 'up';
}
// the event name doubles as the property prefix ($dead_click_* / $dead_swipe_*)
function deadEventName(candidate) {
    return candidate.type === 'swipe' ? '$dead_swipe' : '$dead_click';
}
// surfaces that respond to a gesture without anything our observers can see — a canvas or
// WebGL repaint is not a DOM mutation, and native media controls live in a closed UA shadow
// root — so a swipe over them can never be judged dead
var UNOBSERVABLE_SURFACE_SELECTOR = 'canvas,video,audio,embed,object';
function hasModifierKey(event) {
    return event.ctrlKey || event.metaKey || event.altKey || event.shiftKey;
}
function checkTimeout(value, thresholdMs) {
    return (0, core_1.isNumber)(value) && value >= thresholdMs;
}
var LazyLoadedDeadClicksAutocapture = /** @class */ (function () {
    function LazyLoadedDeadClicksAutocapture(instance, config) {
        var _this = this;
        this.instance = instance;
        this._clicks = [];
        this._deadSwipesCaptured = 0;
        this._defaultConfig = function (defaultOnCapture) { return ({
            element_attribute_ignorelist: [],
            scroll_threshold_ms: 100,
            selection_change_threshold_ms: 100,
            mutation_threshold_ms: 2500,
            capture_clicks_with_modifier_keys: false,
            capture_dead_swipes: true,
            swipe_threshold_px: 30,
            max_dead_swipes_per_page_load: 10,
            __onCapture: defaultOnCapture,
        }); };
        this._onClick = function (event) {
            var click = asCandidate(event, { type: 'click' });
            if (!(0, core_1.isNull)(click) && !_this._ignore(click)) {
                _this._clicks.push(click);
            }
            _this._scheduleCheck();
        };
        this._onScroll = function () {
            var candidateNow = Date.now();
            _this._lastScroll = candidateNow;
            // very naive throttle
            if (candidateNow % 50 === 0) {
                // we can see many scrolls between scheduled checks,
                // so we update scroll delay as we see them
                // to avoid false positives
                _this._clicks.forEach(function (click) {
                    if ((0, core_1.isUndefined)(click.scrollDelayMs)) {
                        click.scrollDelayMs = candidateNow - click.timestamp;
                    }
                });
            }
        };
        this._onSelectionChange = function () {
            _this._lastSelectionChanged = Date.now();
        };
        this._onVisibilityChange = function () {
            if ((globals_1.document === null || globals_1.document === void 0 ? void 0 : globals_1.document.visibilityState) === 'visible') {
                _this._lastVisibilityChange = Date.now();
            }
        };
        this._onTouchStart = function (event) {
            var touches = event.touches;
            // only single-finger gestures are swipes; a second finger (pinch/zoom) is not,
            // so a multi-touch start clears any tracked origin rather than measuring against it
            var touch = (touches === null || touches === void 0 ? void 0 : touches.length) === 1 ? touches[0] : undefined;
            _this._touchStart = touch ? { x: touch.clientX, y: touch.clientY, timestamp: Date.now() } : undefined;
        };
        this._onTouchCancel = function () {
            _this._touchStart = undefined;
        };
        this._onTouchEnd = function (event) {
            var _a, _b, _c;
            var start = _this._touchStart;
            _this._touchStart = undefined;
            if ((0, core_1.isUndefined)(start) || _this._deadSwipesCaptured >= _this._config.max_dead_swipes_per_page_load) {
                return;
            }
            var touchEvent = event;
            var touch = (_a = touchEvent.changedTouches) === null || _a === void 0 ? void 0 : _a[0];
            if (!touch) {
                return;
            }
            var dx = touch.clientX - start.x;
            var dy = touch.clientY - start.y;
            var distanceSq = dx * dx + dy * dy;
            // compare squared distances to avoid a sqrt on every touch; only the actual swipes need it
            var threshold = _this._config.swipe_threshold_px;
            if (distanceSq < threshold * threshold) {
                // a short movement is a tap or a jitter, not a swipe
                return;
            }
            // a gesture that already did something while the finger was down — dragging scrolled
            // the page, a finger-following carousel mutated the DOM, a text selection grew — is not
            // dead, so it never becomes a candidate. surviving candidates keep the touchend
            // timestamp: the thresholds in _checkClicks measure how quickly the page responds
            // after the finger lifts, just as they measure the response to a click
            var gestureCausedActivity = [_this._lastScroll, _this._lastMutation, _this._lastSelectionChanged].some(function (lastActivityAt) { return (0, core_1.isNumber)(lastActivityAt) && lastActivityAt >= start.timestamp; });
            if (gestureCausedActivity) {
                return;
            }
            // hit-test the stack under where the finger landed rather than the candidate's
            // ancestors: canvas apps usually receive their touches through transparent overlay
            // elements stacked on top of the canvas, so the surface is beneath, not above.
            // this runs at most once per dead-candidate gesture, when the page is provably idle
            // (the activity gate above just passed), so a forced layout flush is unlikely
            if (_this._pageHasUnobservableSurfaces()) {
                var stack = (_c = (_b = globals_1.document === null || globals_1.document === void 0 ? void 0 : globals_1.document.elementsFromPoint) === null || _b === void 0 ? void 0 : _b.call(globals_1.document, start.x, start.y)) !== null && _c !== void 0 ? _c : [];
                if (stack.some(function (el) { var _a; return (_a = el.matches) === null || _a === void 0 ? void 0 : _a.call(el, UNOBSERVABLE_SURFACE_SELECTOR); })) {
                    return;
                }
            }
            var swipe = asCandidate(touchEvent, {
                type: 'swipe',
                swipeDirection: swipeDirection(dx, dy),
                swipeDistancePx: Math.round(Math.sqrt(distanceSq)),
            });
            if (!(0, core_1.isNull)(swipe) && !_this._ignore(swipe)) {
                _this._clicks.push(swipe);
            }
            _this._scheduleCheck();
        };
        this._config = this._asRequiredConfig(config);
        this._onCapture = this._config.__onCapture;
    }
    LazyLoadedDeadClicksAutocapture.prototype._asRequiredConfig = function (providedConfig) {
        var _a, _b, _c, _d, _e, _f, _g, _h;
        var defaultConfig = this._defaultConfig((providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.__onCapture) || this._captureDeadClick.bind(this));
        return {
            element_attribute_ignorelist: (_a = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.element_attribute_ignorelist) !== null && _a !== void 0 ? _a : defaultConfig.element_attribute_ignorelist,
            scroll_threshold_ms: (_b = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.scroll_threshold_ms) !== null && _b !== void 0 ? _b : defaultConfig.scroll_threshold_ms,
            selection_change_threshold_ms: (_c = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.selection_change_threshold_ms) !== null && _c !== void 0 ? _c : defaultConfig.selection_change_threshold_ms,
            mutation_threshold_ms: (_d = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.mutation_threshold_ms) !== null && _d !== void 0 ? _d : defaultConfig.mutation_threshold_ms,
            capture_clicks_with_modifier_keys: (_e = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.capture_clicks_with_modifier_keys) !== null && _e !== void 0 ? _e : defaultConfig.capture_clicks_with_modifier_keys,
            capture_dead_swipes: (_f = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.capture_dead_swipes) !== null && _f !== void 0 ? _f : defaultConfig.capture_dead_swipes,
            swipe_threshold_px: (_g = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.swipe_threshold_px) !== null && _g !== void 0 ? _g : defaultConfig.swipe_threshold_px,
            max_dead_swipes_per_page_load: (_h = providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.max_dead_swipes_per_page_load) !== null && _h !== void 0 ? _h : defaultConfig.max_dead_swipes_per_page_load,
            css_selector_ignorelist: providedConfig === null || providedConfig === void 0 ? void 0 : providedConfig.css_selector_ignorelist,
            __onCapture: defaultConfig.__onCapture,
        };
    };
    LazyLoadedDeadClicksAutocapture.prototype.start = function (observerTarget) {
        this._startClickObserver();
        this._startScrollObserver();
        this._startSelectionChangedObserver();
        this._startVisibilityChangeObserver();
        this._startMutationObserver(observerTarget);
        if (this._config.capture_dead_swipes) {
            this._startSwipeObserver();
        }
    };
    LazyLoadedDeadClicksAutocapture.prototype._startMutationObserver = function (observerTarget) {
        var _this = this;
        if (!this._mutationObserver) {
            var NativeMutationObserver = (0, prototype_utils_1.getNativeMutationObserverImplementation)(globals_2.assignableWindow);
            this._mutationObserver = new NativeMutationObserver(function (mutations) {
                _this._onMutation(mutations);
            });
            this._mutationObserver.observe(observerTarget, {
                attributes: true,
                characterData: true,
                childList: true,
                subtree: true,
            });
        }
    };
    LazyLoadedDeadClicksAutocapture.prototype.stop = function () {
        var _a;
        (_a = this._mutationObserver) === null || _a === void 0 ? void 0 : _a.disconnect();
        this._mutationObserver = undefined;
        globals_2.assignableWindow.removeEventListener('click', this._onClick);
        globals_2.assignableWindow.removeEventListener('scroll', this._onScroll, { capture: true });
        globals_2.assignableWindow.removeEventListener('selectionchange', this._onSelectionChange);
        globals_2.assignableWindow.removeEventListener('touchstart', this._onTouchStart, { capture: true });
        globals_2.assignableWindow.removeEventListener('touchend', this._onTouchEnd, { capture: true });
        globals_2.assignableWindow.removeEventListener('touchcancel', this._onTouchCancel, { capture: true });
        globals_1.document === null || globals_1.document === void 0 ? void 0 : globals_1.document.removeEventListener('visibilitychange', this._onVisibilityChange);
        // so a gesture in flight when we stopped can't pair with a touchend after a restart
        this._touchStart = undefined;
    };
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    LazyLoadedDeadClicksAutocapture.prototype._onMutation = function (_mutations) {
        // we don't actually care about the content of the mutations, right now
        this._lastMutation = Date.now();
    };
    LazyLoadedDeadClicksAutocapture.prototype._startClickObserver = function () {
        (0, general_utils_1.addEventListener)(globals_2.assignableWindow, 'click', this._onClick);
    };
    LazyLoadedDeadClicksAutocapture.prototype._scheduleCheck = function () {
        var _this = this;
        if (this._clicks.length && (0, core_1.isUndefined)(this._checkClickTimer)) {
            this._checkClickTimer = globals_2.assignableWindow.setTimeout(function () {
                _this._checkClicks();
            }, 1000);
        }
    };
    // `capture: true` is required to get scroll events for other scrollable elements
    // on the page, not just the window
    // see https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener#usecapture
    //
    // `passive: true` is used to tell the browser that the scroll event handler will not call `preventDefault()`
    // This allows the browser to optimize scrolling performance by not waiting for our handling of the scroll event
    // see https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener#passive
    LazyLoadedDeadClicksAutocapture.prototype._startScrollObserver = function () {
        (0, general_utils_1.addEventListener)(globals_2.assignableWindow, 'scroll', this._onScroll, { capture: true });
    };
    LazyLoadedDeadClicksAutocapture.prototype._startSelectionChangedObserver = function () {
        (0, general_utils_1.addEventListener)(globals_2.assignableWindow, 'selectionchange', this._onSelectionChange);
    };
    LazyLoadedDeadClicksAutocapture.prototype._startVisibilityChangeObserver = function () {
        (0, general_utils_1.addEventListener)(globals_1.document, 'visibilitychange', this._onVisibilityChange);
    };
    // `capture: true` mirrors the scroll observer so we see gestures on nested scrollable
    // elements too. `passive: true` tells the browser we won't call `preventDefault()`.
    LazyLoadedDeadClicksAutocapture.prototype._startSwipeObserver = function () {
        (0, general_utils_1.addEventListener)(globals_2.assignableWindow, 'touchstart', this._onTouchStart, { capture: true, passive: true });
        (0, general_utils_1.addEventListener)(globals_2.assignableWindow, 'touchend', this._onTouchEnd, { capture: true, passive: true });
        // a cancelled gesture (system back-swipe, incoming call, etc.) never fires touchend,
        // so clear the start point to avoid measuring the next gesture from a stale origin
        (0, general_utils_1.addEventListener)(globals_2.assignableWindow, 'touchcancel', this._onTouchCancel, { capture: true, passive: true });
    };
    // whether the page contains surfaces whose response to a gesture we cannot observe —
    // cached, recomputed lazily and only when the DOM has mutated since the last look
    LazyLoadedDeadClicksAutocapture.prototype._pageHasUnobservableSurfaces = function () {
        var domChangedSinceCheck = (0, core_1.isUndefined)(this._surfacesCheckedAt) ||
            ((0, core_1.isNumber)(this._lastMutation) && this._lastMutation > this._surfacesCheckedAt);
        if ((0, core_1.isUndefined)(this._hasUnobservableSurfaces) || domChangedSinceCheck) {
            this._surfacesCheckedAt = Date.now();
            this._hasUnobservableSurfaces = !!(globals_1.document === null || globals_1.document === void 0 ? void 0 : globals_1.document.querySelector(UNOBSERVABLE_SURFACE_SELECTOR));
        }
        return !!this._hasUnobservableSurfaces;
    };
    LazyLoadedDeadClicksAutocapture.prototype._ignore = function (candidate) {
        if (!candidate) {
            return true;
        }
        // swipes run only the shared gates; clicks layer their own intent gates on top
        return candidate.type === 'swipe' ? this._ignoreCommon(candidate) : this._ignoreClick(candidate);
    };
    // gates that apply to every dead candidate, whatever the gesture
    LazyLoadedDeadClicksAutocapture.prototype._ignoreCommon = function (candidate) {
        if ((0, element_utils_1.isElementInToolbar)(candidate.node)) {
            return true;
        }
        var alreadySeenInLastSecond = this._clicks.some(function (c) {
            return (c.type === candidate.type &&
                c.node === candidate.node &&
                Math.abs(c.timestamp - candidate.timestamp) < 1000);
        });
        if (alreadySeenInLastSecond) {
            return true;
        }
        if ((0, element_utils_1.isTag)(candidate.node, 'html') || !(0, element_utils_1.isElementNode)(candidate.node)) {
            return true;
        }
        return !(0, autocapture_utils_1.shouldCaptureDeadClick)(candidate.node, {
            css_selector_ignorelist: this._config.css_selector_ignorelist,
        });
    };
    // click-only gates, layered on top of the shared ones
    LazyLoadedDeadClicksAutocapture.prototype._ignoreClick = function (candidate) {
        // clicks with modifier keys (open in new tab, etc.) are intentional; touch swipes have no modifiers
        if (!this._config.capture_clicks_with_modifier_keys && hasModifierKey(candidate.originalEvent)) {
            return true;
        }
        if (this._ignoreCommon(candidate)) {
            return true;
        }
        // an anchor is a legitimate click activation so we skip it, but a swipe that fails to
        // navigate is the signal we want — which is why anchors are skipped for clicks only.
        // reaching here means _ignoreCommon confirmed an element node, so this is safe to call
        return (0, autocapture_utils_1.shouldSkipDeadClick)(candidate.node);
    };
    LazyLoadedDeadClicksAutocapture.prototype._checkClicks = function () {
        var e_1, _a, _b;
        var _c;
        if (!this._clicks.length) {
            return;
        }
        clearTimeout(this._checkClickTimer);
        this._checkClickTimer = undefined;
        var clicksToCheck = this._clicks;
        this._clicks = [];
        try {
            for (var clicksToCheck_1 = __values(clicksToCheck), clicksToCheck_1_1 = clicksToCheck_1.next(); !clicksToCheck_1_1.done; clicksToCheck_1_1 = clicksToCheck_1.next()) {
                var click = clicksToCheck_1_1.value;
                click.mutationDelayMs =
                    (_c = click.mutationDelayMs) !== null && _c !== void 0 ? _c : (this._lastMutation && click.timestamp <= this._lastMutation
                        ? this._lastMutation - click.timestamp
                        : undefined);
                click.absoluteDelayMs = Date.now() - click.timestamp;
                click.selectionChangedDelayMs =
                    this._lastSelectionChanged && click.timestamp <= this._lastSelectionChanged
                        ? this._lastSelectionChanged - click.timestamp
                        : undefined;
                click.visibilityChangedDelayMs = this._lastVisibilityChange
                    ? Math.abs(click.timestamp - this._lastVisibilityChange)
                    : undefined;
                var scrollTimeout = checkTimeout(click.scrollDelayMs, this._config.scroll_threshold_ms);
                var selectionChangedTimeout = checkTimeout(click.selectionChangedDelayMs, this._config.selection_change_threshold_ms);
                var mutationTimeout = checkTimeout(click.mutationDelayMs, this._config.mutation_threshold_ms);
                // we want to timeout eventually even if nothing else catches it...
                // we leave a little longer than the maximum threshold to give the other checks a chance to catch it
                var absoluteTimeout = checkTimeout(click.absoluteDelayMs, this._config.mutation_threshold_ms * 1.1);
                var hadScroll = (0, core_1.isNumber)(click.scrollDelayMs) && click.scrollDelayMs < this._config.scroll_threshold_ms;
                var hadMutation = (0, core_1.isNumber)(click.mutationDelayMs) && click.mutationDelayMs < this._config.mutation_threshold_ms;
                var hadSelectionChange = (0, core_1.isNumber)(click.selectionChangedDelayMs) &&
                    click.selectionChangedDelayMs < this._config.selection_change_threshold_ms;
                var hadVisibilityChange = (0, core_1.isNumber)(click.visibilityChangedDelayMs) &&
                    click.visibilityChangedDelayMs < this._config.selection_change_threshold_ms;
                if (hadScroll || hadMutation || hadSelectionChange || hadVisibilityChange) {
                    continue;
                }
                var visibilityChangedTimeout = checkTimeout(click.visibilityChangedDelayMs, this._config.selection_change_threshold_ms);
                if (scrollTimeout ||
                    mutationTimeout ||
                    absoluteTimeout ||
                    selectionChangedTimeout ||
                    visibilityChangedTimeout) {
                    var prefix = deadEventName(click);
                    if (prefix === '$dead_swipe') {
                        if (this._deadSwipesCaptured >= this._config.max_dead_swipes_per_page_load) {
                            // the page-load budget for dead swipes is spent — drop, don't requeue
                            continue;
                        }
                        this._deadSwipesCaptured++;
                    }
                    this._onCapture(click, (_b = {},
                        _b["".concat(prefix, "_last_mutation_timestamp")] = this._lastMutation,
                        _b["".concat(prefix, "_event_timestamp")] = click.timestamp,
                        _b["".concat(prefix, "_scroll_timeout")] = scrollTimeout,
                        _b["".concat(prefix, "_mutation_timeout")] = mutationTimeout,
                        _b["".concat(prefix, "_absolute_timeout")] = absoluteTimeout,
                        _b["".concat(prefix, "_selection_changed_timeout")] = selectionChangedTimeout,
                        _b["".concat(prefix, "_visibility_changed_timeout")] = visibilityChangedTimeout,
                        _b));
                }
                else if (click.absoluteDelayMs < this._config.mutation_threshold_ms) {
                    // keep waiting until next check
                    this._clicks.push(click);
                }
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (clicksToCheck_1_1 && !clicksToCheck_1_1.done && (_a = clicksToCheck_1.return)) _a.call(clicksToCheck_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
        this._scheduleCheck();
    };
    LazyLoadedDeadClicksAutocapture.prototype._captureDeadClick = function (click, properties) {
        var _a;
        // TODO need to check safe and captur-able as with autocapture
        // TODO autocaputure config
        var prefix = deadEventName(click);
        this.instance.capture(prefix, __assign(__assign(__assign({}, properties), (0, autocapture_1.autocapturePropertiesForElement)(click.node, {
            e: click.originalEvent,
            maskAllElementAttributes: this.instance.config.mask_all_element_attributes,
            maskAllText: this.instance.config.mask_all_text,
            elementAttributeIgnoreList: this._config.element_attribute_ignorelist,
            // TRICKY: it appears that we were moving to elementsChainAsString, but the UI still depends on elements, so :shrug:
            elementsChainAsString: false,
            disableCaptureUrlHashes: this.instance.config.disable_capture_url_hashes,
        }).props), (_a = {}, _a["".concat(prefix, "_scroll_delay_ms")] = click.scrollDelayMs, _a["".concat(prefix, "_mutation_delay_ms")] = click.mutationDelayMs, _a["".concat(prefix, "_absolute_delay_ms")] = click.absoluteDelayMs, _a["".concat(prefix, "_selection_changed_delay_ms")] = click.selectionChangedDelayMs, _a["".concat(prefix, "_visibility_changed_delay_ms")] = click.visibilityChangedDelayMs, 
        // undefined for clicks (stripped on serialization), like the delay fields above
        _a.$dead_swipe_direction = click.swipeDirection, _a.$dead_swipe_distance_px = click.swipeDistancePx, _a)), {
            timestamp: new Date(click.timestamp),
        });
    };
    return LazyLoadedDeadClicksAutocapture;
}());
globals_2.assignableWindow.__PosthogExtensions__ = globals_2.assignableWindow.__PosthogExtensions__ || {};
globals_2.assignableWindow.__PosthogExtensions__.initDeadClicksAutocapture = function (ph, config) {
    return new LazyLoadedDeadClicksAutocapture(ph, config);
};
exports.default = LazyLoadedDeadClicksAutocapture;
//# sourceMappingURL=dead-clicks-autocapture.js.map