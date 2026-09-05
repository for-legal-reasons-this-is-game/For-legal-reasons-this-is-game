if ('function' != typeof Array.prototype.at) Object.defineProperty(Array.prototype, 'at', {
    value: function(index) {
        const len = this.length;
        let relativeIndex = Math.trunc(index) || 0;
        if (relativeIndex < 0) relativeIndex += len;
        return relativeIndex < 0 || relativeIndex >= len ? void 0 : this[relativeIndex];
    },
    writable: true,
    enumerable: false,
    configurable: true
});
