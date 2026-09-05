import { createDisposable } from "./disposable.mjs";
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
            return createDisposable(()=>{
                subscription.isActive = false;
                const index = this._subscriptions.indexOf(subscription);
                if (-1 !== index) this._subscriptions.splice(index, 1);
            });
        };
    }
}
export { Publisher };
