import { SurveyAppearance } from '../../../posthog-surveys-types';
export declare function BottomSection({ text, submitDisabled, appearance, onSubmit, link, onPreviewSubmit, skipSubmitButton, canGoBack, onBack, }: {
    text: string;
    submitDisabled: boolean;
    appearance: SurveyAppearance;
    onSubmit: () => void;
    link?: string | null;
    onPreviewSubmit?: () => void;
    skipSubmitButton?: boolean;
    canGoBack?: boolean;
    onBack?: () => void;
}): import("preact").JSX.Element;
