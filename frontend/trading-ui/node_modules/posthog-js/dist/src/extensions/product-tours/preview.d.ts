import { JSX } from 'preact';
import { PostHog } from '../../posthog-core';
import { ProductTourStep, ProductTourAppearance } from '../../posthog-product-tours-types';
export interface RenderProductTourPreviewOptions {
    step: ProductTourStep;
    appearance?: ProductTourAppearance;
    parentElement: HTMLElement;
    stepIndex?: number;
    totalSteps?: number;
    style?: JSX.CSSProperties;
    posthog?: PostHog;
}
export declare function renderProductTourPreview({ step, appearance, parentElement, stepIndex, totalSteps, style, posthog, }: RenderProductTourPreviewOptions): void;
