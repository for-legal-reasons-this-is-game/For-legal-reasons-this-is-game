import { isFunction } from "@posthog/core";
function createDisposable(dispose) {
    let active = true;
    return {
        dispose: ()=>{
            if (active) {
                active = false;
                const result = dispose();
                if (result && isFunction(result.then)) result.then(void 0, ()=>{});
            }
        }
    };
}
export { createDisposable };
