/**
 * Web Vitals entrypoint (without attribution)
 *
 * This is the default, lighter bundle (~6KB) that captures core web vitals metrics
 * without attribution data. Attribution data includes debugging information like
 * which elements caused layout shifts, timing breakdowns, etc.
 *
 * We split this into two bundles because:
 * 1. Attribution code adds ~6KB to the bundle size
 * 2. Attribution can cause memory issues in SPAs (onCLS holds references to detached DOM elements)
 * 3. Most users only need aggregate metrics, not debugging attribution data
 *
 * For attribution data, use web-vitals-with-attribution.ts instead by setting:
 *   capture_performance: { web_vitals_attribution: true }
 *
 * @see web-vitals-with-attribution.ts
 */
import '@posthog/browser-common/utils/array-at-polyfill';
declare const postHogWebVitalsCallbacks: {
    onLCP: (onReport: (metric: import("web-vitals").LCPMetric) => void, opts?: import("web-vitals").ReportOpts) => void;
    onCLS: (onReport: (metric: import("web-vitals").CLSMetric) => void, opts?: import("web-vitals").ReportOpts) => void;
    onFCP: (onReport: (metric: import("web-vitals").FCPMetric) => void, opts?: import("web-vitals").ReportOpts) => void;
    onINP: (onReport: (metric: import("web-vitals").INPMetric) => void, opts?: import("web-vitals").INPReportOpts) => void;
};
export default postHogWebVitalsCallbacks;
