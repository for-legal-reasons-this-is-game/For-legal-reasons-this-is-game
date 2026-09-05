type SDKDistChannel = 'npm' | 'cdn';
declare const Config: {
    DEBUG: boolean;
    LIB_VERSION: string;
    LIB_NAME: string;
    SDK_DIST_CHANNEL?: SDKDistChannel;
    JS_SDK_VERSION: string;
};
export default Config;
