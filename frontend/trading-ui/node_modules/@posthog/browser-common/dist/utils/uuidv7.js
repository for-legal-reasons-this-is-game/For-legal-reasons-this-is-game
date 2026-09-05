/*! For license information please see uuidv7.js.LICENSE.txt */
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
    uuid7ToTimestampMs: ()=>uuid7ToTimestampMs,
    UUID: ()=>UUID,
    uuidv7: ()=>uuidv7
});
const external_globals_js_namespaceObject = require("./globals.js");
const core_namespaceObject = require("@posthog/core");
/**
 * uuidv7: An experimental implementation of the proposed UUID Version 7
 *
 * @license Apache-2.0
 * @copyright 2021-2023 LiosK
 * @packageDocumentation
 *
 * from https://github.com/LiosK/uuidv7/blob/e501462ea3d23241de13192ceae726956f9b3b7d/src/index.ts
 */ if (!Math.trunc) Math.trunc = function(v) {
    return v < 0 ? Math.ceil(v) : Math.floor(v);
};
if (!Number.isInteger) Number.isInteger = function(value) {
    return (0, core_namespaceObject.isNumber)(value) && isFinite(value) && Math.floor(value) === value;
};
class UUID {
    constructor(bytes){
        this.bytes = bytes;
        if (16 !== bytes.length) throw new TypeError('not 128-bit length');
    }
    static fromFieldsV7(unixTsMs, randA, randBHi, randBLo) {
        if (!Number.isInteger(unixTsMs) || !Number.isInteger(randA) || !Number.isInteger(randBHi) || !Number.isInteger(randBLo) || unixTsMs < 0 || randA < 0 || randBHi < 0 || randBLo < 0 || unixTsMs > 0xffffffffffff || randA > 0xfff || randBHi > 0x3fffffff || randBLo > 0xffffffff) throw new RangeError('invalid field value');
        const bytes = new Uint8Array(16);
        bytes[0] = unixTsMs / 2 ** 40;
        bytes[1] = unixTsMs / 2 ** 32;
        bytes[2] = unixTsMs / 2 ** 24;
        bytes[3] = unixTsMs / 2 ** 16;
        bytes[4] = unixTsMs / 256;
        bytes[5] = unixTsMs;
        bytes[6] = 0x70 | randA >>> 8;
        bytes[7] = randA;
        bytes[8] = 0x80 | randBHi >>> 24;
        bytes[9] = randBHi >>> 16;
        bytes[10] = randBHi >>> 8;
        bytes[11] = randBHi;
        bytes[12] = randBLo >>> 24;
        bytes[13] = randBLo >>> 16;
        bytes[14] = randBLo >>> 8;
        bytes[15] = randBLo;
        return new UUID(bytes);
    }
    toString() {
        let text = '';
        for(let i = 0; i < this.bytes.length; i++){
            text = text + (this.bytes[i] >>> 4).toString(16) + (0xf & this.bytes[i]).toString(16);
            if (3 === i || 5 === i || 7 === i || 9 === i) text += '-';
        }
        if (36 !== text.length) throw new Error('Invalid UUIDv7 was generated');
        return text;
    }
    clone() {
        return new UUID(this.bytes.slice(0));
    }
    equals(other) {
        return 0 === this.compareTo(other);
    }
    compareTo(other) {
        for(let i = 0; i < 16; i++){
            const diff = this.bytes[i] - other.bytes[i];
            if (0 !== diff) return Math.sign(diff);
        }
        return 0;
    }
}
class V7Generator {
    generate() {
        const value = this.generateOrAbort();
        if (!(0, core_namespaceObject.isUndefined)(value)) return value;
        {
            this._timestamp = 0;
            const valueAfterReset = this.generateOrAbort();
            if ((0, core_namespaceObject.isUndefined)(valueAfterReset)) throw new Error('Could not generate UUID after timestamp reset');
            return valueAfterReset;
        }
    }
    generateOrAbort() {
        const MAX_COUNTER = 0x3ffffffffff;
        const ROLLBACK_ALLOWANCE = 10000;
        const ts = Date.now();
        if (ts > this._timestamp) {
            this._timestamp = ts;
            this._resetCounter();
        } else {
            if (!(ts + ROLLBACK_ALLOWANCE > this._timestamp)) return;
            this._counter++;
            if (this._counter > MAX_COUNTER) {
                this._timestamp++;
                this._resetCounter();
            }
        }
        return UUID.fromFieldsV7(this._timestamp, Math.trunc(this._counter / 2 ** 30), this._counter & 2 ** 30 - 1, this._random.nextUint32());
    }
    _resetCounter() {
        this._counter = 0x400 * this._random.nextUint32() + (0x3ff & this._random.nextUint32());
    }
    constructor(){
        this._timestamp = 0;
        this._counter = 0;
        this._random = new DefaultRandom();
    }
}
let getRandomValues = (buffer)=>{
    if ('undefined' != typeof UUIDV7_DENY_WEAK_RNG && UUIDV7_DENY_WEAK_RNG) throw new Error('no cryptographically strong RNG available');
    for(let i = 0; i < buffer.length; i++)buffer[i] = 0x10000 * Math.trunc(0x10000 * Math.random()) + Math.trunc(0x10000 * Math.random());
    return buffer;
};
if (external_globals_js_namespaceObject.window && !(0, core_namespaceObject.isUndefined)(external_globals_js_namespaceObject.window.crypto) && crypto.getRandomValues) getRandomValues = (buffer)=>crypto.getRandomValues(buffer);
class DefaultRandom {
    nextUint32() {
        if (this._cursor >= this._buffer.length) {
            getRandomValues(this._buffer);
            this._cursor = 0;
        }
        return this._buffer[this._cursor++];
    }
    constructor(){
        this._buffer = new Uint32Array(8);
        this._cursor = 1 / 0;
    }
}
let defaultGenerator;
const uuidv7 = ()=>uuidv7obj().toString();
const uuidv7obj = ()=>(defaultGenerator || (defaultGenerator = new V7Generator())).generate();
const uuid7ToTimestampMs = (uuid)=>{
    const hex = uuid.replace(/-/g, '');
    if (32 !== hex.length) throw new Error('Not a valid UUID');
    if ('7' !== hex[12]) throw new Error('Not a UUIDv7');
    return parseInt(hex.substring(0, 12), 16);
};
exports.UUID = __webpack_exports__.UUID;
exports.uuid7ToTimestampMs = __webpack_exports__.uuid7ToTimestampMs;
exports.uuidv7 = __webpack_exports__.uuidv7;
for(var __webpack_i__ in __webpack_exports__)if (-1 === [
    "UUID",
    "uuid7ToTimestampMs",
    "uuidv7"
].indexOf(__webpack_i__)) exports[__webpack_i__] = __webpack_exports__[__webpack_i__];
Object.defineProperty(exports, '__esModule', {
    value: true
});
