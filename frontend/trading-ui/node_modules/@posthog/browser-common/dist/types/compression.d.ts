export declare const Compression: {
    readonly GZipJS: "gzip-js";
    readonly Base64: "base64";
};
export type Compression = (typeof Compression)[keyof typeof Compression];
