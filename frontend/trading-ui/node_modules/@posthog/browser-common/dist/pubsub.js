"use strict";
var __webpack_require__ = {};
(()=>{
    __webpack_require__.d = (exports1, definition)=>{
        for(var key in definition)if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports1, key)) Object.defineProperty(exports1, key, {
            enumerable: true,
            get: definition[key]
        });
    };
})();
(()=>{
    __webpack_require__.o = (obj, prop)=>Object.prototype.hasOwnProperty.call(obj, prop);
})();
(()=>{
    __webpack_require__.r = (exports1)=>{
        if ('undefined' != typeof Symbol && Symbol.toStringTag) Object.defineProperty(exports1, Symbol.toStringTag, {
            value: 'Module'
        });
        Object.defineProperty(exports1, '__esModule', {
            value: true
        });
    };
})();
var __webpack_exports__ = {};
__webpack_require__.r(__webpack_exports__);
__webpack_require__.d(__webpack_exports__, {
    Publisher: ()=>Publisher
});
const external_disposable_js_namespaceObject = require("./disposable.js");
class Publisher {
    publish(payload) {
        const subscriptions = this._subscriptions.slice();
        subscriptions.forEach((subscription)=>{
            if (subscription.isActive) subscription.handler(payload);
        });
    }
    dispose() {
        this._subscriptions.forEach((subscription)=>{
            subscription.isActive = false;
        });
        this._subscriptions = [];
    }
    constructor(){
        this._subscriptions = [];
        this.listener = (handler)=>{
            const subscription = {
                handler,
                isActive: true
            };
            this._subscriptions.push(subscription);
            return (0, external_disposable_js_namespaceObject.createDisposable)(()=>{
                subscription.isActive = false;
                const index = this._subscriptions.indexOf(subscription);
                if (-1 !== index) this._subscriptions.splice(index, 1);
            });
        };
    }
}
exports.Publisher = __webpack_exports__.Publisher;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "Publisher"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
