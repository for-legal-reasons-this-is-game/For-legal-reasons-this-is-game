import { isFunction } from "@posthog/core";
class ExtensionRuntime {
    constructor(_logger, _client){
        this._logger = _logger;
        this._client = _client;
        this._extensions = new Map();
        this._disposed = false;
    }
    async add(extension) {
        if (this._disposed) throw new Error('Cannot add an extension to a disposed ExtensionRuntime');
        if (this._extensions.has(extension.name)) throw new Error(`Browser extension "${extension.name}" is already registered`);
        this._extensions.set(extension.name, extension);
        try {
            const setup = extension.setup(this._client);
            if (setup) await setup;
        } catch (error) {
            const active = this._extensions.get(extension.name) === extension;
            if (active) this._extensions.delete(extension.name);
            this._logger.error(`Failed to set up browser extension "${extension.name}"`, error);
            if (active) this._disposeExtension(extension);
        }
    }
    dispose() {
        if (this._disposed) return;
        this._disposed = true;
        const extensions = Array.from(this._extensions.values()).reverse();
        this._extensions.clear();
        for (const extension of extensions)this._disposeExtension(extension);
    }
    _disposeExtension(extension) {
        try {
            const result = extension.dispose?.();
            if (result && isFunction(result.then)) result.then(void 0, (error)=>{
                this._logger.error(`Failed to dispose browser extension "${extension.name}"`, error);
            });
        } catch (error) {
            this._logger.error(`Failed to dispose browser extension "${extension.name}"`, error);
        }
    }
}
export { ExtensionRuntime };
