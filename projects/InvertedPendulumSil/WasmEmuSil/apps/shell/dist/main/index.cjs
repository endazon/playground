"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/utf8.cjs
var require_utf8 = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/utf8.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.utf8Count = utf8Count;
    exports2.utf8EncodeJs = utf8EncodeJs;
    exports2.utf8EncodeTE = utf8EncodeTE;
    exports2.utf8Encode = utf8Encode;
    exports2.utf8DecodeJs = utf8DecodeJs;
    exports2.utf8DecodeTD = utf8DecodeTD;
    exports2.utf8Decode = utf8Decode;
    function utf8Count(str2) {
      const strLength = str2.length;
      let byteLength = 0;
      let pos = 0;
      while (pos < strLength) {
        let value = str2.charCodeAt(pos++);
        if ((value & 4294967168) === 0) {
          byteLength++;
          continue;
        } else if ((value & 4294965248) === 0) {
          byteLength += 2;
        } else {
          if (value >= 55296 && value <= 56319) {
            if (pos < strLength) {
              const extra = str2.charCodeAt(pos);
              if ((extra & 64512) === 56320) {
                ++pos;
                value = ((value & 1023) << 10) + (extra & 1023) + 65536;
              }
            }
          }
          if ((value & 4294901760) === 0) {
            byteLength += 3;
          } else {
            byteLength += 4;
          }
        }
      }
      return byteLength;
    }
    function utf8EncodeJs(str2, output, outputOffset) {
      const strLength = str2.length;
      let offset = outputOffset;
      let pos = 0;
      while (pos < strLength) {
        let value = str2.charCodeAt(pos++);
        if ((value & 4294967168) === 0) {
          output[offset++] = value;
          continue;
        } else if ((value & 4294965248) === 0) {
          output[offset++] = value >> 6 & 31 | 192;
        } else {
          if (value >= 55296 && value <= 56319) {
            if (pos < strLength) {
              const extra = str2.charCodeAt(pos);
              if ((extra & 64512) === 56320) {
                ++pos;
                value = ((value & 1023) << 10) + (extra & 1023) + 65536;
              }
            }
          }
          if ((value & 4294901760) === 0) {
            output[offset++] = value >> 12 & 15 | 224;
            output[offset++] = value >> 6 & 63 | 128;
          } else {
            output[offset++] = value >> 18 & 7 | 240;
            output[offset++] = value >> 12 & 63 | 128;
            output[offset++] = value >> 6 & 63 | 128;
          }
        }
        output[offset++] = value & 63 | 128;
      }
    }
    var sharedTextEncoder = new TextEncoder();
    var TEXT_ENCODER_THRESHOLD = 50;
    function utf8EncodeTE(str2, output, outputOffset) {
      sharedTextEncoder.encodeInto(str2, output.subarray(outputOffset));
    }
    function utf8Encode(str2, output, outputOffset) {
      if (str2.length > TEXT_ENCODER_THRESHOLD) {
        utf8EncodeTE(str2, output, outputOffset);
      } else {
        utf8EncodeJs(str2, output, outputOffset);
      }
    }
    var CHUNK_SIZE = 4096;
    function utf8DecodeJs(bytes, inputOffset, byteLength) {
      let offset = inputOffset;
      const end = offset + byteLength;
      const units = [];
      let result = "";
      while (offset < end) {
        const byte1 = bytes[offset++];
        if ((byte1 & 128) === 0) {
          units.push(byte1);
        } else if ((byte1 & 224) === 192) {
          const byte2 = bytes[offset++] & 63;
          units.push((byte1 & 31) << 6 | byte2);
        } else if ((byte1 & 240) === 224) {
          const byte2 = bytes[offset++] & 63;
          const byte3 = bytes[offset++] & 63;
          units.push((byte1 & 31) << 12 | byte2 << 6 | byte3);
        } else if ((byte1 & 248) === 240) {
          const byte2 = bytes[offset++] & 63;
          const byte3 = bytes[offset++] & 63;
          const byte4 = bytes[offset++] & 63;
          let unit = (byte1 & 7) << 18 | byte2 << 12 | byte3 << 6 | byte4;
          if (unit > 65535) {
            unit -= 65536;
            units.push(unit >>> 10 & 1023 | 55296);
            unit = 56320 | unit & 1023;
          }
          units.push(unit);
        } else {
          units.push(byte1);
        }
        if (units.length >= CHUNK_SIZE) {
          result += String.fromCharCode(...units);
          units.length = 0;
        }
      }
      if (units.length > 0) {
        result += String.fromCharCode(...units);
      }
      return result;
    }
    var sharedTextDecoder = new TextDecoder();
    var TEXT_DECODER_THRESHOLD = 200;
    function utf8DecodeTD(bytes, inputOffset, byteLength) {
      const stringBytes = bytes.subarray(inputOffset, inputOffset + byteLength);
      return sharedTextDecoder.decode(stringBytes);
    }
    function utf8Decode(bytes, inputOffset, byteLength) {
      if (byteLength > TEXT_DECODER_THRESHOLD) {
        return utf8DecodeTD(bytes, inputOffset, byteLength);
      } else {
        return utf8DecodeJs(bytes, inputOffset, byteLength);
      }
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/ExtData.cjs
var require_ExtData = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/ExtData.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.ExtData = void 0;
    var ExtData = class {
      type;
      data;
      constructor(type, data) {
        this.type = type;
        this.data = data;
      }
    };
    exports2.ExtData = ExtData;
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/DecodeError.cjs
var require_DecodeError = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/DecodeError.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.DecodeError = void 0;
    var DecodeError = class _DecodeError extends Error {
      constructor(message) {
        super(message);
        const proto = Object.create(_DecodeError.prototype);
        Object.setPrototypeOf(this, proto);
        Object.defineProperty(this, "name", {
          configurable: true,
          enumerable: false,
          value: _DecodeError.name
        });
      }
    };
    exports2.DecodeError = DecodeError;
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/int.cjs
var require_int = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/int.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.UINT32_MAX = void 0;
    exports2.setUint64 = setUint64;
    exports2.setInt64 = setInt64;
    exports2.getInt64 = getInt64;
    exports2.getUint64 = getUint64;
    exports2.UINT32_MAX = 4294967295;
    function setUint64(view, offset, value) {
      const high = value / 4294967296;
      const low = value;
      view.setUint32(offset, high);
      view.setUint32(offset + 4, low);
    }
    function setInt64(view, offset, value) {
      const high = Math.floor(value / 4294967296);
      const low = value;
      view.setUint32(offset, high);
      view.setUint32(offset + 4, low);
    }
    function getInt64(view, offset) {
      const high = view.getInt32(offset);
      const low = view.getUint32(offset + 4);
      return high * 4294967296 + low;
    }
    function getUint64(view, offset) {
      const high = view.getUint32(offset);
      const low = view.getUint32(offset + 4);
      return high * 4294967296 + low;
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/timestamp.cjs
var require_timestamp = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/timestamp.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.timestampExtension = exports2.EXT_TIMESTAMP = void 0;
    exports2.encodeTimeSpecToTimestamp = encodeTimeSpecToTimestamp;
    exports2.encodeDateToTimeSpec = encodeDateToTimeSpec;
    exports2.encodeTimestampExtension = encodeTimestampExtension;
    exports2.decodeTimestampToTimeSpec = decodeTimestampToTimeSpec;
    exports2.decodeTimestampExtension = decodeTimestampExtension;
    var DecodeError_ts_1 = require_DecodeError();
    var int_ts_1 = require_int();
    exports2.EXT_TIMESTAMP = -1;
    var TIMESTAMP32_MAX_SEC = 4294967296 - 1;
    var TIMESTAMP64_MAX_SEC = 17179869184 - 1;
    function encodeTimeSpecToTimestamp({ sec, nsec }) {
      if (sec >= 0 && nsec >= 0 && sec <= TIMESTAMP64_MAX_SEC) {
        if (nsec === 0 && sec <= TIMESTAMP32_MAX_SEC) {
          const rv = new Uint8Array(4);
          const view = new DataView(rv.buffer);
          view.setUint32(0, sec);
          return rv;
        } else {
          const secHigh = sec / 4294967296;
          const secLow = sec & 4294967295;
          const rv = new Uint8Array(8);
          const view = new DataView(rv.buffer);
          view.setUint32(0, nsec << 2 | secHigh & 3);
          view.setUint32(4, secLow);
          return rv;
        }
      } else {
        const rv = new Uint8Array(12);
        const view = new DataView(rv.buffer);
        view.setUint32(0, nsec);
        (0, int_ts_1.setInt64)(view, 4, sec);
        return rv;
      }
    }
    function encodeDateToTimeSpec(date) {
      const msec = date.getTime();
      const sec = Math.floor(msec / 1e3);
      const nsec = (msec - sec * 1e3) * 1e6;
      const nsecInSec = Math.floor(nsec / 1e9);
      return {
        sec: sec + nsecInSec,
        nsec: nsec - nsecInSec * 1e9
      };
    }
    function encodeTimestampExtension(object) {
      if (object instanceof Date) {
        const timeSpec = encodeDateToTimeSpec(object);
        return encodeTimeSpecToTimestamp(timeSpec);
      } else {
        return null;
      }
    }
    function decodeTimestampToTimeSpec(data) {
      const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
      switch (data.byteLength) {
        case 4: {
          const sec = view.getUint32(0);
          const nsec = 0;
          return { sec, nsec };
        }
        case 8: {
          const nsec30AndSecHigh2 = view.getUint32(0);
          const secLow32 = view.getUint32(4);
          const sec = (nsec30AndSecHigh2 & 3) * 4294967296 + secLow32;
          const nsec = nsec30AndSecHigh2 >>> 2;
          return { sec, nsec };
        }
        case 12: {
          const sec = (0, int_ts_1.getInt64)(view, 4);
          const nsec = view.getUint32(0);
          return { sec, nsec };
        }
        default:
          throw new DecodeError_ts_1.DecodeError(`Unrecognized data size for timestamp (expected 4, 8, or 12): ${data.length}`);
      }
    }
    function decodeTimestampExtension(data) {
      const timeSpec = decodeTimestampToTimeSpec(data);
      return new Date(timeSpec.sec * 1e3 + timeSpec.nsec / 1e6);
    }
    exports2.timestampExtension = {
      type: exports2.EXT_TIMESTAMP,
      encode: encodeTimestampExtension,
      decode: decodeTimestampExtension
    };
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/ExtensionCodec.cjs
var require_ExtensionCodec = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/ExtensionCodec.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.ExtensionCodec = void 0;
    var ExtData_ts_1 = require_ExtData();
    var timestamp_ts_1 = require_timestamp();
    var ExtensionCodec = class _ExtensionCodec {
      static defaultCodec = new _ExtensionCodec();
      // ensures ExtensionCodecType<X> matches ExtensionCodec<X>
      // this will make type errors a lot more clear
      // eslint-disable-next-line @typescript-eslint/naming-convention
      __brand;
      // built-in extensions
      builtInEncoders = [];
      builtInDecoders = [];
      // custom extensions
      encoders = [];
      decoders = [];
      constructor() {
        this.register(timestamp_ts_1.timestampExtension);
      }
      register({ type, encode, decode: decode2 }) {
        if (type >= 0) {
          this.encoders[type] = encode;
          this.decoders[type] = decode2;
        } else {
          const index = -1 - type;
          this.builtInEncoders[index] = encode;
          this.builtInDecoders[index] = decode2;
        }
      }
      tryToEncode(object, context) {
        for (let i = 0; i < this.builtInEncoders.length; i++) {
          const encodeExt = this.builtInEncoders[i];
          if (encodeExt != null) {
            const data = encodeExt(object, context);
            if (data != null) {
              const type = -1 - i;
              return new ExtData_ts_1.ExtData(type, data);
            }
          }
        }
        for (let i = 0; i < this.encoders.length; i++) {
          const encodeExt = this.encoders[i];
          if (encodeExt != null) {
            const data = encodeExt(object, context);
            if (data != null) {
              const type = i;
              return new ExtData_ts_1.ExtData(type, data);
            }
          }
        }
        if (object instanceof ExtData_ts_1.ExtData) {
          return object;
        }
        return null;
      }
      decode(data, type, context) {
        const decodeExt = type < 0 ? this.builtInDecoders[-1 - type] : this.decoders[type];
        if (decodeExt) {
          return decodeExt(data, type, context);
        } else {
          return new ExtData_ts_1.ExtData(type, data);
        }
      }
    };
    exports2.ExtensionCodec = ExtensionCodec;
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/typedArrays.cjs
var require_typedArrays = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/typedArrays.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.ensureUint8Array = ensureUint8Array;
    function isArrayBufferLike(buffer) {
      return buffer instanceof ArrayBuffer || typeof SharedArrayBuffer !== "undefined" && buffer instanceof SharedArrayBuffer;
    }
    function ensureUint8Array(buffer) {
      if (buffer instanceof Uint8Array) {
        return buffer;
      } else if (ArrayBuffer.isView(buffer)) {
        return new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength);
      } else if (isArrayBufferLike(buffer)) {
        return new Uint8Array(buffer);
      } else {
        return Uint8Array.from(buffer);
      }
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/Encoder.cjs
var require_Encoder = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/Encoder.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.Encoder = exports2.DEFAULT_INITIAL_BUFFER_SIZE = exports2.DEFAULT_MAX_DEPTH = void 0;
    var utf8_ts_1 = require_utf8();
    var ExtensionCodec_ts_1 = require_ExtensionCodec();
    var int_ts_1 = require_int();
    var typedArrays_ts_1 = require_typedArrays();
    exports2.DEFAULT_MAX_DEPTH = 100;
    exports2.DEFAULT_INITIAL_BUFFER_SIZE = 2048;
    var Encoder = class _Encoder {
      extensionCodec;
      context;
      useBigInt64;
      maxDepth;
      initialBufferSize;
      sortKeys;
      forceFloat32;
      ignoreUndefined;
      forceIntegerToFloat;
      pos;
      view;
      bytes;
      entered = false;
      constructor(options) {
        this.extensionCodec = options?.extensionCodec ?? ExtensionCodec_ts_1.ExtensionCodec.defaultCodec;
        this.context = options?.context;
        this.useBigInt64 = options?.useBigInt64 ?? false;
        this.maxDepth = options?.maxDepth ?? exports2.DEFAULT_MAX_DEPTH;
        this.initialBufferSize = options?.initialBufferSize ?? exports2.DEFAULT_INITIAL_BUFFER_SIZE;
        this.sortKeys = options?.sortKeys ?? false;
        this.forceFloat32 = options?.forceFloat32 ?? false;
        this.ignoreUndefined = options?.ignoreUndefined ?? false;
        this.forceIntegerToFloat = options?.forceIntegerToFloat ?? false;
        this.pos = 0;
        this.view = new DataView(new ArrayBuffer(this.initialBufferSize));
        this.bytes = new Uint8Array(this.view.buffer);
      }
      clone() {
        return new _Encoder({
          extensionCodec: this.extensionCodec,
          context: this.context,
          useBigInt64: this.useBigInt64,
          maxDepth: this.maxDepth,
          initialBufferSize: this.initialBufferSize,
          sortKeys: this.sortKeys,
          forceFloat32: this.forceFloat32,
          ignoreUndefined: this.ignoreUndefined,
          forceIntegerToFloat: this.forceIntegerToFloat
        });
      }
      reinitializeState() {
        this.pos = 0;
      }
      /**
       * This is almost equivalent to {@link Encoder#encode}, but it returns an reference of the encoder's internal buffer and thus much faster than {@link Encoder#encode}.
       *
       * @returns Encodes the object and returns a shared reference the encoder's internal buffer.
       */
      encodeSharedRef(object) {
        if (this.entered) {
          const instance = this.clone();
          return instance.encodeSharedRef(object);
        }
        try {
          this.entered = true;
          this.reinitializeState();
          this.doEncode(object, 1);
          return this.bytes.subarray(0, this.pos);
        } finally {
          this.entered = false;
        }
      }
      /**
       * @returns Encodes the object and returns a copy of the encoder's internal buffer.
       */
      encode(object) {
        if (this.entered) {
          const instance = this.clone();
          return instance.encode(object);
        }
        try {
          this.entered = true;
          this.reinitializeState();
          this.doEncode(object, 1);
          return this.bytes.slice(0, this.pos);
        } finally {
          this.entered = false;
        }
      }
      doEncode(object, depth) {
        if (depth > this.maxDepth) {
          throw new Error(`Too deep objects in depth ${depth}`);
        }
        if (object == null) {
          this.encodeNil();
        } else if (typeof object === "boolean") {
          this.encodeBoolean(object);
        } else if (typeof object === "number") {
          if (!this.forceIntegerToFloat) {
            this.encodeNumber(object);
          } else {
            this.encodeNumberAsFloat(object);
          }
        } else if (typeof object === "string") {
          this.encodeString(object);
        } else if (this.useBigInt64 && typeof object === "bigint") {
          this.encodeBigInt64(object);
        } else {
          this.encodeObject(object, depth);
        }
      }
      ensureBufferSizeToWrite(sizeToWrite) {
        const requiredSize = this.pos + sizeToWrite;
        if (this.view.byteLength < requiredSize) {
          this.resizeBuffer(requiredSize * 2);
        }
      }
      resizeBuffer(newSize) {
        const newBuffer = new ArrayBuffer(newSize);
        const newBytes = new Uint8Array(newBuffer);
        const newView = new DataView(newBuffer);
        newBytes.set(this.bytes);
        this.view = newView;
        this.bytes = newBytes;
      }
      encodeNil() {
        this.writeU8(192);
      }
      encodeBoolean(object) {
        if (object === false) {
          this.writeU8(194);
        } else {
          this.writeU8(195);
        }
      }
      encodeNumber(object) {
        if (!this.forceIntegerToFloat && Number.isSafeInteger(object)) {
          if (object >= 0) {
            if (object < 128) {
              this.writeU8(object);
            } else if (object < 256) {
              this.writeU8(204);
              this.writeU8(object);
            } else if (object < 65536) {
              this.writeU8(205);
              this.writeU16(object);
            } else if (object < 4294967296) {
              this.writeU8(206);
              this.writeU32(object);
            } else if (!this.useBigInt64) {
              this.writeU8(207);
              this.writeU64(object);
            } else {
              this.encodeNumberAsFloat(object);
            }
          } else {
            if (object >= -32) {
              this.writeU8(224 | object + 32);
            } else if (object >= -128) {
              this.writeU8(208);
              this.writeI8(object);
            } else if (object >= -32768) {
              this.writeU8(209);
              this.writeI16(object);
            } else if (object >= -2147483648) {
              this.writeU8(210);
              this.writeI32(object);
            } else if (!this.useBigInt64) {
              this.writeU8(211);
              this.writeI64(object);
            } else {
              this.encodeNumberAsFloat(object);
            }
          }
        } else {
          this.encodeNumberAsFloat(object);
        }
      }
      encodeNumberAsFloat(object) {
        if (this.forceFloat32) {
          this.writeU8(202);
          this.writeF32(object);
        } else {
          this.writeU8(203);
          this.writeF64(object);
        }
      }
      encodeBigInt64(object) {
        if (object >= BigInt(0)) {
          this.writeU8(207);
          this.writeBigUint64(object);
        } else {
          this.writeU8(211);
          this.writeBigInt64(object);
        }
      }
      writeStringHeader(byteLength) {
        if (byteLength < 32) {
          this.writeU8(160 + byteLength);
        } else if (byteLength < 256) {
          this.writeU8(217);
          this.writeU8(byteLength);
        } else if (byteLength < 65536) {
          this.writeU8(218);
          this.writeU16(byteLength);
        } else if (byteLength < 4294967296) {
          this.writeU8(219);
          this.writeU32(byteLength);
        } else {
          throw new Error(`Too long string: ${byteLength} bytes in UTF-8`);
        }
      }
      encodeString(object) {
        const maxHeaderSize = 1 + 4;
        const byteLength = (0, utf8_ts_1.utf8Count)(object);
        this.ensureBufferSizeToWrite(maxHeaderSize + byteLength);
        this.writeStringHeader(byteLength);
        (0, utf8_ts_1.utf8Encode)(object, this.bytes, this.pos);
        this.pos += byteLength;
      }
      encodeObject(object, depth) {
        const ext = this.extensionCodec.tryToEncode(object, this.context);
        if (ext != null) {
          this.encodeExtension(ext);
        } else if (Array.isArray(object)) {
          this.encodeArray(object, depth);
        } else if (ArrayBuffer.isView(object)) {
          this.encodeBinary(object);
        } else if (typeof object === "object") {
          this.encodeMap(object, depth);
        } else {
          throw new Error(`Unrecognized object: ${Object.prototype.toString.apply(object)}`);
        }
      }
      encodeBinary(object) {
        const size = object.byteLength;
        if (size < 256) {
          this.writeU8(196);
          this.writeU8(size);
        } else if (size < 65536) {
          this.writeU8(197);
          this.writeU16(size);
        } else if (size < 4294967296) {
          this.writeU8(198);
          this.writeU32(size);
        } else {
          throw new Error(`Too large binary: ${size}`);
        }
        const bytes = (0, typedArrays_ts_1.ensureUint8Array)(object);
        this.writeU8a(bytes);
      }
      encodeArray(object, depth) {
        const size = object.length;
        if (size < 16) {
          this.writeU8(144 + size);
        } else if (size < 65536) {
          this.writeU8(220);
          this.writeU16(size);
        } else if (size < 4294967296) {
          this.writeU8(221);
          this.writeU32(size);
        } else {
          throw new Error(`Too large array: ${size}`);
        }
        for (const item of object) {
          this.doEncode(item, depth + 1);
        }
      }
      countWithoutUndefined(object, keys) {
        let count = 0;
        for (const key of keys) {
          if (object[key] !== void 0) {
            count++;
          }
        }
        return count;
      }
      encodeMap(object, depth) {
        const keys = Object.keys(object);
        if (this.sortKeys) {
          keys.sort();
        }
        const size = this.ignoreUndefined ? this.countWithoutUndefined(object, keys) : keys.length;
        if (size < 16) {
          this.writeU8(128 + size);
        } else if (size < 65536) {
          this.writeU8(222);
          this.writeU16(size);
        } else if (size < 4294967296) {
          this.writeU8(223);
          this.writeU32(size);
        } else {
          throw new Error(`Too large map object: ${size}`);
        }
        for (const key of keys) {
          const value = object[key];
          if (!(this.ignoreUndefined && value === void 0)) {
            this.encodeString(key);
            this.doEncode(value, depth + 1);
          }
        }
      }
      encodeExtension(ext) {
        if (typeof ext.data === "function") {
          const data = ext.data(this.pos + 6);
          const size2 = data.length;
          if (size2 >= 4294967296) {
            throw new Error(`Too large extension object: ${size2}`);
          }
          this.writeU8(201);
          this.writeU32(size2);
          this.writeI8(ext.type);
          this.writeU8a(data);
          return;
        }
        const size = ext.data.length;
        if (size === 1) {
          this.writeU8(212);
        } else if (size === 2) {
          this.writeU8(213);
        } else if (size === 4) {
          this.writeU8(214);
        } else if (size === 8) {
          this.writeU8(215);
        } else if (size === 16) {
          this.writeU8(216);
        } else if (size < 256) {
          this.writeU8(199);
          this.writeU8(size);
        } else if (size < 65536) {
          this.writeU8(200);
          this.writeU16(size);
        } else if (size < 4294967296) {
          this.writeU8(201);
          this.writeU32(size);
        } else {
          throw new Error(`Too large extension object: ${size}`);
        }
        this.writeI8(ext.type);
        this.writeU8a(ext.data);
      }
      writeU8(value) {
        this.ensureBufferSizeToWrite(1);
        this.view.setUint8(this.pos, value);
        this.pos++;
      }
      writeU8a(values) {
        const size = values.length;
        this.ensureBufferSizeToWrite(size);
        this.bytes.set(values, this.pos);
        this.pos += size;
      }
      writeI8(value) {
        this.ensureBufferSizeToWrite(1);
        this.view.setInt8(this.pos, value);
        this.pos++;
      }
      writeU16(value) {
        this.ensureBufferSizeToWrite(2);
        this.view.setUint16(this.pos, value);
        this.pos += 2;
      }
      writeI16(value) {
        this.ensureBufferSizeToWrite(2);
        this.view.setInt16(this.pos, value);
        this.pos += 2;
      }
      writeU32(value) {
        this.ensureBufferSizeToWrite(4);
        this.view.setUint32(this.pos, value);
        this.pos += 4;
      }
      writeI32(value) {
        this.ensureBufferSizeToWrite(4);
        this.view.setInt32(this.pos, value);
        this.pos += 4;
      }
      writeF32(value) {
        this.ensureBufferSizeToWrite(4);
        this.view.setFloat32(this.pos, value);
        this.pos += 4;
      }
      writeF64(value) {
        this.ensureBufferSizeToWrite(8);
        this.view.setFloat64(this.pos, value);
        this.pos += 8;
      }
      writeU64(value) {
        this.ensureBufferSizeToWrite(8);
        (0, int_ts_1.setUint64)(this.view, this.pos, value);
        this.pos += 8;
      }
      writeI64(value) {
        this.ensureBufferSizeToWrite(8);
        (0, int_ts_1.setInt64)(this.view, this.pos, value);
        this.pos += 8;
      }
      writeBigUint64(value) {
        this.ensureBufferSizeToWrite(8);
        this.view.setBigUint64(this.pos, value);
        this.pos += 8;
      }
      writeBigInt64(value) {
        this.ensureBufferSizeToWrite(8);
        this.view.setBigInt64(this.pos, value);
        this.pos += 8;
      }
    };
    exports2.Encoder = Encoder;
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/encode.cjs
var require_encode = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/encode.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.encode = encode;
    var Encoder_ts_1 = require_Encoder();
    function encode(value, options) {
      const encoder = new Encoder_ts_1.Encoder(options);
      return encoder.encodeSharedRef(value);
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/prettyByte.cjs
var require_prettyByte = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/prettyByte.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.prettyByte = prettyByte;
    function prettyByte(byte) {
      return `${byte < 0 ? "-" : ""}0x${Math.abs(byte).toString(16).padStart(2, "0")}`;
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/CachedKeyDecoder.cjs
var require_CachedKeyDecoder = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/CachedKeyDecoder.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.CachedKeyDecoder = void 0;
    var utf8_ts_1 = require_utf8();
    var DEFAULT_MAX_KEY_LENGTH = 16;
    var DEFAULT_MAX_LENGTH_PER_KEY = 16;
    var CachedKeyDecoder = class {
      hit = 0;
      miss = 0;
      caches;
      maxKeyLength;
      maxLengthPerKey;
      constructor(maxKeyLength = DEFAULT_MAX_KEY_LENGTH, maxLengthPerKey = DEFAULT_MAX_LENGTH_PER_KEY) {
        this.maxKeyLength = maxKeyLength;
        this.maxLengthPerKey = maxLengthPerKey;
        this.caches = [];
        for (let i = 0; i < this.maxKeyLength; i++) {
          this.caches.push([]);
        }
      }
      canBeCached(byteLength) {
        return byteLength > 0 && byteLength <= this.maxKeyLength;
      }
      find(bytes, inputOffset, byteLength) {
        const records = this.caches[byteLength - 1];
        FIND_CHUNK: for (const record of records) {
          const recordBytes = record.bytes;
          for (let j = 0; j < byteLength; j++) {
            if (recordBytes[j] !== bytes[inputOffset + j]) {
              continue FIND_CHUNK;
            }
          }
          return record.str;
        }
        return null;
      }
      store(bytes, value) {
        const records = this.caches[bytes.length - 1];
        const record = { bytes, str: value };
        if (records.length >= this.maxLengthPerKey) {
          records[Math.random() * records.length | 0] = record;
        } else {
          records.push(record);
        }
      }
      decode(bytes, inputOffset, byteLength) {
        const cachedValue = this.find(bytes, inputOffset, byteLength);
        if (cachedValue != null) {
          this.hit++;
          return cachedValue;
        }
        this.miss++;
        const str2 = (0, utf8_ts_1.utf8DecodeJs)(bytes, inputOffset, byteLength);
        const slicedCopyOfBytes = Uint8Array.prototype.slice.call(bytes, inputOffset, inputOffset + byteLength);
        this.store(slicedCopyOfBytes, str2);
        return str2;
      }
    };
    exports2.CachedKeyDecoder = CachedKeyDecoder;
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/Decoder.cjs
var require_Decoder = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/Decoder.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.Decoder = void 0;
    var prettyByte_ts_1 = require_prettyByte();
    var ExtensionCodec_ts_1 = require_ExtensionCodec();
    var int_ts_1 = require_int();
    var utf8_ts_1 = require_utf8();
    var typedArrays_ts_1 = require_typedArrays();
    var CachedKeyDecoder_ts_1 = require_CachedKeyDecoder();
    var DecodeError_ts_1 = require_DecodeError();
    var STATE_ARRAY = "array";
    var STATE_MAP_KEY = "map_key";
    var STATE_MAP_VALUE = "map_value";
    var mapKeyConverter = (key) => {
      if (typeof key === "string" || typeof key === "number") {
        return key;
      }
      throw new DecodeError_ts_1.DecodeError("The type of key must be string or number but " + typeof key);
    };
    var StackPool = class {
      stack = [];
      stackHeadPosition = -1;
      get length() {
        return this.stackHeadPosition + 1;
      }
      top() {
        return this.stack[this.stackHeadPosition];
      }
      pushArrayState(size) {
        const state = this.getUninitializedStateFromPool();
        state.type = STATE_ARRAY;
        state.position = 0;
        state.size = size;
        state.array = new Array(size);
      }
      pushMapState(size) {
        const state = this.getUninitializedStateFromPool();
        state.type = STATE_MAP_KEY;
        state.readCount = 0;
        state.size = size;
        state.map = {};
      }
      getUninitializedStateFromPool() {
        this.stackHeadPosition++;
        if (this.stackHeadPosition === this.stack.length) {
          const partialState = {
            type: void 0,
            size: 0,
            array: void 0,
            position: 0,
            readCount: 0,
            map: void 0,
            key: null
          };
          this.stack.push(partialState);
        }
        return this.stack[this.stackHeadPosition];
      }
      release(state) {
        const topStackState = this.stack[this.stackHeadPosition];
        if (topStackState !== state) {
          throw new Error("Invalid stack state. Released state is not on top of the stack.");
        }
        if (state.type === STATE_ARRAY) {
          const partialState = state;
          partialState.size = 0;
          partialState.array = void 0;
          partialState.position = 0;
          partialState.type = void 0;
        }
        if (state.type === STATE_MAP_KEY || state.type === STATE_MAP_VALUE) {
          const partialState = state;
          partialState.size = 0;
          partialState.map = void 0;
          partialState.readCount = 0;
          partialState.type = void 0;
        }
        this.stackHeadPosition--;
      }
      reset() {
        this.stack.length = 0;
        this.stackHeadPosition = -1;
      }
    };
    var HEAD_BYTE_REQUIRED = -1;
    var EMPTY_VIEW = new DataView(new ArrayBuffer(0));
    var EMPTY_BYTES = new Uint8Array(EMPTY_VIEW.buffer);
    try {
      EMPTY_VIEW.getInt8(0);
    } catch (e) {
      if (!(e instanceof RangeError)) {
        throw new Error("This module is not supported in the current JavaScript engine because DataView does not throw RangeError on out-of-bounds access");
      }
    }
    var MORE_DATA = new RangeError("Insufficient data");
    var sharedCachedKeyDecoder = new CachedKeyDecoder_ts_1.CachedKeyDecoder();
    var Decoder = class _Decoder {
      extensionCodec;
      context;
      useBigInt64;
      rawStrings;
      maxStrLength;
      maxBinLength;
      maxArrayLength;
      maxMapLength;
      maxExtLength;
      keyDecoder;
      mapKeyConverter;
      totalPos = 0;
      pos = 0;
      view = EMPTY_VIEW;
      bytes = EMPTY_BYTES;
      headByte = HEAD_BYTE_REQUIRED;
      stack = new StackPool();
      entered = false;
      constructor(options) {
        this.extensionCodec = options?.extensionCodec ?? ExtensionCodec_ts_1.ExtensionCodec.defaultCodec;
        this.context = options?.context;
        this.useBigInt64 = options?.useBigInt64 ?? false;
        this.rawStrings = options?.rawStrings ?? false;
        this.maxStrLength = options?.maxStrLength ?? int_ts_1.UINT32_MAX;
        this.maxBinLength = options?.maxBinLength ?? int_ts_1.UINT32_MAX;
        this.maxArrayLength = options?.maxArrayLength ?? int_ts_1.UINT32_MAX;
        this.maxMapLength = options?.maxMapLength ?? int_ts_1.UINT32_MAX;
        this.maxExtLength = options?.maxExtLength ?? int_ts_1.UINT32_MAX;
        this.keyDecoder = options?.keyDecoder !== void 0 ? options.keyDecoder : sharedCachedKeyDecoder;
        this.mapKeyConverter = options?.mapKeyConverter ?? mapKeyConverter;
      }
      clone() {
        return new _Decoder({
          extensionCodec: this.extensionCodec,
          context: this.context,
          useBigInt64: this.useBigInt64,
          rawStrings: this.rawStrings,
          maxStrLength: this.maxStrLength,
          maxBinLength: this.maxBinLength,
          maxArrayLength: this.maxArrayLength,
          maxMapLength: this.maxMapLength,
          maxExtLength: this.maxExtLength,
          keyDecoder: this.keyDecoder
        });
      }
      reinitializeState() {
        this.totalPos = 0;
        this.headByte = HEAD_BYTE_REQUIRED;
        this.stack.reset();
      }
      setBuffer(buffer) {
        const bytes = (0, typedArrays_ts_1.ensureUint8Array)(buffer);
        this.bytes = bytes;
        this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        this.pos = 0;
      }
      appendBuffer(buffer) {
        if (this.headByte === HEAD_BYTE_REQUIRED && !this.hasRemaining(1)) {
          this.setBuffer(buffer);
        } else {
          const remainingData = this.bytes.subarray(this.pos);
          const newData = (0, typedArrays_ts_1.ensureUint8Array)(buffer);
          const newBuffer = new Uint8Array(remainingData.length + newData.length);
          newBuffer.set(remainingData);
          newBuffer.set(newData, remainingData.length);
          this.setBuffer(newBuffer);
        }
      }
      hasRemaining(size) {
        return this.view.byteLength - this.pos >= size;
      }
      createExtraByteError(posToShow) {
        const { view, pos } = this;
        return new RangeError(`Extra ${view.byteLength - pos} of ${view.byteLength} byte(s) found at buffer[${posToShow}]`);
      }
      /**
       * @throws {@link DecodeError}
       * @throws {@link RangeError}
       */
      decode(buffer) {
        if (this.entered) {
          const instance = this.clone();
          return instance.decode(buffer);
        }
        try {
          this.entered = true;
          this.reinitializeState();
          this.setBuffer(buffer);
          const object = this.doDecodeSync();
          if (this.hasRemaining(1)) {
            throw this.createExtraByteError(this.pos);
          }
          return object;
        } finally {
          this.entered = false;
        }
      }
      *decodeMulti(buffer) {
        if (this.entered) {
          const instance = this.clone();
          yield* instance.decodeMulti(buffer);
          return;
        }
        try {
          this.entered = true;
          this.reinitializeState();
          this.setBuffer(buffer);
          while (this.hasRemaining(1)) {
            yield this.doDecodeSync();
          }
        } finally {
          this.entered = false;
        }
      }
      async decodeAsync(stream) {
        if (this.entered) {
          const instance = this.clone();
          return instance.decodeAsync(stream);
        }
        try {
          this.entered = true;
          let decoded = false;
          let object;
          for await (const buffer of stream) {
            if (decoded) {
              this.entered = false;
              throw this.createExtraByteError(this.totalPos);
            }
            this.appendBuffer(buffer);
            try {
              object = this.doDecodeSync();
              decoded = true;
            } catch (e) {
              if (!(e instanceof RangeError)) {
                throw e;
              }
            }
            this.totalPos += this.pos;
          }
          if (decoded) {
            if (this.hasRemaining(1)) {
              throw this.createExtraByteError(this.totalPos);
            }
            return object;
          }
          const { headByte, pos, totalPos } = this;
          throw new RangeError(`Insufficient data in parsing ${(0, prettyByte_ts_1.prettyByte)(headByte)} at ${totalPos} (${pos} in the current buffer)`);
        } finally {
          this.entered = false;
        }
      }
      decodeArrayStream(stream) {
        return this.decodeMultiAsync(stream, true);
      }
      decodeStream(stream) {
        return this.decodeMultiAsync(stream, false);
      }
      async *decodeMultiAsync(stream, isArray) {
        if (this.entered) {
          const instance = this.clone();
          yield* instance.decodeMultiAsync(stream, isArray);
          return;
        }
        try {
          this.entered = true;
          let isArrayHeaderRequired = isArray;
          let arrayItemsLeft = -1;
          for await (const buffer of stream) {
            if (isArray && arrayItemsLeft === 0) {
              throw this.createExtraByteError(this.totalPos);
            }
            this.appendBuffer(buffer);
            if (isArrayHeaderRequired) {
              arrayItemsLeft = this.readArraySize();
              isArrayHeaderRequired = false;
              this.complete();
            }
            try {
              while (true) {
                yield this.doDecodeSync();
                if (--arrayItemsLeft === 0) {
                  break;
                }
              }
            } catch (e) {
              if (!(e instanceof RangeError)) {
                throw e;
              }
            }
            this.totalPos += this.pos;
          }
        } finally {
          this.entered = false;
        }
      }
      doDecodeSync() {
        DECODE: while (true) {
          const headByte = this.readHeadByte();
          let object;
          if (headByte >= 224) {
            object = headByte - 256;
          } else if (headByte < 192) {
            if (headByte < 128) {
              object = headByte;
            } else if (headByte < 144) {
              const size = headByte - 128;
              if (size !== 0) {
                this.pushMapState(size);
                this.complete();
                continue DECODE;
              } else {
                object = {};
              }
            } else if (headByte < 160) {
              const size = headByte - 144;
              if (size !== 0) {
                this.pushArrayState(size);
                this.complete();
                continue DECODE;
              } else {
                object = [];
              }
            } else {
              const byteLength = headByte - 160;
              object = this.decodeString(byteLength, 0);
            }
          } else if (headByte === 192) {
            object = null;
          } else if (headByte === 194) {
            object = false;
          } else if (headByte === 195) {
            object = true;
          } else if (headByte === 202) {
            object = this.readF32();
          } else if (headByte === 203) {
            object = this.readF64();
          } else if (headByte === 204) {
            object = this.readU8();
          } else if (headByte === 205) {
            object = this.readU16();
          } else if (headByte === 206) {
            object = this.readU32();
          } else if (headByte === 207) {
            if (this.useBigInt64) {
              object = this.readU64AsBigInt();
            } else {
              object = this.readU64();
            }
          } else if (headByte === 208) {
            object = this.readI8();
          } else if (headByte === 209) {
            object = this.readI16();
          } else if (headByte === 210) {
            object = this.readI32();
          } else if (headByte === 211) {
            if (this.useBigInt64) {
              object = this.readI64AsBigInt();
            } else {
              object = this.readI64();
            }
          } else if (headByte === 217) {
            const byteLength = this.lookU8();
            object = this.decodeString(byteLength, 1);
          } else if (headByte === 218) {
            const byteLength = this.lookU16();
            object = this.decodeString(byteLength, 2);
          } else if (headByte === 219) {
            const byteLength = this.lookU32();
            object = this.decodeString(byteLength, 4);
          } else if (headByte === 220) {
            const size = this.readU16();
            if (size !== 0) {
              this.pushArrayState(size);
              this.complete();
              continue DECODE;
            } else {
              object = [];
            }
          } else if (headByte === 221) {
            const size = this.readU32();
            if (size !== 0) {
              this.pushArrayState(size);
              this.complete();
              continue DECODE;
            } else {
              object = [];
            }
          } else if (headByte === 222) {
            const size = this.readU16();
            if (size !== 0) {
              this.pushMapState(size);
              this.complete();
              continue DECODE;
            } else {
              object = {};
            }
          } else if (headByte === 223) {
            const size = this.readU32();
            if (size !== 0) {
              this.pushMapState(size);
              this.complete();
              continue DECODE;
            } else {
              object = {};
            }
          } else if (headByte === 196) {
            const size = this.lookU8();
            object = this.decodeBinary(size, 1);
          } else if (headByte === 197) {
            const size = this.lookU16();
            object = this.decodeBinary(size, 2);
          } else if (headByte === 198) {
            const size = this.lookU32();
            object = this.decodeBinary(size, 4);
          } else if (headByte === 212) {
            object = this.decodeExtension(1, 0);
          } else if (headByte === 213) {
            object = this.decodeExtension(2, 0);
          } else if (headByte === 214) {
            object = this.decodeExtension(4, 0);
          } else if (headByte === 215) {
            object = this.decodeExtension(8, 0);
          } else if (headByte === 216) {
            object = this.decodeExtension(16, 0);
          } else if (headByte === 199) {
            const size = this.lookU8();
            object = this.decodeExtension(size, 1);
          } else if (headByte === 200) {
            const size = this.lookU16();
            object = this.decodeExtension(size, 2);
          } else if (headByte === 201) {
            const size = this.lookU32();
            object = this.decodeExtension(size, 4);
          } else {
            throw new DecodeError_ts_1.DecodeError(`Unrecognized type byte: ${(0, prettyByte_ts_1.prettyByte)(headByte)}`);
          }
          this.complete();
          const stack = this.stack;
          while (stack.length > 0) {
            const state = stack.top();
            if (state.type === STATE_ARRAY) {
              state.array[state.position] = object;
              state.position++;
              if (state.position === state.size) {
                object = state.array;
                stack.release(state);
              } else {
                continue DECODE;
              }
            } else if (state.type === STATE_MAP_KEY) {
              if (object === "__proto__") {
                throw new DecodeError_ts_1.DecodeError("The key __proto__ is not allowed");
              }
              state.key = this.mapKeyConverter(object);
              state.type = STATE_MAP_VALUE;
              continue DECODE;
            } else {
              state.map[state.key] = object;
              state.readCount++;
              if (state.readCount === state.size) {
                object = state.map;
                stack.release(state);
              } else {
                state.key = null;
                state.type = STATE_MAP_KEY;
                continue DECODE;
              }
            }
          }
          return object;
        }
      }
      readHeadByte() {
        if (this.headByte === HEAD_BYTE_REQUIRED) {
          this.headByte = this.readU8();
        }
        return this.headByte;
      }
      complete() {
        this.headByte = HEAD_BYTE_REQUIRED;
      }
      readArraySize() {
        const headByte = this.readHeadByte();
        switch (headByte) {
          case 220:
            return this.readU16();
          case 221:
            return this.readU32();
          default: {
            if (headByte < 160) {
              return headByte - 144;
            } else {
              throw new DecodeError_ts_1.DecodeError(`Unrecognized array type byte: ${(0, prettyByte_ts_1.prettyByte)(headByte)}`);
            }
          }
        }
      }
      pushMapState(size) {
        if (size > this.maxMapLength) {
          throw new DecodeError_ts_1.DecodeError(`Max length exceeded: map length (${size}) > maxMapLengthLength (${this.maxMapLength})`);
        }
        this.stack.pushMapState(size);
      }
      pushArrayState(size) {
        if (size > this.maxArrayLength) {
          throw new DecodeError_ts_1.DecodeError(`Max length exceeded: array length (${size}) > maxArrayLength (${this.maxArrayLength})`);
        }
        this.stack.pushArrayState(size);
      }
      decodeString(byteLength, headerOffset) {
        if (!this.rawStrings || this.stateIsMapKey()) {
          return this.decodeUtf8String(byteLength, headerOffset);
        }
        return this.decodeBinary(byteLength, headerOffset);
      }
      /**
       * @throws {@link RangeError}
       */
      decodeUtf8String(byteLength, headerOffset) {
        if (byteLength > this.maxStrLength) {
          throw new DecodeError_ts_1.DecodeError(`Max length exceeded: UTF-8 byte length (${byteLength}) > maxStrLength (${this.maxStrLength})`);
        }
        if (this.bytes.byteLength < this.pos + headerOffset + byteLength) {
          throw MORE_DATA;
        }
        const offset = this.pos + headerOffset;
        let object;
        if (this.stateIsMapKey() && this.keyDecoder?.canBeCached(byteLength)) {
          object = this.keyDecoder.decode(this.bytes, offset, byteLength);
        } else {
          object = (0, utf8_ts_1.utf8Decode)(this.bytes, offset, byteLength);
        }
        this.pos += headerOffset + byteLength;
        return object;
      }
      stateIsMapKey() {
        if (this.stack.length > 0) {
          const state = this.stack.top();
          return state.type === STATE_MAP_KEY;
        }
        return false;
      }
      /**
       * @throws {@link RangeError}
       */
      decodeBinary(byteLength, headOffset) {
        if (byteLength > this.maxBinLength) {
          throw new DecodeError_ts_1.DecodeError(`Max length exceeded: bin length (${byteLength}) > maxBinLength (${this.maxBinLength})`);
        }
        if (!this.hasRemaining(byteLength + headOffset)) {
          throw MORE_DATA;
        }
        const offset = this.pos + headOffset;
        const object = this.bytes.subarray(offset, offset + byteLength);
        this.pos += headOffset + byteLength;
        return object;
      }
      decodeExtension(size, headOffset) {
        if (size > this.maxExtLength) {
          throw new DecodeError_ts_1.DecodeError(`Max length exceeded: ext length (${size}) > maxExtLength (${this.maxExtLength})`);
        }
        const extType = this.view.getInt8(this.pos + headOffset);
        const data = this.decodeBinary(
          size,
          headOffset + 1
          /* extType */
        );
        return this.extensionCodec.decode(data, extType, this.context);
      }
      lookU8() {
        return this.view.getUint8(this.pos);
      }
      lookU16() {
        return this.view.getUint16(this.pos);
      }
      lookU32() {
        return this.view.getUint32(this.pos);
      }
      readU8() {
        const value = this.view.getUint8(this.pos);
        this.pos++;
        return value;
      }
      readI8() {
        const value = this.view.getInt8(this.pos);
        this.pos++;
        return value;
      }
      readU16() {
        const value = this.view.getUint16(this.pos);
        this.pos += 2;
        return value;
      }
      readI16() {
        const value = this.view.getInt16(this.pos);
        this.pos += 2;
        return value;
      }
      readU32() {
        const value = this.view.getUint32(this.pos);
        this.pos += 4;
        return value;
      }
      readI32() {
        const value = this.view.getInt32(this.pos);
        this.pos += 4;
        return value;
      }
      readU64() {
        const value = (0, int_ts_1.getUint64)(this.view, this.pos);
        this.pos += 8;
        return value;
      }
      readI64() {
        const value = (0, int_ts_1.getInt64)(this.view, this.pos);
        this.pos += 8;
        return value;
      }
      readU64AsBigInt() {
        const value = this.view.getBigUint64(this.pos);
        this.pos += 8;
        return value;
      }
      readI64AsBigInt() {
        const value = this.view.getBigInt64(this.pos);
        this.pos += 8;
        return value;
      }
      readF32() {
        const value = this.view.getFloat32(this.pos);
        this.pos += 4;
        return value;
      }
      readF64() {
        const value = this.view.getFloat64(this.pos);
        this.pos += 8;
        return value;
      }
    };
    exports2.Decoder = Decoder;
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/decode.cjs
var require_decode = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/decode.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.decode = decode2;
    exports2.decodeMulti = decodeMulti;
    var Decoder_ts_1 = require_Decoder();
    function decode2(buffer, options) {
      const decoder = new Decoder_ts_1.Decoder(options);
      return decoder.decode(buffer);
    }
    function decodeMulti(buffer, options) {
      const decoder = new Decoder_ts_1.Decoder(options);
      return decoder.decodeMulti(buffer);
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/stream.cjs
var require_stream = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/utils/stream.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.isAsyncIterable = isAsyncIterable;
    exports2.asyncIterableFromStream = asyncIterableFromStream;
    exports2.ensureAsyncIterable = ensureAsyncIterable;
    function isAsyncIterable(object) {
      return object[Symbol.asyncIterator] != null;
    }
    async function* asyncIterableFromStream(stream) {
      const reader = stream.getReader();
      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) {
            return;
          }
          yield value;
        }
      } finally {
        reader.releaseLock();
      }
    }
    function ensureAsyncIterable(streamLike) {
      if (isAsyncIterable(streamLike)) {
        return streamLike;
      } else {
        return asyncIterableFromStream(streamLike);
      }
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/decodeAsync.cjs
var require_decodeAsync = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/decodeAsync.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.decodeAsync = decodeAsync;
    exports2.decodeArrayStream = decodeArrayStream;
    exports2.decodeMultiStream = decodeMultiStream;
    var Decoder_ts_1 = require_Decoder();
    var stream_ts_1 = require_stream();
    async function decodeAsync(streamLike, options) {
      const stream = (0, stream_ts_1.ensureAsyncIterable)(streamLike);
      const decoder = new Decoder_ts_1.Decoder(options);
      return decoder.decodeAsync(stream);
    }
    function decodeArrayStream(streamLike, options) {
      const stream = (0, stream_ts_1.ensureAsyncIterable)(streamLike);
      const decoder = new Decoder_ts_1.Decoder(options);
      return decoder.decodeArrayStream(stream);
    }
    function decodeMultiStream(streamLike, options) {
      const stream = (0, stream_ts_1.ensureAsyncIterable)(streamLike);
      const decoder = new Decoder_ts_1.Decoder(options);
      return decoder.decodeStream(stream);
    }
  }
});

// ../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/index.cjs
var require_dist = __commonJS({
  "../../node_modules/.pnpm/@msgpack+msgpack@3.1.3/node_modules/@msgpack/msgpack/dist.cjs/index.cjs"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.decodeTimestampExtension = exports2.encodeTimestampExtension = exports2.decodeTimestampToTimeSpec = exports2.encodeTimeSpecToTimestamp = exports2.encodeDateToTimeSpec = exports2.EXT_TIMESTAMP = exports2.ExtData = exports2.ExtensionCodec = exports2.Encoder = exports2.DecodeError = exports2.Decoder = exports2.decodeMultiStream = exports2.decodeArrayStream = exports2.decodeAsync = exports2.decodeMulti = exports2.decode = exports2.encode = void 0;
    var encode_ts_1 = require_encode();
    Object.defineProperty(exports2, "encode", { enumerable: true, get: function() {
      return encode_ts_1.encode;
    } });
    var decode_ts_1 = require_decode();
    Object.defineProperty(exports2, "decode", { enumerable: true, get: function() {
      return decode_ts_1.decode;
    } });
    Object.defineProperty(exports2, "decodeMulti", { enumerable: true, get: function() {
      return decode_ts_1.decodeMulti;
    } });
    var decodeAsync_ts_1 = require_decodeAsync();
    Object.defineProperty(exports2, "decodeAsync", { enumerable: true, get: function() {
      return decodeAsync_ts_1.decodeAsync;
    } });
    Object.defineProperty(exports2, "decodeArrayStream", { enumerable: true, get: function() {
      return decodeAsync_ts_1.decodeArrayStream;
    } });
    Object.defineProperty(exports2, "decodeMultiStream", { enumerable: true, get: function() {
      return decodeAsync_ts_1.decodeMultiStream;
    } });
    var Decoder_ts_1 = require_Decoder();
    Object.defineProperty(exports2, "Decoder", { enumerable: true, get: function() {
      return Decoder_ts_1.Decoder;
    } });
    var DecodeError_ts_1 = require_DecodeError();
    Object.defineProperty(exports2, "DecodeError", { enumerable: true, get: function() {
      return DecodeError_ts_1.DecodeError;
    } });
    var Encoder_ts_1 = require_Encoder();
    Object.defineProperty(exports2, "Encoder", { enumerable: true, get: function() {
      return Encoder_ts_1.Encoder;
    } });
    var ExtensionCodec_ts_1 = require_ExtensionCodec();
    Object.defineProperty(exports2, "ExtensionCodec", { enumerable: true, get: function() {
      return ExtensionCodec_ts_1.ExtensionCodec;
    } });
    var ExtData_ts_1 = require_ExtData();
    Object.defineProperty(exports2, "ExtData", { enumerable: true, get: function() {
      return ExtData_ts_1.ExtData;
    } });
    var timestamp_ts_1 = require_timestamp();
    Object.defineProperty(exports2, "EXT_TIMESTAMP", { enumerable: true, get: function() {
      return timestamp_ts_1.EXT_TIMESTAMP;
    } });
    Object.defineProperty(exports2, "encodeDateToTimeSpec", { enumerable: true, get: function() {
      return timestamp_ts_1.encodeDateToTimeSpec;
    } });
    Object.defineProperty(exports2, "encodeTimeSpecToTimestamp", { enumerable: true, get: function() {
      return timestamp_ts_1.encodeTimeSpecToTimestamp;
    } });
    Object.defineProperty(exports2, "decodeTimestampToTimeSpec", { enumerable: true, get: function() {
      return timestamp_ts_1.decodeTimestampToTimeSpec;
    } });
    Object.defineProperty(exports2, "encodeTimestampExtension", { enumerable: true, get: function() {
      return timestamp_ts_1.encodeTimestampExtension;
    } });
    Object.defineProperty(exports2, "decodeTimestampExtension", { enumerable: true, get: function() {
      return timestamp_ts_1.decodeTimestampExtension;
    } });
  }
});

// src/main/index.ts
var index_exports = {};
__export(index_exports, {
  normalizeAddress: () => normalizeAddress
});
module.exports = __toCommonJS(index_exports);
var import_electron3 = require("electron");
var import_node_fs6 = require("node:fs");
var import_node_path6 = require("node:path");

// src/main/app-protocol.ts
var import_node_fs = require("node:fs");
var import_node_path = require("node:path");
var MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".map": "application/json",
  ".wasm": "application/wasm",
  ".txt": "text/plain; charset=utf-8"
};
function mimeFor(path) {
  return MIME[(0, import_node_path.extname)(path).toLowerCase()] ?? "application/octet-stream";
}
function resolveUnderRoot(root, pathname) {
  let p;
  try {
    p = decodeURIComponent(pathname);
  } catch {
    return null;
  }
  if (p.includes("\0")) return null;
  if (p === "" || p.endsWith("/")) p += "index.html";
  const target = (0, import_node_path.normalize)((0, import_node_path.join)(root, p));
  const base = (0, import_node_path.normalize)(root + import_node_path.sep);
  if (!target.startsWith(base)) return null;
  return target;
}
function cspFor(host) {
  if (host === "ui") {
    return [
      "default-src 'self'",
      "script-src 'self'",
      "style-src 'self' 'unsafe-inline'",
      "img-src 'self' data: blob:",
      "font-src 'self' data:",
      "connect-src 'self' ws://127.0.0.1:* ws://localhost:*",
      "worker-src 'self' blob:",
      "object-src 'none'",
      "base-uri 'none'",
      "frame-ancestors 'none'"
    ].join("; ");
  }
  return ["default-src 'self'", "style-src 'self' 'unsafe-inline'", "img-src 'self' data:", "object-src 'none'", "base-uri 'none'", "frame-ancestors 'none'"].join("; ");
}
function serveFile(roots, url) {
  let u;
  try {
    u = new URL(url);
  } catch {
    return text(400, "bad url");
  }
  const root = roots[u.hostname];
  if (!root) return text(404, `unknown app host: ${u.hostname}`);
  const path = resolveUnderRoot(root, u.pathname);
  if (!path) return text(403, "forbidden");
  try {
    const st = (0, import_node_fs.statSync)(path);
    if (!st.isFile()) return text(404, "not found");
    return {
      status: 200,
      headers: {
        "content-type": mimeFor(path),
        "content-security-policy": cspFor(u.hostname),
        "cache-control": u.hostname === "ui" ? "no-cache" : "no-store",
        "x-content-type-options": "nosniff"
      },
      body: (0, import_node_fs.readFileSync)(path)
    };
  } catch {
    return text(404, "not found");
  }
}
function text(status, body) {
  return { status, headers: { "content-type": "text/plain; charset=utf-8" }, body: Buffer.from(body) };
}

// src/main/browser.ts
var import_electron = require("electron");
var CHROME_HEIGHT = 78;
var BrowserShell = class {
  constructor(opts) {
    this.opts = opts;
    this.window = new import_electron.BaseWindow({ width: 1440, height: 960, title: "ipemu" });
    this.window.setMenuBarVisibility(false);
    this.chrome = new import_electron.WebContentsView({
      webPreferences: { preload: opts.preloads.shellui, sandbox: true, contextIsolation: true, nodeIntegration: false }
    });
    this.window.contentView.addChildView(this.chrome);
    void this.chrome.webContents.loadURL("app://shell/chrome.html");
    this.window.on("resize", () => this.layout());
    this.layout();
  }
  window;
  chrome;
  tabs = [];
  activeId = null;
  nextId = 1;
  get chromeContents() {
    return this.chrome.webContents;
  }
  layout() {
    const [w = 0, h = 0] = this.window.getContentSize();
    this.chrome.setBounds({ x: 0, y: 0, width: w, height: CHROME_HEIGHT });
    for (const t of this.tabs) {
      t.view.setBounds({ x: 0, y: CHROME_HEIGHT, width: w, height: Math.max(0, h - CHROME_HEIGHT) });
      t.view.setVisible(t.id === this.activeId);
    }
  }
  preloadFor(kind) {
    return kind === "sim" ? this.opts.preloads.simui : kind === "home" ? this.opts.preloads.shellui : this.opts.preloads.bridge;
  }
  newTab(url, kind = "web") {
    const view = new import_electron.WebContentsView({
      webPreferences: {
        preload: this.preloadFor(kind),
        sandbox: true,
        contextIsolation: true,
        nodeIntegration: false,
        webviewTag: false
      }
    });
    const id = this.nextId++;
    const tab = { id, kind, view };
    const wc = view.webContents;
    wc.on("will-navigate", (e, target) => {
      if (!this.opts.allowNavigation(kind, target)) e.preventDefault();
    });
    wc.on("will-redirect", (e, target) => {
      if (!this.opts.allowNavigation(kind, target)) e.preventDefault();
    });
    wc.setWindowOpenHandler(({ url: target }) => {
      if (kind === "web" && this.opts.allowNavigation("web", target)) this.newTab(target, "web");
      else if (/^https?:/.test(target)) void import_electron.shell.openExternal(target);
      return { action: "deny" };
    });
    wc.session.setPermissionRequestHandler((_wc, _perm, cb) => cb(false));
    for (const ev of ["did-start-loading", "did-stop-loading", "did-navigate", "did-navigate-in-page", "page-title-updated"]) {
      wc.on(ev, () => this.notify());
    }
    this.window.contentView.addChildView(view);
    this.tabs.push(tab);
    this.activeId = id;
    this.layout();
    void wc.loadURL(url);
    this.notify();
    return this.info(tab);
  }
  closeTab(id) {
    const i = this.tabs.findIndex((t2) => t2.id === id);
    if (i < 0) return;
    const [t] = this.tabs.splice(i, 1);
    this.window.contentView.removeChildView(t.view);
    t.view.webContents.close();
    if (this.activeId === id) this.activeId = this.tabs[Math.min(i, this.tabs.length - 1)]?.id ?? null;
    this.layout();
    this.notify();
  }
  selectTab(id) {
    if (!this.tabs.some((t) => t.id === id)) return;
    this.activeId = id;
    this.layout();
    this.notify();
  }
  findTab(pred) {
    return this.tabs.find((t) => pred(t.kind, t.view.webContents.getURL()));
  }
  get active() {
    return this.tabs.find((t) => t.id === this.activeId);
  }
  navigate(url) {
    const t = this.active;
    if (!t) {
      this.newTab(url, "web");
      return;
    }
    if (!this.opts.allowNavigation(t.kind, url)) {
      if (this.opts.allowNavigation("web", url)) this.newTab(url, "web");
      return;
    }
    void t.view.webContents.loadURL(url);
  }
  back() {
    const nh = this.active?.view.webContents.navigationHistory;
    if (nh?.canGoBack()) nh.goBack();
  }
  forward() {
    const nh = this.active?.view.webContents.navigationHistory;
    if (nh?.canGoForward()) nh.goForward();
  }
  reload() {
    this.active?.view.webContents.reload();
  }
  /** すべてのタブの WebContents (ブリッジ IPC の送信元照合に使う) */
  contentsOf(id) {
    return this.tabs.find((t) => t.view.webContents.id === id)?.view.webContents;
  }
  tabOfContents(wcId) {
    return this.tabs.find((t) => t.view.webContents.id === wcId);
  }
  sendToKind(kind, channel, ...args) {
    for (const t of this.tabs) if (t.kind === kind) t.view.webContents.send(channel, ...args);
  }
  info(t) {
    const wc = t.view.webContents;
    const nh = wc.navigationHistory;
    return {
      id: t.id,
      kind: t.kind,
      title: wc.getTitle() || wc.getURL(),
      url: wc.getURL(),
      loading: wc.isLoading(),
      canGoBack: nh.canGoBack(),
      canGoForward: nh.canGoForward(),
      active: t.id === this.activeId
    };
  }
  notify() {
    this.opts.onTabsChanged(this.tabs.map((t) => this.info(t)));
  }
};

// src/main/dialogs.ts
var import_electron2 = require("electron");
var pending = /* @__PURE__ */ new Map();
var registered = false;
function registerOnce() {
  if (registered) return;
  registered = true;
  import_electron2.ipcMain.handle("dialog:answer", (event, answer) => {
    const url = event.senderFrame?.url ?? "";
    if (!url.startsWith("app://shell/")) return false;
    const resolve2 = pending.get(event.sender.id);
    if (!resolve2) return false;
    pending.delete(event.sender.id);
    resolve2(answer);
    return true;
  });
}
function showDialog(o) {
  registerOnce();
  const win = new import_electron2.BrowserWindow({
    width: o.width ?? 520,
    height: o.height ?? 420,
    parent: o.parent,
    modal: !!o.parent,
    resizable: false,
    minimizable: false,
    maximizable: false,
    title: o.page === "confirm" ? "ipemu \u2014 \u8D77\u52D5\u306E\u78BA\u8A8D" : "ipemu \u2014 \u30A2\u30AF\u30BB\u30B9\u306E\u540C\u610F",
    webPreferences: {
      preload: o.preload,
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false
    }
  });
  win.setMenuBarVisibility(false);
  return new Promise((resolve2) => {
    let done = false;
    const finish = (v) => {
      if (done) return;
      done = true;
      pending.delete(win.webContents.id);
      resolve2(v);
      if (!win.isDestroyed()) win.close();
    };
    pending.set(win.webContents.id, (a) => finish(a));
    win.on("closed", () => finish(null));
    win.webContents.on("did-finish-load", () => sendData(win.webContents, o.data));
    void win.loadURL(`app://shell/${o.page}.html`);
  });
}
function sendData(wc, data) {
  if (!wc.isDestroyed()) wc.send("dialog:data", data);
}

// src/main/engine.ts
var import_node_child_process = require("node:child_process");
var import_node_events = require("node:events");
var import_node_fs2 = require("node:fs");
var import_node_path2 = require("node:path");
function engineCandidates(l) {
  const exe = (l.platform ?? process.platform) === "win32" ? "ipemu-engine.exe" : "ipemu-engine";
  const env = l.env ?? process.env;
  const out = [];
  if (l.configured) out.push(l.configured);
  if (env.IPEMU_ENGINE) out.push(env.IPEMU_ENGINE);
  if (l.bundledDir) out.push((0, import_node_path2.join)(l.bundledDir, exe));
  if (l.devRoot) {
    out.push((0, import_node_path2.join)(l.devRoot, "target", "release", exe));
    out.push((0, import_node_path2.join)(l.devRoot, "target", "debug", exe));
  }
  return out;
}
function locateEngine(l) {
  for (const c of engineCandidates(l)) if ((0, import_node_fs2.existsSync)(c)) return c;
  return null;
}
function parseDiscoveryLine(line) {
  try {
    const v = JSON.parse(line.trim());
    if (typeof v.port !== "number" || typeof v.token !== "string" || !v.token) return null;
    return {
      port: v.port,
      token: v.token,
      pid: typeof v.pid === "number" ? v.pid : 0,
      ipcVersion: typeof v.ipcVersion === "number" ? v.ipcVersion : 0,
      engineVersion: typeof v.engineVersion === "string" ? v.engineVersion : ""
    };
  } catch {
    return null;
  }
}
var EngineProcess = class extends import_node_events.EventEmitter {
  constructor(exe, extraArgs = []) {
    super();
    this.exe = exe;
    this.extraArgs = extraArgs;
  }
  child = null;
  _discovery = null;
  stopping = false;
  get discovery() {
    return this._discovery;
  }
  get running() {
    return this.child !== null && this.child.exitCode === null;
  }
  /** 起動して Discovery を待つ */
  start(timeoutMs = 3e4) {
    if (this.child) return Promise.reject(new Error("\u3059\u3067\u306B\u8D77\u52D5\u3057\u3066\u3044\u307E\u3059"));
    this.stopping = false;
    this._discovery = null;
    const args = ["serve", "--exit-on-stdin-close", ...this.extraArgs];
    const child = (0, import_node_child_process.spawn)(this.exe, args, { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
    this.child = child;
    this.emit("log", `spawn ${this.exe} ${args.join(" ")} (pid ${child.pid})`);
    return new Promise((resolve2, reject) => {
      let buf = "";
      let got = false;
      const timer = setTimeout(() => {
        if (!got) {
          reject(new Error("\u30A8\u30F3\u30B8\u30F3\u304C\u63A5\u7D9A\u60C5\u5831\u3092\u51FA\u3057\u307E\u305B\u3093 (timeout)"));
          this.stop();
        }
      }, timeoutMs);
      child.stdout.setEncoding("utf8");
      child.stdout.on("data", (chunk) => {
        if (got) {
          for (const l of chunk.split("\n")) if (l.trim()) this.emit("log", l);
          return;
        }
        buf += chunk;
        const nl = buf.indexOf("\n");
        if (nl === -1) return;
        const line = buf.slice(0, nl);
        const rest = buf.slice(nl + 1);
        const d = parseDiscoveryLine(line);
        got = true;
        clearTimeout(timer);
        if (!d) {
          reject(new Error(`\u30A8\u30F3\u30B8\u30F3\u306E 1 \u884C\u76EE\u3092\u89E3\u91C8\u3067\u304D\u307E\u305B\u3093: ${line}`));
          this.stop();
          return;
        }
        this._discovery = d;
        this.emit("ready", d);
        resolve2(d);
        for (const l of rest.split("\n")) if (l.trim()) this.emit("log", l);
      });
      child.stderr.setEncoding("utf8");
      child.stderr.on("data", (chunk) => {
        for (const l of chunk.split("\n")) if (l.trim()) this.emit("log", l);
      });
      child.on("error", (e) => {
        clearTimeout(timer);
        this.child = null;
        if (!got) reject(e);
        this.emit("exit", { code: null, signal: null, expected: this.stopping });
      });
      child.on("exit", (code, signal) => {
        clearTimeout(timer);
        this.child = null;
        this._discovery = null;
        if (!got) reject(new Error(`\u30A8\u30F3\u30B8\u30F3\u304C\u8D77\u52D5\u76F4\u5F8C\u306B\u7D42\u4E86\u3057\u307E\u3057\u305F (code ${code})`));
        this.emit("exit", { code, signal, expected: this.stopping });
      });
    });
  }
  /** 停止 (stdin を閉じる → 猶予後に kill) */
  stop(graceMs = 2e3) {
    const child = this.child;
    if (!child) return Promise.resolve();
    this.stopping = true;
    return new Promise((resolve2) => {
      const done = () => resolve2();
      child.once("exit", done);
      try {
        child.stdin?.end();
      } catch {
      }
      setTimeout(() => {
        if (child.exitCode === null) child.kill();
      }, graceMs);
      setTimeout(() => {
        if (child.exitCode === null) child.kill("SIGKILL");
        resolve2();
      }, graceMs + 2e3);
    });
  }
};

// src/main/ipemu-url.ts
var ID_RE = /^[a-z0-9][a-z0-9-]{0,63}$/;
function parseLaunchUrl(raw) {
  let u;
  try {
    u = new URL(raw);
  } catch {
    return { ok: false, reason: "URL \u3068\u3057\u3066\u89E3\u91C8\u3067\u304D\u307E\u305B\u3093" };
  }
  if (u.protocol !== "ipemu:") return { ok: false, reason: `scheme \u304C ipemu \u3067\u306F\u3042\u308A\u307E\u305B\u3093 (${u.protocol})` };
  const action = (u.host || u.pathname.replace(/^\/+/, "")).toLowerCase();
  if (action !== "launch") return { ok: false, reason: `\u672A\u5BFE\u5FDC\u306E\u64CD\u4F5C \`${action}\` (launch \u306E\u307F)` };
  const q = u.searchParams;
  const link = {};
  const server = q.get("server");
  if (server) {
    let s;
    try {
      s = new URL(server);
    } catch {
      return { ok: false, reason: "server \u304C URL \u3067\u306F\u3042\u308A\u307E\u305B\u3093" };
    }
    if (s.protocol !== "https:" && !(s.protocol === "http:" && (s.hostname === "localhost" || s.hostname === "127.0.0.1"))) {
      return { ok: false, reason: "server \u306F https \u3067\u3042\u308B\u5FC5\u8981\u304C\u3042\u308A\u307E\u3059" };
    }
    link.server = s.origin;
  }
  const manifest = q.get("manifest") ?? q.get("manifestId");
  if (manifest) {
    if (!ID_RE.test(manifest)) return { ok: false, reason: "manifest id \u306E\u5F62\u5F0F\u304C\u4E0D\u6B63\u3067\u3059" };
    link.manifestId = manifest;
  }
  const rev = q.get("rev") ?? q.get("revision");
  if (rev !== null && rev !== "") {
    if (!/^\d{1,9}$/.test(rev)) return { ok: false, reason: "rev \u306F\u6574\u6570\u3067\u3042\u308B\u5FC5\u8981\u304C\u3042\u308A\u307E\u3059" };
    link.revision = Number(rev);
  }
  const ticket = q.get("ticket");
  if (ticket) {
    if (ticket.length > 512 || !/^[A-Za-z0-9._~-]+$/.test(ticket)) return { ok: false, reason: "ticket \u306E\u5F62\u5F0F\u304C\u4E0D\u6B63\u3067\u3059" };
    link.ticket = ticket;
  }
  const manifestPath = q.get("manifestPath");
  if (manifestPath) {
    if (manifestPath.includes("\0") || manifestPath.length > 4096) return { ok: false, reason: "manifestPath \u304C\u4E0D\u6B63\u3067\u3059" };
    link.manifestPath = manifestPath;
  }
  if (!link.manifestPath && !(link.server && link.manifestId)) {
    return { ok: false, reason: "server \u3068 manifest (\u307E\u305F\u306F manifestPath) \u304C\u5FC5\u8981\u3067\u3059" };
  }
  return { ok: true, link };
}
function describeLink(l) {
  if (l.manifestPath) return `\u30ED\u30FC\u30AB\u30EB manifest ${l.manifestPath}`;
  return `${l.server} \u306E\u69CB\u6210 ${l.manifestId}${l.revision !== void 0 ? ` rev ${l.revision}` : " (\u6700\u65B0)"}${l.ticket ? " (\u30C1\u30B1\u30C3\u30C8\u4ED8\u304D)" : ""}`;
}

// src/main/session.ts
var import_node_events2 = require("node:events");
var import_node_fs4 = require("node:fs");
var import_node_path4 = require("node:path");

// ../ui-kit/src/ipc/client.ts
var import_msgpack = __toESM(require_dist(), 1);

// ../../crates/ipemu-ipc/bindings/index.ts
var FRAME_STATE = 1;
var FRAME_SCOPE = 2;

// ../ui-kit/src/ipc/client.ts
var RpcCallError = class extends Error {
  code;
  data;
  constructor(e) {
    super(e.message);
    this.name = "RpcCallError";
    this.code = e.code;
    this.data = e.data;
  }
};
function endpointUrl(port, token) {
  return `ws://127.0.0.1:${port}/?token=${encodeURIComponent(token)}`;
}
var EngineClient = class {
  constructor(url, WebSocketImpl = WebSocket) {
    this.url = url;
    this.WebSocketImpl = WebSocketImpl;
  }
  ws = null;
  nextId = 1;
  pending = /* @__PURE__ */ new Map();
  stateListeners = /* @__PURE__ */ new Set();
  scopeListeners = /* @__PURE__ */ new Set();
  notifyListeners = /* @__PURE__ */ new Set();
  connListeners = /* @__PURE__ */ new Set();
  _state = "closed";
  get connection() {
    return this._state;
  }
  /** 接続して open まで待つ。失敗 (トークン不一致など) は reject */
  connect() {
    if (this.ws) return Promise.resolve();
    this.setConn("connecting");
    return new Promise((resolve2, reject) => {
      const ws = new this.WebSocketImpl(this.url);
      ws.binaryType = "arraybuffer";
      this.ws = ws;
      let opened = false;
      ws.onopen = () => {
        opened = true;
        this.setConn("open");
        resolve2();
      };
      ws.onmessage = (ev) => this.onMessage(ev.data);
      ws.onerror = () => {
      };
      ws.onclose = (ev) => {
        this.ws = null;
        this.setConn("closed");
        const err = new Error(`\u63A5\u7D9A\u304C\u9589\u3058\u307E\u3057\u305F (code ${ev.code}${ev.reason ? `: ${ev.reason}` : ""})`);
        for (const p of this.pending.values()) p.reject(err);
        this.pending.clear();
        if (!opened) reject(err);
      };
    });
  }
  close() {
    this.ws?.close();
    this.ws = null;
  }
  /** 型付き呼び出し。エンジンの error は RpcCallError で reject */
  call(method, params) {
    const ws = this.ws;
    if (!ws || ws.readyState !== ws.OPEN) {
      return Promise.reject(new Error("\u30A8\u30F3\u30B8\u30F3\u306B\u63A5\u7D9A\u3057\u3066\u3044\u307E\u305B\u3093"));
    }
    const id = this.nextId++;
    const text2 = JSON.stringify({ jsonrpc: "2.0", id, method, params });
    return new Promise((resolve2, reject) => {
      this.pending.set(id, { resolve: resolve2, reject });
      ws.send(text2);
    });
  }
  onState(fn) {
    this.stateListeners.add(fn);
    return () => this.stateListeners.delete(fn);
  }
  onScope(fn) {
    this.scopeListeners.add(fn);
    return () => this.scopeListeners.delete(fn);
  }
  onNotification(fn) {
    this.notifyListeners.add(fn);
    return () => this.notifyListeners.delete(fn);
  }
  onConnection(fn) {
    this.connListeners.add(fn);
    return () => this.connListeners.delete(fn);
  }
  setConn(s) {
    this._state = s;
    for (const l of this.connListeners) l(s);
  }
  /** 受信フレームの振り分け (テストから直接呼べるよう public 相当の形にしてある) */
  onMessage(data) {
    if (typeof data === "string") {
      this.onText(data);
      return;
    }
    const bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : data;
    decodeBinaryFrame(bytes, {
      state: (f) => this.stateListeners.forEach((l) => l(f)),
      scope: (f) => this.scopeListeners.forEach((l) => l(f))
    });
  }
  onText(text2) {
    let msg;
    try {
      msg = JSON.parse(text2);
    } catch {
      return;
    }
    if (!msg || typeof msg !== "object") return;
    const m = msg;
    if ("id" in m && m.id !== void 0 && m.id !== null) {
      const id = typeof m.id === "number" ? m.id : Number(m.id);
      const p = this.pending.get(id);
      if (!p) return;
      this.pending.delete(id);
      if (m.error) p.reject(new RpcCallError(m.error));
      else p.resolve(m.result ?? null);
      return;
    }
    if (typeof m.method === "string") {
      for (const l of this.notifyListeners) l(m);
    }
  }
};
function decodeBinaryFrame(bytes, on) {
  if (bytes.length < 2) return null;
  const kind = bytes[0];
  const payload = bytes.subarray(1);
  if (kind === FRAME_STATE) {
    on.state?.((0, import_msgpack.decode)(payload));
    return "state";
  }
  if (kind === FRAME_SCOPE) {
    on.scope?.((0, import_msgpack.decode)(payload));
    return "scope";
  }
  return null;
}

// src/main/untar.ts
var import_node_crypto = require("node:crypto");
var import_node_fs3 = require("node:fs");
var import_node_path3 = require("node:path");
var import_node_zlib = require("node:zlib");
function str(b, off, len) {
  const end = b.indexOf(0, off);
  const stop = end === -1 || end > off + len ? off + len : end;
  return Buffer.from(b.subarray(off, stop)).toString("utf8");
}
function octal(b, off, len) {
  const s = str(b, off, len).trim();
  return s ? parseInt(s, 8) : 0;
}
function isSafeRelativePath(p) {
  if (!p || p.includes("\0")) return false;
  if (p.startsWith("/") || p.startsWith("\\") || /^[A-Za-z]:/.test(p)) return false;
  const parts = p.split(/[\\/]+/);
  return parts.every((x) => x !== ".." && x !== "");
}
function parseTar(buf) {
  const out = [];
  let off = 0;
  let longName = null;
  while (off + 512 <= buf.length) {
    const hdr = buf.subarray(off, off + 512);
    if (hdr.every((x) => x === 0)) break;
    const name = str(hdr, 0, 100);
    const size = octal(hdr, 124, 12);
    const type = String.fromCharCode(hdr[156] ?? 48);
    const magic = str(hdr, 257, 6);
    const prefix = magic.startsWith("ustar") ? str(hdr, 345, 155) : "";
    const dataStart = off + 512;
    const dataEnd = dataStart + size;
    if (dataEnd > buf.length) throw new Error("tar \u304C\u9014\u4E2D\u3067\u5207\u308C\u3066\u3044\u307E\u3059");
    const data = buf.subarray(dataStart, dataEnd);
    off = dataStart + Math.ceil(size / 512) * 512;
    if (type === "L") {
      longName = str(data, 0, data.length);
      continue;
    }
    let full = longName ?? (prefix ? `${prefix}/${name}` : name);
    longName = null;
    full = full.replace(/^\.\//, "");
    if (type === "5" || full.endsWith("/")) continue;
    if (type === "x" || type === "g") continue;
    if (type !== "0" && type !== "\0" && type !== "7") {
      throw new Error(`tar \u306B\u672A\u5BFE\u5FDC\u306E\u30A8\u30F3\u30C8\u30EA (type ${JSON.stringify(type)}): ${full}`);
    }
    if (!isSafeRelativePath(full)) throw new Error(`tar \u306B\u5371\u967A\u306A\u7D4C\u8DEF: ${full}`);
    out.push({ path: full.split("\\").join("/"), data });
  }
  return out;
}
function extractTarGz(gz) {
  return parseTar((0, import_node_zlib.gunzipSync)(gz));
}
function sha256Hex(data) {
  return (0, import_node_crypto.createHash)("sha256").update(data).digest("hex");
}
function writeEntries(entries, dest) {
  const root = (0, import_node_path3.normalize)(dest + import_node_path3.sep);
  for (const e of entries) {
    const target = (0, import_node_path3.normalize)((0, import_node_path3.join)(dest, e.path));
    if (!target.startsWith(root)) throw new Error(`\u5C55\u958B\u5148\u306E\u5916: ${e.path}`);
    (0, import_node_fs3.mkdirSync)((0, import_node_path3.dirname)(target), { recursive: true });
    (0, import_node_fs3.writeFileSync)(target, e.data);
  }
}

// src/main/session.ts
var SessionManager = class extends import_node_events2.EventEmitter {
  constructor(opts) {
    super();
    this.opts = opts;
  }
  proc = null;
  client = null;
  lastLaunch = null;
  lastDesc = null;
  /** 展開済み UI バンドルのディレクトリ (app://ui の root) */
  uiRoot = null;
  restarting = false;
  get discovery() {
    return this.proc?.discovery ?? null;
  }
  /** エンジンが動いていなければ起動して接続する */
  async ensureEngine() {
    if (this.proc?.running && this.proc.discovery && this.client?.connection === "open") return this.proc.discovery;
    await this.shutdownEngine();
    const proc = new EngineProcess(this.opts.enginePath, this.opts.engineArgs);
    proc.on("log", (l) => this.opts.log(`[engine] ${l}`));
    proc.on("exit", (info) => {
      this.opts.log(`[engine] exit code=${info.code} signal=${info.signal} expected=${info.expected}`);
      this.client?.close();
      this.client = null;
      if (this.proc === proc) this.proc = null;
      this.emit("engine-exit", info);
      if (!info.expected && !this.restarting && this.lastLaunch) this.emit("toast", { level: "warn", text: "\u30A8\u30F3\u30B8\u30F3\u304C\u7D42\u4E86\u3057\u307E\u3057\u305F\u3002\u6B21\u306E\u64CD\u4F5C\u3067\u518D\u8D77\u52D5\u3057\u307E\u3059" });
    });
    this.proc = proc;
    const d = await proc.start();
    const client = new EngineClient(endpointUrl(d.port, d.token));
    client.onNotification((n) => this.onNotification(n));
    await client.connect();
    this.client = client;
    this.emit("endpoint", { port: d.port, token: d.token });
    return d;
  }
  onNotification(n) {
    if (n.method === "consent.required") this.emit("consent-required", n.params);
    else if (n.method === "event") {
      const e = n.params;
      if (e.kind === "fault") this.emit("toast", { level: "error", text: `FAULT: ${e.message}` });
      if (e.kind === "firmware" && e.data && e.data.restartRequired) {
        this.emit("toast", { level: "warn", text: e.message });
      }
    }
  }
  /** 構成を起動 (実行中のセッションは置き換わる)。UI バンドルがあれば展開して app://ui を更新する */
  async launch(p) {
    await this.ensureEngine();
    const desc = await this.client.call("session.launch", p);
    this.lastLaunch = p;
    this.lastDesc = desc;
    this.prepareUi(desc.ui ?? null);
    this.emit("session", desc);
    return desc;
  }
  async describe() {
    if (!this.client || this.client.connection !== "open") return null;
    const d = await this.client.call("session.describe", {});
    this.lastDesc = d.session ?? null;
    return this.lastDesc;
  }
  async catalogLocal() {
    await this.ensureEngine();
    return this.client.call("catalog.local", {});
  }
  async consentAnswer(requestId, grant, remember) {
    if (!this.client) throw new Error("\u30A8\u30F3\u30B8\u30F3\u306B\u63A5\u7D9A\u3057\u3066\u3044\u307E\u305B\u3093");
    return this.client.call("consent.answer", { requestId, grant, remember });
  }
  /**
   * エンジンの再起動 (native FW の差し替えなど、05-firmware.md「リビルド後の反映」)。
   * 直近の describe にある上書き (firmware / params) を launch に畳み込んで同じ構成を組み立て直す。
   */
  async restart() {
    const base = this.lastLaunch;
    if (!base) {
      await this.shutdownEngine();
      await this.ensureEngine();
      return null;
    }
    const latest = await this.describe().catch(() => null) ?? this.lastDesc;
    const merged = {
      ...base,
      firmware: { ...base.firmware, ...latest?.overrides.firmware ?? {} },
      params: mergeParams(base.params, latest?.overrides.params ?? []),
      pacing: latest ? latest.run.pacing : base.pacing,
      autoStart: latest ? latest.run.state === "running" : base.autoStart
    };
    this.restarting = true;
    try {
      await this.shutdownEngine();
      this.emit("toast", { level: "info", text: "\u30A8\u30F3\u30B8\u30F3\u3092\u518D\u8D77\u52D5\u3057\u3066\u3044\u307E\u3059\u2026" });
      return await this.launch(merged);
    } finally {
      this.restarting = false;
    }
  }
  async status() {
    const d = this.proc?.discovery ?? null;
    const s = await this.describe().catch(() => null);
    return {
      engine: { running: !!this.proc?.running, port: d?.port, pid: d?.pid, version: d?.engineVersion },
      session: s ? { id: s.manifest.id, revision: s.manifest.revision, name: s.manifest.name, run: s.run } : null,
      uiReady: this.uiRoot !== null
    };
  }
  /**
   * UI バンドル (単一ファイルの tar.gz アーティファクト) を <uiCacheDir>/<sha256>/ に展開する。
   * sha256 が manifest にあればそれで、なければ中身のハッシュでディレクトリを決め、同じものは展開し直さない。
   */
  prepareUi(ui) {
    this.uiRoot = null;
    if (!ui) return;
    if (!(0, import_node_fs4.existsSync)(ui.path)) {
      this.opts.log(`[ui] \u30D0\u30F3\u30C9\u30EB\u304C\u3042\u308A\u307E\u305B\u3093: ${ui.path} (apps/sim-ui \u3092 build \u3057\u3066\u304F\u3060\u3055\u3044)`);
      this.emit("toast", { level: "warn", text: `UI \u30D0\u30F3\u30C9\u30EB\u304C\u3042\u308A\u307E\u305B\u3093: ${ui.path}` });
      return;
    }
    try {
      const bytes = (0, import_node_fs4.readFileSync)(ui.path);
      const sha = sha256Hex(bytes);
      if (ui.sha256 && ui.sha256 !== sha) throw new Error(`sha256 \u4E0D\u4E00\u81F4 (manifest ${ui.sha256}, \u5B9F\u4F53 ${sha})`);
      const dir = (0, import_node_path4.join)(this.opts.uiCacheDir, sha);
      const stamp = (0, import_node_path4.join)(dir, ".ipemu-ok");
      if (!(0, import_node_fs4.existsSync)(stamp)) {
        (0, import_node_fs4.rmSync)(dir, { recursive: true, force: true });
        (0, import_node_fs4.mkdirSync)(dir, { recursive: true });
        const entries = extractTarGz(bytes);
        if (!entries.some((e) => e.path === "index.html")) throw new Error("index.html \u304C\u3042\u308A\u307E\u305B\u3093");
        writeEntries(entries, dir);
        (0, import_node_fs4.writeFileSync)(stamp, `${ui.artifact}
`);
      }
      this.uiRoot = dir;
      this.opts.log(`[ui] ${ui.artifact} \u2192 ${dir}`);
    } catch (e) {
      this.opts.log(`[ui] \u5C55\u958B\u306B\u5931\u6557: ${String(e)}`);
      this.emit("toast", { level: "error", text: `UI \u30D0\u30F3\u30C9\u30EB\u3092\u5C55\u958B\u3067\u304D\u307E\u305B\u3093: ${String(e)}` });
    }
  }
  async shutdownEngine() {
    this.client?.close();
    this.client = null;
    const p = this.proc;
    this.proc = null;
    if (p) await p.stop();
  }
};
function mergeParams(a, b) {
  const out = [...a];
  for (const o of b) {
    const i = out.findIndex((x) => x.instance === o.instance && x.key === o.key);
    if (i >= 0) out[i] = o;
    else out.push(o);
  }
  return out;
}

// src/main/settings.ts
var import_node_fs5 = require("node:fs");
var import_node_path5 = require("node:path");
var DEFAULT_SETTINGS = {
  trustedServers: [],
  allowLocalhost: true,
  allowLocalLaunch: true,
  catalogDirs: [],
  registerProtocol: true
};
function loadSettings(path) {
  if (!(0, import_node_fs5.existsSync)(path)) return { ...DEFAULT_SETTINGS };
  try {
    const raw = JSON.parse((0, import_node_fs5.readFileSync)(path, "utf8"));
    return sanitize(raw);
  } catch (e) {
    console.warn(`settings.json \u3092\u8AAD\u3081\u307E\u305B\u3093 (${String(e)})\u3002\u65E2\u5B9A\u5024\u3092\u4F7F\u3044\u307E\u3059`);
    return { ...DEFAULT_SETTINGS };
  }
}
function sanitize(raw) {
  const strs = (v) => Array.isArray(v) ? v.filter((x) => typeof x === "string") : [];
  return {
    trustedServers: strs(raw.trustedServers),
    allowLocalhost: raw.allowLocalhost ?? DEFAULT_SETTINGS.allowLocalhost,
    allowLocalLaunch: raw.allowLocalLaunch ?? DEFAULT_SETTINGS.allowLocalLaunch,
    enginePath: typeof raw.enginePath === "string" ? raw.enginePath : void 0,
    catalogDirs: strs(raw.catalogDirs),
    homeUrl: typeof raw.homeUrl === "string" ? raw.homeUrl : void 0,
    registerProtocol: raw.registerProtocol ?? DEFAULT_SETTINGS.registerProtocol
  };
}
function saveSettings(path, s) {
  (0, import_node_fs5.mkdirSync)((0, import_node_path5.dirname)(path), { recursive: true });
  (0, import_node_fs5.writeFileSync)(path, JSON.stringify(s, null, 2) + "\n");
}

// src/main/trusted-origins.ts
function normalizeOrigin(entry) {
  const s = entry.trim();
  if (!s) return null;
  let u;
  try {
    u = new URL(s.includes("://") ? s : `https://${s}`);
  } catch {
    return null;
  }
  if (u.protocol !== "https:" && u.protocol !== "http:") return null;
  if (u.username || u.password) return null;
  return u.origin;
}
function isLoopbackHost(host) {
  return host === "localhost" || host === "127.0.0.1" || host === "[::1]";
}
function isTrustedOrigin(url, trustedServers, opts = {}) {
  let u;
  try {
    u = new URL(url);
  } catch {
    return false;
  }
  if (u.origin === "null") return false;
  if (u.protocol === "https:") {
  } else if (u.protocol === "http:") {
    if (!(opts.allowLocalhost && isLoopbackHost(u.hostname))) return false;
  } else {
    return false;
  }
  const origin = u.origin;
  for (const entry of trustedServers) {
    const t = normalizeOrigin(entry);
    if (!t) continue;
    if (t.startsWith("http:") && !(opts.allowLocalhost && isLoopbackHost(new URL(t).hostname))) continue;
    if (t === origin) return true;
  }
  return false;
}

// src/main/index.ts
var distDir = (0, import_node_path6.resolve)(__dirname, "..");
var rendererDir = (0, import_node_path6.join)(distDir, "renderer");
var preloads = {
  bridge: (0, import_node_path6.join)(distDir, "preload", "bridge.cjs"),
  simui: (0, import_node_path6.join)(distDir, "preload", "simui.cjs"),
  shellui: (0, import_node_path6.join)(distDir, "preload", "shellui.cjs")
};
var devRoot = (0, import_node_path6.resolve)(distDir, "..", "..", "..");
var settingsPath = () => (0, import_node_path6.join)(import_electron3.app.getPath("userData"), "settings.json");
var settings;
var browser = null;
var sessions;
var log = (line) => console.log(`[ipemu-shell] ${line}`);
import_electron3.protocol.registerSchemesAsPrivileged([
  { scheme: "app", privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: false, stream: true } }
]);
if (!import_electron3.app.requestSingleInstanceLock()) {
  import_electron3.app.quit();
} else {
  import_electron3.app.on("second-instance", (_e, argv) => {
    const link = argv.find((a) => a.startsWith("ipemu:"));
    if (link) void handleLaunchLink(link);
    if (browser) browser.window.focus();
  });
  import_electron3.app.on("open-url", (e, url) => {
    e.preventDefault();
    void whenReady().then(() => handleLaunchLink(url));
  });
  void main();
}
var readyPromise = null;
function whenReady() {
  return readyPromise ??= import_electron3.app.whenReady();
}
async function main() {
  if (process.argv.includes("--no-sandbox-if-root") && process.getuid?.() === 0) import_electron3.app.commandLine.appendSwitch("no-sandbox");
  if (process.env.IPEMU_SHELL_USER_DATA) import_electron3.app.setPath("userData", process.env.IPEMU_SHELL_USER_DATA);
  await whenReady();
  settings = loadSettings(settingsPath());
  log(`settings: ${settingsPath()} trustedServers=${JSON.stringify(settings.trustedServers)}`);
  if (settings.registerProtocol && import_electron3.app.isPackaged) {
    const ok = import_electron3.app.setAsDefaultProtocolClient("ipemu");
    log(`ipemu:// handler registered: ${ok}`);
  }
  import_electron3.protocol.handle("app", (req) => {
    const r = serveFile({ shell: rendererDir, ui: sessions.uiRoot ?? void 0 }, req.url);
    return new Response(r.body, { status: r.status, headers: r.headers });
  });
  const enginePath = locateEngine({
    configured: settings.enginePath,
    bundledDir: import_electron3.app.isPackaged ? (0, import_node_path6.join)(process.resourcesPath, "bin") : void 0,
    devRoot
  });
  if (!enginePath) {
    log(`ipemu-engine \u304C\u898B\u3064\u304B\u308A\u307E\u305B\u3093\u3002settings.enginePath \u304B IPEMU_ENGINE \u3067\u6307\u5B9A\u3057\u3066\u304F\u3060\u3055\u3044 (\u958B\u767A: cargo build -p ipemu-engine)`);
  }
  const catalogDirs = settings.catalogDirs.length ? settings.catalogDirs : (0, import_node_fs6.existsSync)((0, import_node_path6.join)(devRoot, "dev")) ? [(0, import_node_path6.join)(devRoot, "dev")] : [];
  sessions = new SessionManager({
    enginePath: enginePath ?? "ipemu-engine",
    engineArgs: catalogDirs.flatMap((d) => ["--catalog-dir", d]),
    uiCacheDir: (0, import_node_path6.join)(import_electron3.app.getPath("userData"), "ui-cache"),
    log
  });
  sessions.on("toast", (t) => toast(t.level, t.text));
  sessions.on("endpoint", (ep) => browser?.sendToKind("sim", "simui:endpoint-changed", ep));
  sessions.on("consent-required", (req) => void handleConsent(req));
  sessions.on("session", () => {
    const t = browser?.findTab((k) => k === "sim");
    if (t) t.view.webContents.reload();
  });
  registerIpc();
  browser = new BrowserShell({
    preloads,
    allowNavigation,
    onTabsChanged: (tabs) => browser?.chromeContents.send("shell:tabs", tabs)
  });
  browser.newTab(settings.homeUrl ?? "app://shell/home.html", settings.homeUrl ? "web" : "home");
  const link = process.argv.find((a) => a.startsWith("ipemu:"));
  if (link) void handleLaunchLink(link);
  import_electron3.app.on("window-all-closed", () => {
    void sessions.shutdownEngine().finally(() => import_electron3.app.quit());
  });
  import_electron3.app.on("before-quit", () => {
    for (const w of import_electron3.BrowserWindow.getAllWindows()) if (!w.isDestroyed()) w.destroy();
    void sessions.shutdownEngine();
  });
}
function allowNavigation(kind, url) {
  if (kind === "sim") return url.startsWith("app://ui/");
  if (kind === "home") return url.startsWith("app://shell/");
  return /^(https?:|about:blank)/.test(url);
}
function toast(level, text2) {
  log(`${level}: ${text2}`);
  browser?.chromeContents.send("shell:toast", { level, text: text2 });
}
async function handleLaunchLink(raw) {
  const parsed = parseLaunchUrl(raw);
  if (!parsed.ok) {
    toast("error", `ipemu \u30EA\u30F3\u30AF\u3092\u53D7\u3051\u4ED8\u3051\u307E\u305B\u3093: ${parsed.reason}`);
    return;
  }
  const link = parsed.link;
  const problems = [];
  if (link.server && !isTrustedOrigin(link.server + "/", settings.trustedServers, { allowLocalhost: settings.allowLocalhost })) {
    problems.push(`${link.server} \u306F\u4FE1\u983C\u3059\u308B\u30B5\u30FC\u30D0\u306B\u767B\u9332\u3055\u308C\u3066\u3044\u307E\u305B\u3093`);
  }
  if (link.manifestPath && !settings.allowLocalLaunch) problems.push("\u30ED\u30FC\u30AB\u30EB manifest \u306E\u8D77\u52D5\u306F\u8A2D\u5B9A\u3067\u7121\u52B9\u3067\u3059");
  const answer = await showDialog({
    page: "confirm",
    preload: preloads.shellui,
    parent: void 0,
    data: { summary: describeLink(link), link, problems, raw }
  });
  if (!answer?.ok || problems.length) return;
  await launchFromLink(link);
}
async function launchFromLink(link) {
  const params = link.manifestPath ? { source: { kind: "local", manifestPath: link.manifestPath, allowUnhashed: true }, firmware: {}, params: [], pacing: { mode: "realtime", speed: 1 }, autoStart: false } : {
    source: {
      kind: "server",
      server: link.server,
      manifestId: link.manifestId,
      revision: link.revision,
      credential: { kind: "ticket", ticket: link.ticket ?? "" }
    },
    firmware: {},
    params: [],
    pacing: { mode: "realtime", speed: 1 },
    autoStart: false
  };
  await doLaunch(params);
}
async function doLaunch(params) {
  try {
    const desc = await sessions.launch(params);
    toast("info", `\u7D44\u307F\u7ACB\u3066\u307E\u3057\u305F: ${desc.manifest.name}`);
    openSimulator();
    return { ok: true, session: { id: desc.manifest.id, revision: desc.manifest.revision, name: desc.manifest.name } };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    toast("error", `\u8D77\u52D5\u306B\u5931\u6557: ${msg}`);
    return { ok: false, error: msg };
  }
}
function openSimulator() {
  if (!browser) return;
  if (!sessions.uiRoot) {
    toast("warn", "UI \u30D0\u30F3\u30C9\u30EB\u304C\u672A\u5C55\u958B\u3067\u3059 (manifest \u306E ui.bundle \u3092\u78BA\u8A8D\u3057\u3066\u304F\u3060\u3055\u3044)");
    return;
  }
  const t = browser.findTab((k) => k === "sim");
  if (t) {
    browser.selectTab(t.id);
    t.view.webContents.reload();
  } else browser.newTab("app://ui/index.html", "sim");
}
async function handleConsent(req) {
  const parent = browser ? import_electron3.BrowserWindow.getFocusedWindow() ?? void 0 : void 0;
  const a = await showDialog({ page: "consent", preload: preloads.shellui, data: req, parent });
  const grant = a?.grant ?? false;
  const remember = a?.remember ?? false;
  try {
    await sessions.consentAnswer(req.requestId, grant, remember);
  } catch (e) {
    toast("error", `\u540C\u610F\u306E\u9001\u4FE1\u306B\u5931\u6557: ${String(e)}`);
  }
}
function senderIs(event, pred) {
  const url = event.senderFrame?.url ?? "";
  const tab = browser?.tabOfContents(event.sender.id);
  const kind = tab ? tab.kind : event.sender.id === browser?.chromeContents.id ? "chrome" : "dialog";
  return pred(url, kind);
}
var isShellPage = (url) => url.startsWith("app://shell/");
var isTrustedWeb = (url) => isTrustedOrigin(url, settings.trustedServers, { allowLocalhost: settings.allowLocalhost });
function registerIpc() {
  import_electron3.ipcMain.on("ipemu:bridge-allowed", (event) => {
    event.returnValue = senderIs(event, (url, kind) => kind === "web" && isTrustedWeb(url));
  });
  const guardBridge = (event) => {
    if (!senderIs(event, (url, kind) => kind === "web" && isTrustedWeb(url) || (kind === "home" || kind === "chrome") && isShellPage(url))) {
      throw new Error("\u3053\u306E origin \u304B\u3089\u306F\u8A31\u53EF\u3055\u308C\u3066\u3044\u307E\u305B\u3093");
    }
  };
  import_electron3.ipcMain.handle("ipemu:launch", async (event, req) => {
    guardBridge(event);
    const r = req ?? {};
    const origin = new URL(event.senderFrame?.url ?? "about:blank").origin;
    const params = r.manifestPath ? { source: { kind: "local", manifestPath: r.manifestPath, allowUnhashed: true }, firmware: {}, params: [], pacing: { mode: "realtime", speed: 1 }, autoStart: false } : {
      source: {
        kind: "server",
        server: r.server ?? origin,
        // ページ自身の origin を既定にする (他サーバへ資格情報を流させない)
        manifestId: String(r.manifestId ?? ""),
        revision: typeof r.revision === "number" ? r.revision : void 0,
        credential: { kind: "sessionToken", token: String(r.sessionToken ?? "") }
      },
      firmware: {},
      params: [],
      pacing: { mode: "realtime", speed: 1 },
      autoStart: false
    };
    if (params.source.kind === "server" && params.source.server !== origin) throw new Error("server \u306F\u30DA\u30FC\u30B8\u81EA\u8EAB\u306E origin \u3067\u3042\u308B\u5FC5\u8981\u304C\u3042\u308A\u307E\u3059");
    return doLaunch(params);
  });
  import_electron3.ipcMain.handle("ipemu:status", (event) => {
    guardBridge(event);
    return sessions.status();
  });
  import_electron3.ipcMain.handle("ipemu:open-simulator", (event) => {
    guardBridge(event);
    openSimulator();
  });
  const guardSim = (event) => {
    if (!senderIs(event, (url, kind) => kind === "sim" && url.startsWith("app://ui/"))) throw new Error("not allowed");
  };
  import_electron3.ipcMain.handle("simui:endpoint", (event) => {
    guardSim(event);
    const d = sessions.discovery;
    return d ? { port: d.port, token: d.token } : null;
  });
  import_electron3.ipcMain.handle("simui:restart-engine", async (event) => {
    guardSim(event);
    await sessions.restart();
  });
  const guardShell = (event) => {
    if (!senderIs(event, (url, kind) => (kind === "chrome" || kind === "home") && isShellPage(url))) throw new Error("not allowed");
  };
  import_electron3.ipcMain.handle("shell:navigate", (e, url) => {
    guardShell(e);
    browser?.navigate(normalizeAddress(url));
  });
  import_electron3.ipcMain.handle("shell:back", (e) => (guardShell(e), browser?.back()));
  import_electron3.ipcMain.handle("shell:forward", (e) => (guardShell(e), browser?.forward()));
  import_electron3.ipcMain.handle("shell:reload", (e) => (guardShell(e), browser?.reload()));
  import_electron3.ipcMain.handle("shell:new-tab", (e, url) => {
    guardShell(e);
    const u = url ? normalizeAddress(url) : "app://shell/home.html";
    browser?.newTab(u, u.startsWith("app://shell/") ? "home" : "web");
  });
  import_electron3.ipcMain.handle("shell:close-tab", (e, id) => (guardShell(e), browser?.closeTab(id)));
  import_electron3.ipcMain.handle("shell:select-tab", (e, id) => (guardShell(e), browser?.selectTab(id)));
  import_electron3.ipcMain.handle("shell:get-settings", (e) => (guardShell(e), { ...settings, settingsPath: settingsPath() }));
  import_electron3.ipcMain.handle("shell:set-settings", (e, patch) => {
    guardShell(e);
    settings = { ...settings, ...patch };
    saveSettings(settingsPath(), settings);
    return settings;
  });
  import_electron3.ipcMain.handle("shell:catalog-local", async (e) => {
    guardShell(e);
    return sessions.catalogLocal();
  });
  import_electron3.ipcMain.handle("shell:launch-local", (e, manifestPath) => {
    guardShell(e);
    return doLaunch({ source: { kind: "local", manifestPath, allowUnhashed: true }, firmware: {}, params: [], pacing: { mode: "realtime", speed: 1 }, autoStart: false });
  });
}
function normalizeAddress(input) {
  const s = input.trim();
  if (!s) return "app://shell/home.html";
  if (/^(https?|app|about):/i.test(s)) return s;
  if (/^[\w.-]+(:\d+)?(\/.*)?$/.test(s) && (s.includes(".") || s.startsWith("localhost"))) return `https://${s}`;
  return `https://duckduckgo.com/?q=${encodeURIComponent(s)}`;
}
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  normalizeAddress
});
//# sourceMappingURL=index.cjs.map
