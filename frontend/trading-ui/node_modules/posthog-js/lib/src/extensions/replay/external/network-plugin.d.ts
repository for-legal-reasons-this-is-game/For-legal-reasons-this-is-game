import type { RecordPlugin } from '../types/rrweb-types';
import { CapturedNetworkRequest, Headers, NetworkRecordOptions } from '../../../types';
export type NetworkData = {
    requests: CapturedNetworkRequest[];
    isInitial?: boolean;
};
export declare function findLast<T>(array: Array<T>, predicate: (value: T) => boolean): T | undefined;
export declare const NEVER_RECORD_BODY_CONTENT_TYPES: string[];
export declare function shouldRecordBody({ type, recordBody, headers, url, }: {
    type: 'request' | 'response';
    headers: Headers;
    url: string | URL | RequestInfo;
    recordBody: NetworkRecordOptions['recordBody'];
}): boolean;
export declare function _contentLengthExceedsLimit(r: Request | Response, limitBytes: number): boolean;
export declare function _readBody(r: Request | Response, options: NetworkRecordOptions): Promise<string>;
export declare function _tryReadBodyStreaming(r: Request | Response, limitBytes: number): Promise<string>;
export declare const NETWORK_PLUGIN_NAME = "rrweb/network@1";
export declare const getRecordNetworkPlugin: (options?: NetworkRecordOptions) => RecordPlugin;
