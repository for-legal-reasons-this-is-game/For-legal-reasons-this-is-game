import type { PostHogConfig, PostHogInterface } from '../types';
import '../config';
type PostHogWithFlags = Pick<PostHogInterface, 'setPersonPropertiesForFlags'> & {
    config: Pick<PostHogConfig, 'mask_personal_data_properties' | 'custom_personal_data_properties' | 'detect_google_search_app' | 'disable_capture_url_hashes' | 'custom_campaign_params'>;
};
export declare const setAllPersonProfilePropertiesAsPersonPropertiesForFlags: (posthog: PostHogWithFlags) => void;
export {};
