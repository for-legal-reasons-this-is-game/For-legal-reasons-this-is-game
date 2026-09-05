import { SessionIdManager } from '../sessionid';
import type { TracingHeadersDistinctId, TracingHeadersHostnames } from '../extensions/tracing-headers-types';
declare const patchFns: {
    _patchFetch: (hostnames: TracingHeadersHostnames, distinctId: TracingHeadersDistinctId, sessionManager?: SessionIdManager) => (() => void);
    _patchXHR: (hostnames: TracingHeadersHostnames, distinctId: TracingHeadersDistinctId, sessionManager?: SessionIdManager) => (() => void);
};
export default patchFns;
