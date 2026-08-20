var __defProp = Object.defineProperty;
var __defProps = Object.defineProperties;
var __getOwnPropDescs = Object.getOwnPropertyDescriptors;
var __getOwnPropSymbols = Object.getOwnPropertySymbols;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __propIsEnum = Object.prototype.propertyIsEnumerable;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __spreadValues = (a, b) => {
  for (var prop in b || (b = {}))
    if (__hasOwnProp.call(b, prop))
      __defNormalProp(a, prop, b[prop]);
  if (__getOwnPropSymbols)
    for (var prop of __getOwnPropSymbols(b)) {
      if (__propIsEnum.call(b, prop))
        __defNormalProp(a, prop, b[prop]);
    }
  return a;
};
var __spreadProps = (a, b) => __defProps(a, __getOwnPropDescs(b));
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);

// src/text-codec.mts
function appendCodePoint(output, codePoint) {
  if (codePoint <= 65535) {
    output.push(String.fromCharCode(codePoint));
    return;
  }
  const value = codePoint - 65536;
  output.push(String.fromCharCode(55296 + (value >> 10), 56320 + (value & 1023)));
}
var TextEncoder = class {
  constructor() {
    __publicField(this, "encoding", "utf-8");
  }
  encode(input = "") {
    const bytes = [];
    for (let index = 0; index < input.length; index += 1) {
      let codePoint = input.charCodeAt(index);
      if (codePoint >= 55296 && codePoint <= 56319) {
        const trailing = input.charCodeAt(index + 1);
        if (trailing >= 56320 && trailing <= 57343) {
          codePoint = 65536 + (codePoint - 55296 << 10) + trailing - 56320;
          index += 1;
        } else {
          codePoint = 65533;
        }
      } else if (codePoint >= 56320 && codePoint <= 57343) {
        codePoint = 65533;
      }
      if (codePoint <= 127) {
        bytes.push(codePoint);
      } else if (codePoint <= 2047) {
        bytes.push(192 | codePoint >> 6, 128 | codePoint & 63);
      } else if (codePoint <= 65535) {
        bytes.push(
          224 | codePoint >> 12,
          128 | codePoint >> 6 & 63,
          128 | codePoint & 63
        );
      } else {
        bytes.push(
          240 | codePoint >> 18,
          128 | codePoint >> 12 & 63,
          128 | codePoint >> 6 & 63,
          128 | codePoint & 63
        );
      }
    }
    return new Uint8Array(bytes);
  }
};
var TextDecoder = class {
  constructor() {
    __publicField(this, "encoding", "utf-8");
    __publicField(this, "fatal", false);
    __publicField(this, "ignoreBOM", true);
  }
  decode(input = new Uint8Array()) {
    const bytes = input instanceof ArrayBuffer ? new Uint8Array(input) : new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
    const output = [];
    for (let index = 0; index < bytes.length; ) {
      const leading = bytes[index];
      if (leading <= 127) {
        appendCodePoint(output, leading);
        index += 1;
        continue;
      }
      const width = leading >= 194 && leading <= 223 ? 2 : leading >= 224 && leading <= 239 ? 3 : leading >= 240 && leading <= 244 ? 4 : 0;
      if (width === 0 || index + width > bytes.length) {
        appendCodePoint(output, 65533);
        index += 1;
        continue;
      }
      let codePoint = leading & 127 >> width;
      let valid = true;
      for (let offset = 1; offset < width; offset += 1) {
        const continuation = bytes[index + offset];
        if ((continuation & 192) !== 128) {
          valid = false;
          break;
        }
        codePoint = codePoint << 6 | continuation & 63;
      }
      const minimum = width === 2 ? 128 : width === 3 ? 2048 : 65536;
      if (!valid || codePoint < minimum || codePoint > 1114111 || codePoint >= 55296 && codePoint <= 57343) {
        appendCodePoint(output, 65533);
        index += 1;
        continue;
      }
      appendCodePoint(output, codePoint);
      index += width;
    }
    return output.join("");
  }
};

// node_modules/flatbuffers/mjs/constants.js
var SIZEOF_SHORT = 2;
var SIZEOF_INT = 4;
var FILE_IDENTIFIER_LENGTH = 4;
var SIZE_PREFIX_LENGTH = 4;

// node_modules/flatbuffers/mjs/utils.js
var int32 = new Int32Array(2);
var float32 = new Float32Array(int32.buffer);
var float64 = new Float64Array(int32.buffer);
var isLittleEndian = new Uint16Array(new Uint8Array([1, 0]).buffer)[0] === 1;

// node_modules/flatbuffers/mjs/encoding.js
var Encoding;
(function(Encoding2) {
  Encoding2[Encoding2["UTF8_BYTES"] = 1] = "UTF8_BYTES";
  Encoding2[Encoding2["UTF16_STRING"] = 2] = "UTF16_STRING";
})(Encoding || (Encoding = {}));

// node_modules/flatbuffers/mjs/byte-buffer.js
var ByteBuffer = class _ByteBuffer {
  /**
   * Create a new ByteBuffer with a given array of bytes (`Uint8Array`)
   */
  constructor(bytes_) {
    this.bytes_ = bytes_;
    this.position_ = 0;
    this.text_decoder_ = new TextDecoder();
  }
  /**
   * Create and allocate a new ByteBuffer with a given size.
   */
  static allocate(byte_size) {
    return new _ByteBuffer(new Uint8Array(byte_size));
  }
  clear() {
    this.position_ = 0;
  }
  /**
   * Get the underlying `Uint8Array`.
   */
  bytes() {
    return this.bytes_;
  }
  /**
   * Get the buffer's position.
   */
  position() {
    return this.position_;
  }
  /**
   * Set the buffer's position.
   */
  setPosition(position) {
    this.position_ = position;
  }
  /**
   * Get the buffer's capacity.
   */
  capacity() {
    return this.bytes_.length;
  }
  readInt8(offset) {
    return this.readUint8(offset) << 24 >> 24;
  }
  readUint8(offset) {
    return this.bytes_[offset];
  }
  readInt16(offset) {
    return this.readUint16(offset) << 16 >> 16;
  }
  readUint16(offset) {
    return this.bytes_[offset] | this.bytes_[offset + 1] << 8;
  }
  readInt32(offset) {
    return this.bytes_[offset] | this.bytes_[offset + 1] << 8 | this.bytes_[offset + 2] << 16 | this.bytes_[offset + 3] << 24;
  }
  readUint32(offset) {
    return this.readInt32(offset) >>> 0;
  }
  readInt64(offset) {
    return BigInt.asIntN(64, BigInt(this.readUint32(offset)) + (BigInt(this.readUint32(offset + 4)) << BigInt(32)));
  }
  readUint64(offset) {
    return BigInt.asUintN(64, BigInt(this.readUint32(offset)) + (BigInt(this.readUint32(offset + 4)) << BigInt(32)));
  }
  readFloat32(offset) {
    int32[0] = this.readInt32(offset);
    return float32[0];
  }
  readFloat64(offset) {
    int32[isLittleEndian ? 0 : 1] = this.readInt32(offset);
    int32[isLittleEndian ? 1 : 0] = this.readInt32(offset + 4);
    return float64[0];
  }
  writeInt8(offset, value) {
    this.bytes_[offset] = value;
  }
  writeUint8(offset, value) {
    this.bytes_[offset] = value;
  }
  writeInt16(offset, value) {
    this.bytes_[offset] = value;
    this.bytes_[offset + 1] = value >> 8;
  }
  writeUint16(offset, value) {
    this.bytes_[offset] = value;
    this.bytes_[offset + 1] = value >> 8;
  }
  writeInt32(offset, value) {
    this.bytes_[offset] = value;
    this.bytes_[offset + 1] = value >> 8;
    this.bytes_[offset + 2] = value >> 16;
    this.bytes_[offset + 3] = value >> 24;
  }
  writeUint32(offset, value) {
    this.bytes_[offset] = value;
    this.bytes_[offset + 1] = value >> 8;
    this.bytes_[offset + 2] = value >> 16;
    this.bytes_[offset + 3] = value >> 24;
  }
  writeInt64(offset, value) {
    this.writeInt32(offset, Number(BigInt.asIntN(32, value)));
    this.writeInt32(offset + 4, Number(BigInt.asIntN(32, value >> BigInt(32))));
  }
  writeUint64(offset, value) {
    this.writeUint32(offset, Number(BigInt.asUintN(32, value)));
    this.writeUint32(offset + 4, Number(BigInt.asUintN(32, value >> BigInt(32))));
  }
  writeFloat32(offset, value) {
    float32[0] = value;
    this.writeInt32(offset, int32[0]);
  }
  writeFloat64(offset, value) {
    float64[0] = value;
    this.writeInt32(offset, int32[isLittleEndian ? 0 : 1]);
    this.writeInt32(offset + 4, int32[isLittleEndian ? 1 : 0]);
  }
  /**
   * Return the file identifier.   Behavior is undefined for FlatBuffers whose
   * schema does not include a file_identifier (likely points at padding or the
   * start of a the root vtable).
   */
  getBufferIdentifier() {
    if (this.bytes_.length < this.position_ + SIZEOF_INT + FILE_IDENTIFIER_LENGTH) {
      throw new Error("FlatBuffers: ByteBuffer is too short to contain an identifier.");
    }
    let result = "";
    for (let i = 0; i < FILE_IDENTIFIER_LENGTH; i++) {
      result += String.fromCharCode(this.readInt8(this.position_ + SIZEOF_INT + i));
    }
    return result;
  }
  /**
   * Look up a field in the vtable, return an offset into the object, or 0 if the
   * field is not present.
   */
  __offset(bb_pos, vtable_offset) {
    const vtable = bb_pos - this.readInt32(bb_pos);
    return vtable_offset < this.readInt16(vtable) ? this.readInt16(vtable + vtable_offset) : 0;
  }
  /**
   * Initialize any Table-derived type to point to the union at the given offset.
   */
  __union(t, offset) {
    t.bb_pos = offset + this.readInt32(offset);
    t.bb = this;
    return t;
  }
  /**
   * Create a JavaScript string from UTF-8 data stored inside the FlatBuffer.
   * This allocates a new string and converts to wide chars upon each access.
   *
   * To avoid the conversion to string, pass Encoding.UTF8_BYTES as the
   * "optionalEncoding" argument. This is useful for avoiding conversion when
   * the data will just be packaged back up in another FlatBuffer later on.
   *
   * @param offset
   * @param opt_encoding Defaults to UTF16_STRING
   */
  __string(offset, opt_encoding) {
    offset += this.readInt32(offset);
    const length = this.readInt32(offset);
    offset += SIZEOF_INT;
    const utf8bytes = this.bytes_.subarray(offset, offset + length);
    if (opt_encoding === Encoding.UTF8_BYTES)
      return utf8bytes;
    else
      return this.text_decoder_.decode(utf8bytes);
  }
  /**
   * Handle unions that can contain string as its member, if a Table-derived type then initialize it,
   * if a string then return a new one
   *
   * WARNING: strings are immutable in JS so we can't change the string that the user gave us, this
   * makes the behaviour of __union_with_string different compared to __union
   */
  __union_with_string(o, offset) {
    if (typeof o === "string") {
      return this.__string(offset);
    }
    return this.__union(o, offset);
  }
  /**
   * Retrieve the relative offset stored at "offset"
   */
  __indirect(offset) {
    return offset + this.readInt32(offset);
  }
  /**
   * Get the start of data of a vector whose offset is stored at "offset" in this object.
   */
  __vector(offset) {
    return offset + this.readInt32(offset) + SIZEOF_INT;
  }
  /**
   * Get the length of a vector whose offset is stored at "offset" in this object.
   */
  __vector_len(offset) {
    return this.readInt32(offset + this.readInt32(offset));
  }
  __has_identifier(ident) {
    if (ident.length != FILE_IDENTIFIER_LENGTH) {
      throw new Error("FlatBuffers: file identifier must be length " + FILE_IDENTIFIER_LENGTH);
    }
    for (let i = 0; i < FILE_IDENTIFIER_LENGTH; i++) {
      if (ident.charCodeAt(i) != this.readInt8(this.position() + SIZEOF_INT + i)) {
        return false;
      }
    }
    return true;
  }
  /**
   * A helper function for generating list for obj api
   */
  createScalarList(listAccessor, listLength) {
    const ret = [];
    for (let i = 0; i < listLength; ++i) {
      const val = listAccessor(i);
      if (val !== null) {
        ret.push(val);
      }
    }
    return ret;
  }
  /**
   * A helper function for generating list for obj api
   * @param listAccessor function that accepts an index and return data at that index
   * @param listLength listLength
   * @param res result list
   */
  createObjList(listAccessor, listLength) {
    const ret = [];
    for (let i = 0; i < listLength; ++i) {
      const val = listAccessor(i);
      if (val !== null) {
        ret.push(val.unpack());
      }
    }
    return ret;
  }
};

// node_modules/flatbuffers/mjs/builder.js
var Builder = class _Builder {
  /**
   * Create a FlatBufferBuilder.
   */
  constructor(opt_initial_size) {
    this.minalign = 1;
    this.vtable = null;
    this.vtable_in_use = 0;
    this.isNested = false;
    this.object_start = 0;
    this.vtables = [];
    this.vector_num_elems = 0;
    this.force_defaults = false;
    this.string_maps = null;
    this.text_encoder = new TextEncoder();
    let initial_size;
    if (!opt_initial_size) {
      initial_size = 1024;
    } else {
      initial_size = opt_initial_size;
    }
    this.bb = ByteBuffer.allocate(initial_size);
    this.space = initial_size;
  }
  clear() {
    this.bb.clear();
    this.space = this.bb.capacity();
    this.minalign = 1;
    this.vtable = null;
    this.vtable_in_use = 0;
    this.isNested = false;
    this.object_start = 0;
    this.vtables = [];
    this.vector_num_elems = 0;
    this.force_defaults = false;
    this.string_maps = null;
  }
  /**
   * In order to save space, fields that are set to their default value
   * don't get serialized into the buffer. Forcing defaults provides a
   * way to manually disable this optimization.
   *
   * @param forceDefaults true always serializes default values
   */
  forceDefaults(forceDefaults) {
    this.force_defaults = forceDefaults;
  }
  /**
   * Get the ByteBuffer representing the FlatBuffer. Only call this after you've
   * called finish(). The actual data starts at the ByteBuffer's current position,
   * not necessarily at 0.
   */
  dataBuffer() {
    return this.bb;
  }
  /**
   * Get the bytes representing the FlatBuffer. Only call this after you've
   * called finish().
   */
  asUint8Array() {
    return this.bb.bytes().subarray(this.bb.position(), this.bb.position() + this.offset());
  }
  /**
   * Prepare to write an element of `size` after `additional_bytes` have been
   * written, e.g. if you write a string, you need to align such the int length
   * field is aligned to 4 bytes, and the string data follows it directly. If all
   * you need to do is alignment, `additional_bytes` will be 0.
   *
   * @param size This is the of the new element to write
   * @param additional_bytes The padding size
   */
  prep(size, additional_bytes) {
    if (size > this.minalign) {
      this.minalign = size;
    }
    const align_size = ~(this.bb.capacity() - this.space + additional_bytes) + 1 & size - 1;
    while (this.space < align_size + size + additional_bytes) {
      const old_buf_size = this.bb.capacity();
      this.bb = _Builder.growByteBuffer(this.bb);
      this.space += this.bb.capacity() - old_buf_size;
    }
    this.pad(align_size);
  }
  pad(byte_size) {
    for (let i = 0; i < byte_size; i++) {
      this.bb.writeInt8(--this.space, 0);
    }
  }
  writeInt8(value) {
    this.bb.writeInt8(this.space -= 1, value);
  }
  writeInt16(value) {
    this.bb.writeInt16(this.space -= 2, value);
  }
  writeInt32(value) {
    this.bb.writeInt32(this.space -= 4, value);
  }
  writeInt64(value) {
    this.bb.writeInt64(this.space -= 8, value);
  }
  writeFloat32(value) {
    this.bb.writeFloat32(this.space -= 4, value);
  }
  writeFloat64(value) {
    this.bb.writeFloat64(this.space -= 8, value);
  }
  /**
   * Add an `int8` to the buffer, properly aligned, and grows the buffer (if necessary).
   * @param value The `int8` to add the buffer.
   */
  addInt8(value) {
    this.prep(1, 0);
    this.writeInt8(value);
  }
  /**
   * Add an `int16` to the buffer, properly aligned, and grows the buffer (if necessary).
   * @param value The `int16` to add the buffer.
   */
  addInt16(value) {
    this.prep(2, 0);
    this.writeInt16(value);
  }
  /**
   * Add an `int32` to the buffer, properly aligned, and grows the buffer (if necessary).
   * @param value The `int32` to add the buffer.
   */
  addInt32(value) {
    this.prep(4, 0);
    this.writeInt32(value);
  }
  /**
   * Add an `int64` to the buffer, properly aligned, and grows the buffer (if necessary).
   * @param value The `int64` to add the buffer.
   */
  addInt64(value) {
    this.prep(8, 0);
    this.writeInt64(value);
  }
  /**
   * Add a `float32` to the buffer, properly aligned, and grows the buffer (if necessary).
   * @param value The `float32` to add the buffer.
   */
  addFloat32(value) {
    this.prep(4, 0);
    this.writeFloat32(value);
  }
  /**
   * Add a `float64` to the buffer, properly aligned, and grows the buffer (if necessary).
   * @param value The `float64` to add the buffer.
   */
  addFloat64(value) {
    this.prep(8, 0);
    this.writeFloat64(value);
  }
  addFieldInt8(voffset, value, defaultValue) {
    if (this.force_defaults || value != defaultValue) {
      this.addInt8(value);
      this.slot(voffset);
    }
  }
  addFieldInt16(voffset, value, defaultValue) {
    if (this.force_defaults || value != defaultValue) {
      this.addInt16(value);
      this.slot(voffset);
    }
  }
  addFieldInt32(voffset, value, defaultValue) {
    if (this.force_defaults || value != defaultValue) {
      this.addInt32(value);
      this.slot(voffset);
    }
  }
  addFieldInt64(voffset, value, defaultValue) {
    if (this.force_defaults || value !== defaultValue) {
      this.addInt64(value);
      this.slot(voffset);
    }
  }
  addFieldFloat32(voffset, value, defaultValue) {
    if (this.force_defaults || value != defaultValue) {
      this.addFloat32(value);
      this.slot(voffset);
    }
  }
  addFieldFloat64(voffset, value, defaultValue) {
    if (this.force_defaults || value != defaultValue) {
      this.addFloat64(value);
      this.slot(voffset);
    }
  }
  addFieldOffset(voffset, value, defaultValue) {
    if (this.force_defaults || value != defaultValue) {
      this.addOffset(value);
      this.slot(voffset);
    }
  }
  /**
   * Structs are stored inline, so nothing additional is being added. `d` is always 0.
   */
  addFieldStruct(voffset, value, defaultValue) {
    if (value != defaultValue) {
      this.nested(value);
      this.slot(voffset);
    }
  }
  /**
   * Structures are always stored inline, they need to be created right
   * where they're used.  You'll get this assertion failure if you
   * created it elsewhere.
   */
  nested(obj) {
    if (obj != this.offset()) {
      throw new TypeError("FlatBuffers: struct must be serialized inline.");
    }
  }
  /**
   * Should not be creating any other object, string or vector
   * while an object is being constructed
   */
  notNested() {
    if (this.isNested) {
      throw new TypeError("FlatBuffers: object serialization must not be nested.");
    }
  }
  /**
   * Set the current vtable at `voffset` to the current location in the buffer.
   */
  slot(voffset) {
    if (this.vtable !== null)
      this.vtable[voffset] = this.offset();
  }
  /**
   * @returns Offset relative to the end of the buffer.
   */
  offset() {
    return this.bb.capacity() - this.space;
  }
  /**
   * Doubles the size of the backing ByteBuffer and copies the old data towards
   * the end of the new buffer (since we build the buffer backwards).
   *
   * @param bb The current buffer with the existing data
   * @returns A new byte buffer with the old data copied
   * to it. The data is located at the end of the buffer.
   *
   * uint8Array.set() formally takes {Array<number>|ArrayBufferView}, so to pass
   * it a uint8Array we need to suppress the type check:
   * @suppress {checkTypes}
   */
  static growByteBuffer(bb) {
    const old_buf_size = bb.capacity();
    if (old_buf_size & 3221225472) {
      throw new Error("FlatBuffers: cannot grow buffer beyond 2 gigabytes.");
    }
    const new_buf_size = old_buf_size << 1;
    const nbb = ByteBuffer.allocate(new_buf_size);
    nbb.setPosition(new_buf_size - old_buf_size);
    nbb.bytes().set(bb.bytes(), new_buf_size - old_buf_size);
    return nbb;
  }
  /**
   * Adds on offset, relative to where it will be written.
   *
   * @param offset The offset to add.
   */
  addOffset(offset) {
    this.prep(SIZEOF_INT, 0);
    this.writeInt32(this.offset() - offset + SIZEOF_INT);
  }
  /**
   * Start encoding a new object in the buffer.  Users will not usually need to
   * call this directly. The FlatBuffers compiler will generate helper methods
   * that call this method internally.
   */
  startObject(numfields) {
    this.notNested();
    if (this.vtable == null) {
      this.vtable = [];
    }
    this.vtable_in_use = numfields;
    for (let i = 0; i < numfields; i++) {
      this.vtable[i] = 0;
    }
    this.isNested = true;
    this.object_start = this.offset();
  }
  /**
   * Finish off writing the object that is under construction.
   *
   * @returns The offset to the object inside `dataBuffer`
   */
  endObject() {
    if (this.vtable == null || !this.isNested) {
      throw new Error("FlatBuffers: endObject called without startObject");
    }
    this.addInt32(0);
    const vtableloc = this.offset();
    let i = this.vtable_in_use - 1;
    for (; i >= 0 && this.vtable[i] == 0; i--) {
    }
    const trimmed_size = i + 1;
    for (; i >= 0; i--) {
      this.addInt16(this.vtable[i] != 0 ? vtableloc - this.vtable[i] : 0);
    }
    const standard_fields = 2;
    this.addInt16(vtableloc - this.object_start);
    const len = (trimmed_size + standard_fields) * SIZEOF_SHORT;
    this.addInt16(len);
    let existing_vtable = 0;
    const vt1 = this.space;
    outer_loop: for (i = 0; i < this.vtables.length; i++) {
      const vt2 = this.bb.capacity() - this.vtables[i];
      if (len == this.bb.readInt16(vt2)) {
        for (let j = SIZEOF_SHORT; j < len; j += SIZEOF_SHORT) {
          if (this.bb.readInt16(vt1 + j) != this.bb.readInt16(vt2 + j)) {
            continue outer_loop;
          }
        }
        existing_vtable = this.vtables[i];
        break;
      }
    }
    if (existing_vtable) {
      this.space = this.bb.capacity() - vtableloc;
      this.bb.writeInt32(this.space, existing_vtable - vtableloc);
    } else {
      this.vtables.push(this.offset());
      this.bb.writeInt32(this.bb.capacity() - vtableloc, this.offset() - vtableloc);
    }
    this.isNested = false;
    return vtableloc;
  }
  /**
   * Finalize a buffer, poiting to the given `root_table`.
   */
  finish(root_table, opt_file_identifier, opt_size_prefix) {
    const size_prefix = opt_size_prefix ? SIZE_PREFIX_LENGTH : 0;
    if (opt_file_identifier) {
      const file_identifier = opt_file_identifier;
      this.prep(this.minalign, SIZEOF_INT + FILE_IDENTIFIER_LENGTH + size_prefix);
      if (file_identifier.length != FILE_IDENTIFIER_LENGTH) {
        throw new TypeError("FlatBuffers: file identifier must be length " + FILE_IDENTIFIER_LENGTH);
      }
      for (let i = FILE_IDENTIFIER_LENGTH - 1; i >= 0; i--) {
        this.writeInt8(file_identifier.charCodeAt(i));
      }
    }
    this.prep(this.minalign, SIZEOF_INT + size_prefix);
    this.addOffset(root_table);
    if (size_prefix) {
      this.addInt32(this.bb.capacity() - this.space);
    }
    this.bb.setPosition(this.space);
  }
  /**
   * Finalize a size prefixed buffer, pointing to the given `root_table`.
   */
  finishSizePrefixed(root_table, opt_file_identifier) {
    this.finish(root_table, opt_file_identifier, true);
  }
  /**
   * This checks a required field has been set in a given table that has
   * just been constructed.
   */
  requiredField(table, field) {
    const table_start = this.bb.capacity() - table;
    const vtable_start = table_start - this.bb.readInt32(table_start);
    const ok = field < this.bb.readInt16(vtable_start) && this.bb.readInt16(vtable_start + field) != 0;
    if (!ok) {
      throw new TypeError("FlatBuffers: field " + field + " must be set");
    }
  }
  /**
   * Start a new array/vector of objects.  Users usually will not call
   * this directly. The FlatBuffers compiler will create a start/end
   * method for vector types in generated code.
   *
   * @param elem_size The size of each element in the array
   * @param num_elems The number of elements in the array
   * @param alignment The alignment of the array
   */
  startVector(elem_size, num_elems, alignment) {
    this.notNested();
    this.vector_num_elems = num_elems;
    this.prep(SIZEOF_INT, elem_size * num_elems);
    this.prep(alignment, elem_size * num_elems);
  }
  /**
   * Finish off the creation of an array and all its elements. The array must be
   * created with `startVector`.
   *
   * @returns The offset at which the newly created array
   * starts.
   */
  endVector() {
    this.writeInt32(this.vector_num_elems);
    return this.offset();
  }
  /**
   * Encode the string `s` in the buffer using UTF-8. If the string passed has
   * already been seen, we return the offset of the already written string
   *
   * @param s The string to encode
   * @return The offset in the buffer where the encoded string starts
   */
  createSharedString(s) {
    if (!s) {
      return 0;
    }
    if (!this.string_maps) {
      this.string_maps = /* @__PURE__ */ new Map();
    }
    if (this.string_maps.has(s)) {
      return this.string_maps.get(s);
    }
    const offset = this.createString(s);
    this.string_maps.set(s, offset);
    return offset;
  }
  /**
   * Encode the string `s` in the buffer using UTF-8. If a Uint8Array is passed
   * instead of a string, it is assumed to contain valid UTF-8 encoded data.
   *
   * @param s The string to encode
   * @return The offset in the buffer where the encoded string starts
   */
  createString(s) {
    if (s === null || s === void 0) {
      return 0;
    }
    let utf8;
    if (s instanceof Uint8Array) {
      utf8 = s;
    } else {
      utf8 = this.text_encoder.encode(s);
    }
    this.addInt8(0);
    this.startVector(1, utf8.length, 1);
    this.bb.setPosition(this.space -= utf8.length);
    this.bb.bytes().set(utf8, this.space);
    return this.endVector();
  }
  /**
   * Create a byte vector.
   *
   * @param v The bytes to add
   * @returns The offset in the buffer where the byte vector starts
   */
  createByteVector(v) {
    if (v === null || v === void 0) {
      return 0;
    }
    this.startVector(1, v.length, 1);
    this.bb.setPosition(this.space -= v.length);
    this.bb.bytes().set(v, this.space);
    return this.endVector();
  }
  /**
   * A helper function to pack an object
   *
   * @returns offset of obj
   */
  createObjectOffset(obj) {
    if (obj === null) {
      return 0;
    }
    if (typeof obj === "string") {
      return this.createString(obj);
    } else {
      return obj.pack(this);
    }
  }
  /**
   * A helper function to pack a list of object
   *
   * @returns list of offsets of each non null object
   */
  createObjectOffsetList(list) {
    const ret = [];
    for (let i = 0; i < list.length; ++i) {
      const val = list[i];
      if (val !== null) {
        ret.push(this.createObjectOffset(val));
      } else {
        throw new TypeError("FlatBuffers: Argument for createObjectOffsetList cannot contain null.");
      }
    }
    return ret;
  }
  createStructOffsetList(list, startFunc) {
    startFunc(this, list.length);
    this.createObjectOffsetList(list.slice().reverse());
    return this.endVector();
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/payload.ts
var Payload = class _Payload {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsPayload(bb, obj) {
    return (obj || new _Payload()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsPayload(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _Payload()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  contentType(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  bytes(index) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint8(this.bb.__vector(this.bb_pos + offset) + index) : 0;
  }
  bytesLength() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  bytesArray() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? new Uint8Array(this.bb.bytes().buffer, this.bb.bytes().byteOffset + this.bb.__vector(this.bb_pos + offset), this.bb.__vector_len(this.bb_pos + offset)) : null;
  }
  static startPayload(builder) {
    builder.startObject(2);
  }
  static addContentType(builder, contentTypeOffset) {
    builder.addFieldOffset(0, contentTypeOffset, 0);
  }
  static addBytes(builder, bytesOffset) {
    builder.addFieldOffset(1, bytesOffset, 0);
  }
  static createBytesVector(builder, data) {
    builder.startVector(1, data.length, 1);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addInt8(data[i]);
    }
    return builder.endVector();
  }
  static startBytesVector(builder, numElems) {
    builder.startVector(1, numElems, 1);
  }
  static endPayload(builder) {
    const offset = builder.endObject();
    builder.requiredField(offset, 4);
    return offset;
  }
  static createPayload(builder, contentTypeOffset, bytesOffset) {
    _Payload.startPayload(builder);
    _Payload.addContentType(builder, contentTypeOffset);
    _Payload.addBytes(builder, bytesOffset);
    return _Payload.endPayload(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/add-event-listener-command.ts
var AddEventListenerCommand = class _AddEventListenerCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAddEventListenerCommand(bb, obj) {
    return (obj || new _AddEventListenerCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAddEventListenerCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AddEventListenerCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  callback() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startAddEventListenerCommand(builder) {
    builder.startObject(4);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(1, nameOffset, 0);
  }
  static addCallback(builder, callback) {
    builder.addFieldInt32(2, callback, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(3, optionsOffset, 0);
  }
  static endAddEventListenerCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/append-element-command.ts
var AppendElementCommand = class _AppendElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAppendElementCommand(bb, obj) {
    return (obj || new _AppendElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAppendElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AppendElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parent() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startAppendElementCommand(builder) {
    builder.startObject(2);
  }
  static addParent(builder, parent) {
    builder.addFieldInt32(0, parent, 0);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(1, current, 0);
  }
  static endAppendElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createAppendElementCommand(builder, parent, current) {
    _AppendElementCommand.startAppendElementCommand(builder);
    _AppendElementCommand.addParent(builder, parent);
    _AppendElementCommand.addCurrent(builder, current);
    return _AppendElementCommand.endAppendElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/boolean-result.ts
var BooleanResult = class _BooleanResult {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsBooleanResult(bb, obj) {
    return (obj || new _BooleanResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsBooleanResult(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _BooleanResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  value() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? !!this.bb.readInt8(this.bb_pos + offset) : false;
  }
  static startBooleanResult(builder) {
    builder.startObject(1);
  }
  static addValue(builder, value) {
    builder.addFieldInt8(0, +value, 0);
  }
  static endBooleanResult(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createBooleanResult(builder, value) {
    _BooleanResult.startBooleanResult(builder);
    _BooleanResult.addValue(builder, value);
    return _BooleanResult.endBooleanResult(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/add-class-command.ts
var AddClassCommand = class _AddClassCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAddClassCommand(bb, obj) {
    return (obj || new _AddClassCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAddClassCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AddClassCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  className(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startAddClassCommand(builder) {
    builder.startObject(2);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static addClassName(builder, classNameOffset) {
    builder.addFieldOffset(1, classNameOffset, 0);
  }
  static endAddClassCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createAddClassCommand(builder, current, classNameOffset) {
    _AddClassCommand.startAddClassCommand(builder);
    _AddClassCommand.addCurrent(builder, current);
    _AddClassCommand.addClassName(builder, classNameOffset);
    return _AddClassCommand.endAddClassCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/add-config-command.ts
var AddConfigCommand = class _AddConfigCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAddConfigCommand(bb, obj) {
    return (obj || new _AddConfigCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAddConfigCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AddConfigCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  key(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startAddConfigCommand(builder) {
    builder.startObject(3);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static addKey(builder, keyOffset) {
    builder.addFieldOffset(1, keyOffset, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(2, valueOffset, 0);
  }
  static endAddConfigCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/add-dataset-command.ts
var AddDatasetCommand = class _AddDatasetCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAddDatasetCommand(bb, obj) {
    return (obj || new _AddDatasetCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAddDatasetCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AddDatasetCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  key(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startAddDatasetCommand(builder) {
    builder.startObject(3);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addKey(builder, keyOffset) {
    builder.addFieldOffset(1, keyOffset, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(2, valueOffset, 0);
  }
  static endAddDatasetCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/add-event-command.ts
var AddEventCommand = class _AddEventCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAddEventCommand(bb, obj) {
    return (obj || new _AddEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAddEventCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AddEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  valueType(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  func() {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startAddEventCommand(builder) {
    builder.startObject(4);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addValueType(builder, valueTypeOffset) {
    builder.addFieldOffset(1, valueTypeOffset, 0);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(2, nameOffset, 0);
  }
  static addFunc(builder, func) {
    builder.addFieldInt32(3, func, 0);
  }
  static endAddEventCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createAddEventCommand(builder, node, valueTypeOffset, nameOffset, func) {
    _AddEventCommand.startAddEventCommand(builder);
    _AddEventCommand.addNode(builder, node);
    _AddEventCommand.addValueType(builder, valueTypeOffset);
    _AddEventCommand.addName(builder, nameOffset);
    _AddEventCommand.addFunc(builder, func);
    return _AddEventCommand.endAddEventCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/add-inline-style-command.ts
var AddInlineStyleCommand = class _AddInlineStyleCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAddInlineStyleCommand(bb, obj) {
    return (obj || new _AddInlineStyleCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAddInlineStyleCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AddInlineStyleCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  e() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  key(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startAddInlineStyleCommand(builder) {
    builder.startObject(3);
  }
  static addE(builder, e) {
    builder.addFieldInt32(0, e, 0);
  }
  static addKey(builder, keyOffset) {
    builder.addFieldOffset(1, keyOffset, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(2, valueOffset, 0);
  }
  static endAddInlineStyleCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/add-timing-listener-command.ts
var AddTimingListenerCommand = class _AddTimingListenerCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAddTimingListenerCommand(bb, obj) {
    return (obj || new _AddTimingListenerCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAddTimingListenerCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AddTimingListenerCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static startAddTimingListenerCommand(builder) {
    builder.startObject(0);
  }
  static endAddTimingListenerCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createAddTimingListenerCommand(builder) {
    _AddTimingListenerCommand.startAddTimingListenerCommand(builder);
    return _AddTimingListenerCommand.endAddTimingListenerCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/async-resolve-element-command.ts
var AsyncResolveElementCommand = class _AsyncResolveElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAsyncResolveElementCommand(bb, obj) {
    return (obj || new _AsyncResolveElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAsyncResolveElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AsyncResolveElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  element() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startAsyncResolveElementCommand(builder) {
    builder.startObject(1);
  }
  static addElement(builder, element) {
    builder.addFieldInt32(0, element, 0);
  }
  static endAsyncResolveElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createAsyncResolveElementCommand(builder, element) {
    _AsyncResolveElementCommand.startAsyncResolveElementCommand(builder);
    _AsyncResolveElementCommand.addElement(builder, element);
    return _AsyncResolveElementCommand.endAsyncResolveElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/async-resolve-subtree-command.ts
var AsyncResolveSubtreeCommand = class _AsyncResolveSubtreeCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsAsyncResolveSubtreeCommand(bb, obj) {
    return (obj || new _AsyncResolveSubtreeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsAsyncResolveSubtreeCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _AsyncResolveSubtreeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startAsyncResolveSubtreeCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endAsyncResolveSubtreeCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createAsyncResolveSubtreeCommand(builder, node) {
    _AsyncResolveSubtreeCommand.startAsyncResolveSubtreeCommand(builder);
    _AsyncResolveSubtreeCommand.addNode(builder, node);
    return _AsyncResolveSubtreeCommand.endAsyncResolveSubtreeCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/bind-pipeline-idwith-timing-flag-command.ts
var BindPipelineIDWithTimingFlagCommand = class _BindPipelineIDWithTimingFlagCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsBindPipelineIDWithTimingFlagCommand(bb, obj) {
    return (obj || new _BindPipelineIDWithTimingFlagCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsBindPipelineIDWithTimingFlagCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _BindPipelineIDWithTimingFlagCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  pipeLineId(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  timingFlag(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startBindPipelineIDWithTimingFlagCommand(builder) {
    builder.startObject(2);
  }
  static addPipeLineId(builder, pipeLineIdOffset) {
    builder.addFieldOffset(0, pipeLineIdOffset, 0);
  }
  static addTimingFlag(builder, timingFlagOffset) {
    builder.addFieldOffset(1, timingFlagOffset, 0);
  }
  static endBindPipelineIDWithTimingFlagCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createBindPipelineIDWithTimingFlagCommand(builder, pipeLineIdOffset, timingFlagOffset) {
    _BindPipelineIDWithTimingFlagCommand.startBindPipelineIDWithTimingFlagCommand(builder);
    _BindPipelineIDWithTimingFlagCommand.addPipeLineId(builder, pipeLineIdOffset);
    _BindPipelineIDWithTimingFlagCommand.addTimingFlag(builder, timingFlagOffset);
    return _BindPipelineIDWithTimingFlagCommand.endBindPipelineIDWithTimingFlagCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/clone-element-command.ts
var CloneElementCommand = class _CloneElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCloneElementCommand(bb, obj) {
    return (obj || new _CloneElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCloneElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CloneElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCloneElementCommand(builder) {
    builder.startObject(2);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(1, optionsOffset, 0);
  }
  static endCloneElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/consume-gesture-command.ts
var ConsumeGestureCommand = class _ConsumeGestureCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsConsumeGestureCommand(bb, obj) {
    return (obj || new _ConsumeGestureCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsConsumeGestureCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ConsumeGestureCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  gestureId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startConsumeGestureCommand(builder) {
    builder.startObject(3);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addGestureId(builder, gestureId) {
    builder.addFieldInt32(1, gestureId, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(2, optionsOffset, 0);
  }
  static endConsumeGestureCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-block-command.ts
var CreateBlockCommand = class _CreateBlockCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateBlockCommand(bb, obj) {
    return (obj || new _CreateBlockCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateBlockCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateBlockCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateBlockCommand(builder) {
    builder.startObject(2);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateBlockCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-component-command.ts
var CreateComponentCommand = class _CreateComponentCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateComponentCommand(bb, obj) {
    return (obj || new _CreateComponentCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateComponentCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateComponentCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  componentId(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  cssId() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  entryName(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  path(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 14);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  config(obj) {
    const offset = this.bb.__offset(this.bb_pos, 16);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 18);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateComponentCommand(builder) {
    builder.startObject(8);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addComponentId(builder, componentIdOffset) {
    builder.addFieldOffset(1, componentIdOffset, 0);
  }
  static addCssId(builder, cssId) {
    builder.addFieldInt32(2, cssId, 0);
  }
  static addEntryName(builder, entryNameOffset) {
    builder.addFieldOffset(3, entryNameOffset, 0);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(4, nameOffset, 0);
  }
  static addPath(builder, pathOffset) {
    builder.addFieldOffset(5, pathOffset, 0);
  }
  static addConfig(builder, configOffset) {
    builder.addFieldOffset(6, configOffset, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(7, infoOffset, 0);
  }
  static endCreateComponentCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-element-command.ts
var CreateElementCommand = class _CreateElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateElementCommand(bb, obj) {
    return (obj || new _CreateElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  tag(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  comParentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateElementCommand(builder) {
    builder.startObject(3);
  }
  static addTag(builder, tagOffset) {
    builder.addFieldOffset(0, tagOffset, 0);
  }
  static addComParentUniId(builder, comParentUniId) {
    builder.addFieldInt32(1, comParentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(2, infoOffset, 0);
  }
  static endCreateElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/element-references.ts
var ElementReferences = class _ElementReferences {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsElementReferences(bb, obj) {
    return (obj || new _ElementReferences()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsElementReferences(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ElementReferences()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  cardinality() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint8(this.bb_pos + offset) : 0 /* NONE */;
  }
  one() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  many(index) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb.__vector(this.bb_pos + offset) + index * 4) : 0;
  }
  manyLength() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  manyArray() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? new Uint32Array(this.bb.bytes().buffer, this.bb.bytes().byteOffset + this.bb.__vector(this.bb_pos + offset), this.bb.__vector_len(this.bb_pos + offset)) : null;
  }
  static startElementReferences(builder) {
    builder.startObject(3);
  }
  static addCardinality(builder, cardinality) {
    builder.addFieldInt8(0, cardinality, 0 /* NONE */);
  }
  static addOne(builder, one) {
    builder.addFieldInt32(1, one, 0);
  }
  static addMany(builder, manyOffset) {
    builder.addFieldOffset(2, manyOffset, 0);
  }
  static createManyVector(builder, data) {
    builder.startVector(4, data.length, 4);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addInt32(data[i]);
    }
    return builder.endVector();
  }
  static startManyVector(builder, numElems) {
    builder.startVector(4, numElems, 4);
  }
  static endElementReferences(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createElementReferences(builder, cardinality, one, manyOffset) {
    _ElementReferences.startElementReferences(builder);
    _ElementReferences.addCardinality(builder, cardinality);
    _ElementReferences.addOne(builder, one);
    _ElementReferences.addMany(builder, manyOffset);
    return _ElementReferences.endElementReferences(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-element-template-command.ts
var CreateElementTemplateCommand = class _CreateElementTemplateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateElementTemplateCommand(bb, obj) {
    return (obj || new _CreateElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateElementTemplateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  templateKey(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  bundleUrl(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  attributeSlots(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  elementSlots(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new ElementReferences()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  uid(obj) {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 14);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateElementTemplateCommand(builder) {
    builder.startObject(6);
  }
  static addTemplateKey(builder, templateKeyOffset) {
    builder.addFieldOffset(0, templateKeyOffset, 0);
  }
  static addBundleUrl(builder, bundleUrlOffset) {
    builder.addFieldOffset(1, bundleUrlOffset, 0);
  }
  static addAttributeSlots(builder, attributeSlotsOffset) {
    builder.addFieldOffset(2, attributeSlotsOffset, 0);
  }
  static addElementSlots(builder, elementSlotsOffset) {
    builder.addFieldOffset(3, elementSlotsOffset, 0);
  }
  static addUid(builder, uidOffset) {
    builder.addFieldOffset(4, uidOffset, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(5, optionsOffset, 0);
  }
  static endCreateElementTemplateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-event-command.ts
var CreateEventCommand = class _CreateEventCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateEventCommand(bb, obj) {
    return (obj || new _CreateEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateEventCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  valueType(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  detail(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateEventCommand(builder) {
    builder.startObject(4);
  }
  static addValueType(builder, valueTypeOffset) {
    builder.addFieldOffset(0, valueTypeOffset, 0);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(1, nameOffset, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(2, optionsOffset, 0);
  }
  static addDetail(builder, detailOffset) {
    builder.addFieldOffset(3, detailOffset, 0);
  }
  static endCreateEventCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-for-command.ts
var CreateForCommand = class _CreateForCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateForCommand(bb, obj) {
    return (obj || new _CreateForCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateForCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateForCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateForCommand(builder) {
    builder.startObject(2);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateForCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-frame-command.ts
var CreateFrameCommand = class _CreateFrameCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateFrameCommand(bb, obj) {
    return (obj || new _CreateFrameCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateFrameCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateFrameCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  comParentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateFrameCommand(builder) {
    builder.startObject(2);
  }
  static addComParentUniId(builder, comParentUniId) {
    builder.addFieldInt32(0, comParentUniId, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(1, optionsOffset, 0);
  }
  static endCreateFrameCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-gesture-detector-command.ts
var CreateGestureDetectorCommand = class _CreateGestureDetectorCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateGestureDetectorCommand(bb, obj) {
    return (obj || new _CreateGestureDetectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateGestureDetectorCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateGestureDetectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  gestureId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  gestureType() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readFloat64(this.bb_pos + offset) : 0;
  }
  config(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  relationMap(index) {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.readFloat64(this.bb.__vector(this.bb_pos + offset) + index * 8) : 0;
  }
  relationMapLength() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  relationMapArray() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? new Float64Array(this.bb.bytes().buffer, this.bb.bytes().byteOffset + this.bb.__vector(this.bb_pos + offset), this.bb.__vector_len(this.bb_pos + offset)) : null;
  }
  static startCreateGestureDetectorCommand(builder) {
    builder.startObject(5);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addGestureId(builder, gestureId) {
    builder.addFieldInt32(1, gestureId, 0);
  }
  static addGestureType(builder, gestureType) {
    builder.addFieldFloat64(2, gestureType, 0);
  }
  static addConfig(builder, configOffset) {
    builder.addFieldOffset(3, configOffset, 0);
  }
  static addRelationMap(builder, relationMapOffset) {
    builder.addFieldOffset(4, relationMapOffset, 0);
  }
  static createRelationMapVector(builder, data) {
    builder.startVector(8, data.length, 8);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addFloat64(data[i]);
    }
    return builder.endVector();
  }
  static startRelationMapVector(builder, numElems) {
    builder.startVector(8, numElems, 8);
  }
  static endCreateGestureDetectorCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-if-command.ts
var CreateIfCommand = class _CreateIfCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateIfCommand(bb, obj) {
    return (obj || new _CreateIfCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateIfCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateIfCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateIfCommand(builder) {
    builder.startObject(2);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateIfCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-image-command.ts
var CreateImageCommand = class _CreateImageCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateImageCommand(bb, obj) {
    return (obj || new _CreateImageCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateImageCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateImageCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateImageCommand(builder) {
    builder.startObject(2);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateImageCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-list-command.ts
var CreateListCommand = class _CreateListCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateListCommand(bb, obj) {
    return (obj || new _CreateListCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateListCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateListCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  componentAtIndex() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  enqueueComponent() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  componentAtIndexes() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : null;
  }
  static startCreateListCommand(builder) {
    builder.startObject(5);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addComponentAtIndex(builder, componentAtIndex) {
    builder.addFieldInt32(1, componentAtIndex, 0);
  }
  static addEnqueueComponent(builder, enqueueComponent) {
    builder.addFieldInt32(2, enqueueComponent, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(3, infoOffset, 0);
  }
  static addComponentAtIndexes(builder, componentAtIndexes) {
    builder.addFieldInt32(4, componentAtIndexes, null);
  }
  static endCreateListCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-non-element-command.ts
var CreateNonElementCommand = class _CreateNonElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateNonElementCommand(bb, obj) {
    return (obj || new _CreateNonElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateNonElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateNonElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startCreateNonElementCommand(builder) {
    builder.startObject(1);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static endCreateNonElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createCreateNonElementCommand(builder, parentComponentUniId) {
    _CreateNonElementCommand.startCreateNonElementCommand(builder);
    _CreateNonElementCommand.addParentComponentUniId(builder, parentComponentUniId);
    return _CreateNonElementCommand.endCreateNonElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-page-command.ts
var CreatePageCommand = class _CreatePageCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreatePageCommand(bb, obj) {
    return (obj || new _CreatePageCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreatePageCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreatePageCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  componentId(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  cssId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreatePageCommand(builder) {
    builder.startObject(3);
  }
  static addComponentId(builder, componentIdOffset) {
    builder.addFieldOffset(0, componentIdOffset, 0);
  }
  static addCssId(builder, cssId) {
    builder.addFieldInt32(1, cssId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(2, infoOffset, 0);
  }
  static endCreatePageCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-raw-text-command.ts
var CreateRawTextCommand = class _CreateRawTextCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateRawTextCommand(bb, obj) {
    return (obj || new _CreateRawTextCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateRawTextCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateRawTextCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  text(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateRawTextCommand(builder) {
    builder.startObject(2);
  }
  static addText(builder, textOffset) {
    builder.addFieldOffset(0, textOffset, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateRawTextCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-scroll-view-command.ts
var CreateScrollViewCommand = class _CreateScrollViewCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateScrollViewCommand(bb, obj) {
    return (obj || new _CreateScrollViewCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateScrollViewCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateScrollViewCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateScrollViewCommand(builder) {
    builder.startObject(2);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateScrollViewCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-style-object-command.ts
var CreateStyleObjectCommand = class _CreateStyleObjectCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateStyleObjectCommand(bb, obj) {
    return (obj || new _CreateStyleObjectCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateStyleObjectCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateStyleObjectCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  styleObject(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateStyleObjectCommand(builder) {
    builder.startObject(1);
  }
  static addStyleObject(builder, styleObjectOffset) {
    builder.addFieldOffset(0, styleObjectOffset, 0);
  }
  static endCreateStyleObjectCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createCreateStyleObjectCommand(builder, styleObjectOffset) {
    _CreateStyleObjectCommand.startCreateStyleObjectCommand(builder);
    _CreateStyleObjectCommand.addStyleObject(builder, styleObjectOffset);
    return _CreateStyleObjectCommand.endCreateStyleObjectCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-text-command.ts
var CreateTextCommand = class _CreateTextCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateTextCommand(bb, obj) {
    return (obj || new _CreateTextCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateTextCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateTextCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateTextCommand(builder) {
    builder.startObject(2);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateTextCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-typed-element-template-command.ts
var CreateTypedElementTemplateCommand = class _CreateTypedElementTemplateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateTypedElementTemplateCommand(bb, obj) {
    return (obj || new _CreateTypedElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateTypedElementTemplateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateTypedElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  tag(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  attributes(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  elementSlots(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new ElementReferences()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  uid(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateTypedElementTemplateCommand(builder) {
    builder.startObject(5);
  }
  static addTag(builder, tagOffset) {
    builder.addFieldOffset(0, tagOffset, 0);
  }
  static addAttributes(builder, attributesOffset) {
    builder.addFieldOffset(1, attributesOffset, 0);
  }
  static addElementSlots(builder, elementSlotsOffset) {
    builder.addFieldOffset(2, elementSlotsOffset, 0);
  }
  static addUid(builder, uidOffset) {
    builder.addFieldOffset(3, uidOffset, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(4, optionsOffset, 0);
  }
  static endCreateTypedElementTemplateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-view-command.ts
var CreateViewCommand = class _CreateViewCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateViewCommand(bb, obj) {
    return (obj || new _CreateViewCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateViewCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateViewCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startCreateViewCommand(builder) {
    builder.startObject(2);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endCreateViewCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/create-wrapper-element-command.ts
var CreateWrapperElementCommand = class _CreateWrapperElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCreateWrapperElementCommand(bb, obj) {
    return (obj || new _CreateWrapperElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCreateWrapperElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CreateWrapperElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startCreateWrapperElementCommand(builder) {
    builder.startObject(1);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(0, parentComponentUniId, 0);
  }
  static endCreateWrapperElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createCreateWrapperElementCommand(builder, parentComponentUniId) {
    _CreateWrapperElementCommand.startCreateWrapperElementCommand(builder);
    _CreateWrapperElementCommand.addParentComponentUniId(builder, parentComponentUniId);
    return _CreateWrapperElementCommand.endCreateWrapperElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/dispatch-event-command.ts
var DispatchEventCommand = class _DispatchEventCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsDispatchEventCommand(bb, obj) {
    return (obj || new _DispatchEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsDispatchEventCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _DispatchEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  event(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startDispatchEventCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addEvent(builder, eventOffset) {
    builder.addFieldOffset(1, eventOffset, 0);
  }
  static endDispatchEventCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/element-animate-command.ts
var ElementAnimateCommand = class _ElementAnimateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsElementAnimateCommand(bb, obj) {
    return (obj || new _ElementAnimateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsElementAnimateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ElementAnimateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  element() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  args(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startElementAnimateCommand(builder) {
    builder.startObject(2);
  }
  static addElement(builder, element) {
    builder.addFieldInt32(0, element, 0);
  }
  static addArgs(builder, argsOffset) {
    builder.addFieldOffset(1, argsOffset, 0);
  }
  static endElementAnimateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/element-from-binary-command.ts
var ElementFromBinaryCommand = class _ElementFromBinaryCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsElementFromBinaryCommand(bb, obj) {
    return (obj || new _ElementFromBinaryCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsElementFromBinaryCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ElementFromBinaryCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  elementTemplateKey(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  parentComponentUniId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startElementFromBinaryCommand(builder) {
    builder.startObject(2);
  }
  static addElementTemplateKey(builder, elementTemplateKeyOffset) {
    builder.addFieldOffset(0, elementTemplateKeyOffset, 0);
  }
  static addParentComponentUniId(builder, parentComponentUniId) {
    builder.addFieldInt32(1, parentComponentUniId, 0);
  }
  static endElementFromBinaryCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createElementFromBinaryCommand(builder, elementTemplateKeyOffset, parentComponentUniId) {
    _ElementFromBinaryCommand.startElementFromBinaryCommand(builder);
    _ElementFromBinaryCommand.addElementTemplateKey(builder, elementTemplateKeyOffset);
    _ElementFromBinaryCommand.addParentComponentUniId(builder, parentComponentUniId);
    return _ElementFromBinaryCommand.endElementFromBinaryCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/element-is-equal-command.ts
var ElementIsEqualCommand = class _ElementIsEqualCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsElementIsEqualCommand(bb, obj) {
    return (obj || new _ElementIsEqualCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsElementIsEqualCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ElementIsEqualCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  left() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  right() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startElementIsEqualCommand(builder) {
    builder.startObject(2);
  }
  static addLeft(builder, left) {
    builder.addFieldInt32(0, left, 0);
  }
  static addRight(builder, right) {
    builder.addFieldInt32(1, right, 0);
  }
  static endElementIsEqualCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createElementIsEqualCommand(builder, left, right) {
    _ElementIsEqualCommand.startElementIsEqualCommand(builder);
    _ElementIsEqualCommand.addLeft(builder, left);
    _ElementIsEqualCommand.addRight(builder, right);
    return _ElementIsEqualCommand.endElementIsEqualCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/first-element-command.ts
var FirstElementCommand = class _FirstElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsFirstElementCommand(bb, obj) {
    return (obj || new _FirstElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsFirstElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _FirstElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startFirstElementCommand(builder) {
    builder.startObject(1);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static endFirstElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createFirstElementCommand(builder, current) {
    _FirstElementCommand.startFirstElementCommand(builder);
    _FirstElementCommand.addCurrent(builder, current);
    return _FirstElementCommand.endFirstElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/flush-element-tree-command.ts
var FlushElementTreeCommand = class _FlushElementTreeCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsFlushElementTreeCommand(bb, obj) {
    return (obj || new _FlushElementTreeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsFlushElementTreeCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _FlushElementTreeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  element() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : null;
  }
  options() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : null;
  }
  static startFlushElementTreeCommand(builder) {
    builder.startObject(2);
  }
  static addElement(builder, element) {
    builder.addFieldInt32(0, element, null);
  }
  static addOptions(builder, options) {
    builder.addFieldInt32(1, options, null);
  }
  static endFlushElementTreeCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createFlushElementTreeCommand(builder, element, options) {
    _FlushElementTreeCommand.startFlushElementTreeCommand(builder);
    if (element !== null)
      _FlushElementTreeCommand.addElement(builder, element);
    if (options !== null)
      _FlushElementTreeCommand.addOptions(builder, options);
    return _FlushElementTreeCommand.endFlushElementTreeCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/generate-pipeline-options-command.ts
var GeneratePipelineOptionsCommand = class _GeneratePipelineOptionsCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGeneratePipelineOptionsCommand(bb, obj) {
    return (obj || new _GeneratePipelineOptionsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGeneratePipelineOptionsCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GeneratePipelineOptionsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static startGeneratePipelineOptionsCommand(builder) {
    builder.startObject(0);
  }
  static endGeneratePipelineOptionsCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGeneratePipelineOptionsCommand(builder) {
    _GeneratePipelineOptionsCommand.startGeneratePipelineOptionsCommand(builder);
    return _GeneratePipelineOptionsCommand.endGeneratePipelineOptionsCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-attribute-by-name-command.ts
var GetAttributeByNameCommand = class _GetAttributeByNameCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetAttributeByNameCommand(bb, obj) {
    return (obj || new _GetAttributeByNameCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetAttributeByNameCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetAttributeByNameCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  e() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startGetAttributeByNameCommand(builder) {
    builder.startObject(2);
  }
  static addE(builder, e) {
    builder.addFieldInt32(0, e, 0);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(1, nameOffset, 0);
  }
  static endGetAttributeByNameCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetAttributeByNameCommand(builder, e, nameOffset) {
    _GetAttributeByNameCommand.startGetAttributeByNameCommand(builder);
    _GetAttributeByNameCommand.addE(builder, e);
    _GetAttributeByNameCommand.addName(builder, nameOffset);
    return _GetAttributeByNameCommand.endGetAttributeByNameCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-attribute-names-command.ts
var GetAttributeNamesCommand = class _GetAttributeNamesCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetAttributeNamesCommand(bb, obj) {
    return (obj || new _GetAttributeNamesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetAttributeNamesCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetAttributeNamesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  e() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetAttributeNamesCommand(builder) {
    builder.startObject(1);
  }
  static addE(builder, e) {
    builder.addFieldInt32(0, e, 0);
  }
  static endGetAttributeNamesCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetAttributeNamesCommand(builder, e) {
    _GetAttributeNamesCommand.startGetAttributeNamesCommand(builder);
    _GetAttributeNamesCommand.addE(builder, e);
    return _GetAttributeNamesCommand.endGetAttributeNamesCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-attributes-command.ts
var GetAttributesCommand = class _GetAttributesCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetAttributesCommand(bb, obj) {
    return (obj || new _GetAttributesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetAttributesCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetAttributesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  e() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetAttributesCommand(builder) {
    builder.startObject(1);
  }
  static addE(builder, e) {
    builder.addFieldInt32(0, e, 0);
  }
  static endGetAttributesCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetAttributesCommand(builder, e) {
    _GetAttributesCommand.startGetAttributesCommand(builder);
    _GetAttributesCommand.addE(builder, e);
    return _GetAttributesCommand.endGetAttributesCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-children-command.ts
var GetChildrenCommand = class _GetChildrenCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetChildrenCommand(bb, obj) {
    return (obj || new _GetChildrenCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetChildrenCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetChildrenCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetChildrenCommand(builder) {
    builder.startObject(1);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static endGetChildrenCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetChildrenCommand(builder, current) {
    _GetChildrenCommand.startGetChildrenCommand(builder);
    _GetChildrenCommand.addCurrent(builder, current);
    return _GetChildrenCommand.endGetChildrenCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-classes-command.ts
var GetClassesCommand = class _GetClassesCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetClassesCommand(bb, obj) {
    return (obj || new _GetClassesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetClassesCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetClassesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetClassesCommand(builder) {
    builder.startObject(1);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static endGetClassesCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetClassesCommand(builder, current) {
    _GetClassesCommand.startGetClassesCommand(builder);
    _GetClassesCommand.addCurrent(builder, current);
    return _GetClassesCommand.endGetClassesCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-component-idcommand.ts
var GetComponentIDCommand = class _GetComponentIDCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetComponentIDCommand(bb, obj) {
    return (obj || new _GetComponentIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetComponentIDCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetComponentIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetComponentIDCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endGetComponentIDCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetComponentIDCommand(builder, node) {
    _GetComponentIDCommand.startGetComponentIDCommand(builder);
    _GetComponentIDCommand.addNode(builder, node);
    return _GetComponentIDCommand.endGetComponentIDCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-config-command.ts
var GetConfigCommand = class _GetConfigCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetConfigCommand(bb, obj) {
    return (obj || new _GetConfigCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetConfigCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetConfigCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetConfigCommand(builder) {
    builder.startObject(1);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static endGetConfigCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetConfigCommand(builder, ele) {
    _GetConfigCommand.startGetConfigCommand(builder);
    _GetConfigCommand.addEle(builder, ele);
    return _GetConfigCommand.endGetConfigCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-data-by-key-command.ts
var GetDataByKeyCommand = class _GetDataByKeyCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetDataByKeyCommand(bb, obj) {
    return (obj || new _GetDataByKeyCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetDataByKeyCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetDataByKeyCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  key(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startGetDataByKeyCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addKey(builder, keyOffset) {
    builder.addFieldOffset(1, keyOffset, 0);
  }
  static endGetDataByKeyCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetDataByKeyCommand(builder, node, keyOffset) {
    _GetDataByKeyCommand.startGetDataByKeyCommand(builder);
    _GetDataByKeyCommand.addNode(builder, node);
    _GetDataByKeyCommand.addKey(builder, keyOffset);
    return _GetDataByKeyCommand.endGetDataByKeyCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-dataset-command.ts
var GetDatasetCommand = class _GetDatasetCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetDatasetCommand(bb, obj) {
    return (obj || new _GetDatasetCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetDatasetCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetDatasetCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetDatasetCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endGetDatasetCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetDatasetCommand(builder, node) {
    _GetDatasetCommand.startGetDatasetCommand(builder);
    _GetDatasetCommand.addNode(builder, node);
    return _GetDatasetCommand.endGetDatasetCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-element-by-unique-idcommand.ts
var GetElementByUniqueIDCommand = class _GetElementByUniqueIDCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetElementByUniqueIDCommand(bb, obj) {
    return (obj || new _GetElementByUniqueIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetElementByUniqueIDCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetElementByUniqueIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  elementId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetElementByUniqueIDCommand(builder) {
    builder.startObject(1);
  }
  static addElementId(builder, elementId) {
    builder.addFieldInt32(0, elementId, 0);
  }
  static endGetElementByUniqueIDCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetElementByUniqueIDCommand(builder, elementId) {
    _GetElementByUniqueIDCommand.startGetElementByUniqueIDCommand(builder);
    _GetElementByUniqueIDCommand.addElementId(builder, elementId);
    return _GetElementByUniqueIDCommand.endGetElementByUniqueIDCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-element-unique-idcommand.ts
var GetElementUniqueIDCommand = class _GetElementUniqueIDCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetElementUniqueIDCommand(bb, obj) {
    return (obj || new _GetElementUniqueIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetElementUniqueIDCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetElementUniqueIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetElementUniqueIDCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endGetElementUniqueIDCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetElementUniqueIDCommand(builder, node) {
    _GetElementUniqueIDCommand.startGetElementUniqueIDCommand(builder);
    _GetElementUniqueIDCommand.addNode(builder, node);
    return _GetElementUniqueIDCommand.endGetElementUniqueIDCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-event-command.ts
var GetEventCommand = class _GetEventCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetEventCommand(bb, obj) {
    return (obj || new _GetEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetEventCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  valueType(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startGetEventCommand(builder) {
    builder.startObject(3);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(1, nameOffset, 0);
  }
  static addValueType(builder, valueTypeOffset) {
    builder.addFieldOffset(2, valueTypeOffset, 0);
  }
  static endGetEventCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetEventCommand(builder, node, nameOffset, valueTypeOffset) {
    _GetEventCommand.startGetEventCommand(builder);
    _GetEventCommand.addNode(builder, node);
    _GetEventCommand.addName(builder, nameOffset);
    _GetEventCommand.addValueType(builder, valueTypeOffset);
    return _GetEventCommand.endGetEventCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-events-command.ts
var GetEventsCommand = class _GetEventsCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetEventsCommand(bb, obj) {
    return (obj || new _GetEventsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetEventsCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetEventsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetEventsCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endGetEventsCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetEventsCommand(builder, node) {
    _GetEventsCommand.startGetEventsCommand(builder);
    _GetEventsCommand.addNode(builder, node);
    return _GetEventsCommand.endGetEventsCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-idcommand.ts
var GetIDCommand = class _GetIDCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetIDCommand(bb, obj) {
    return (obj || new _GetIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetIDCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetIDCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endGetIDCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetIDCommand(builder, node) {
    _GetIDCommand.startGetIDCommand(builder);
    _GetIDCommand.addNode(builder, node);
    return _GetIDCommand.endGetIDCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-inline-style-command.ts
var GetInlineStyleCommand = class _GetInlineStyleCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetInlineStyleCommand(bb, obj) {
    return (obj || new _GetInlineStyleCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetInlineStyleCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetInlineStyleCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  propertyId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetInlineStyleCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addPropertyId(builder, propertyId) {
    builder.addFieldInt32(1, propertyId, 0);
  }
  static endGetInlineStyleCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetInlineStyleCommand(builder, node, propertyId) {
    _GetInlineStyleCommand.startGetInlineStyleCommand(builder);
    _GetInlineStyleCommand.addNode(builder, node);
    _GetInlineStyleCommand.addPropertyId(builder, propertyId);
    return _GetInlineStyleCommand.endGetInlineStyleCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-inline-styles-command.ts
var GetInlineStylesCommand = class _GetInlineStylesCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetInlineStylesCommand(bb, obj) {
    return (obj || new _GetInlineStylesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetInlineStylesCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetInlineStylesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetInlineStylesCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endGetInlineStylesCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetInlineStylesCommand(builder, node) {
    _GetInlineStylesCommand.startGetInlineStylesCommand(builder);
    _GetInlineStylesCommand.addNode(builder, node);
    return _GetInlineStylesCommand.endGetInlineStylesCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-page-element-command.ts
var GetPageElementCommand = class _GetPageElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetPageElementCommand(bb, obj) {
    return (obj || new _GetPageElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetPageElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetPageElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static startGetPageElementCommand(builder) {
    builder.startObject(0);
  }
  static endGetPageElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetPageElementCommand(builder) {
    _GetPageElementCommand.startGetPageElementCommand(builder);
    return _GetPageElementCommand.endGetPageElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-parent-command.ts
var GetParentCommand = class _GetParentCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetParentCommand(bb, obj) {
    return (obj || new _GetParentCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetParentCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetParentCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetParentCommand(builder) {
    builder.startObject(1);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static endGetParentCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetParentCommand(builder, current) {
    _GetParentCommand.startGetParentCommand(builder);
    _GetParentCommand.addCurrent(builder, current);
    return _GetParentCommand.endGetParentCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-tag-command.ts
var GetTagCommand = class _GetTagCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetTagCommand(bb, obj) {
    return (obj || new _GetTagCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetTagCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetTagCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetTagCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endGetTagCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetTagCommand(builder, node) {
    _GetTagCommand.startGetTagCommand(builder);
    _GetTagCommand.addNode(builder, node);
    return _GetTagCommand.endGetTagCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/get-template-parts-command.ts
var GetTemplatePartsCommand = class _GetTemplatePartsCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsGetTemplatePartsCommand(bb, obj) {
    return (obj || new _GetTemplatePartsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsGetTemplatePartsCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _GetTemplatePartsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startGetTemplatePartsCommand(builder) {
    builder.startObject(1);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static endGetTemplatePartsCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createGetTemplatePartsCommand(builder, ele) {
    _GetTemplatePartsCommand.startGetTemplatePartsCommand(builder);
    _GetTemplatePartsCommand.addEle(builder, ele);
    return _GetTemplatePartsCommand.endGetTemplatePartsCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/insert-element-before-command.ts
var InsertElementBeforeCommand = class _InsertElementBeforeCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsInsertElementBeforeCommand(bb, obj) {
    return (obj || new _InsertElementBeforeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsInsertElementBeforeCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _InsertElementBeforeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parent() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  marker() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : null;
  }
  static startInsertElementBeforeCommand(builder) {
    builder.startObject(3);
  }
  static addParent(builder, parent) {
    builder.addFieldInt32(0, parent, 0);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(1, current, 0);
  }
  static addMarker(builder, marker) {
    builder.addFieldInt32(2, marker, null);
  }
  static endInsertElementBeforeCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createInsertElementBeforeCommand(builder, parent, current, marker) {
    _InsertElementBeforeCommand.startInsertElementBeforeCommand(builder);
    _InsertElementBeforeCommand.addParent(builder, parent);
    _InsertElementBeforeCommand.addCurrent(builder, current);
    if (marker !== null)
      _InsertElementBeforeCommand.addMarker(builder, marker);
    return _InsertElementBeforeCommand.endInsertElementBeforeCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/insert-node-to-element-template-command.ts
var InsertNodeToElementTemplateCommand = class _InsertNodeToElementTemplateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsInsertNodeToElementTemplateCommand(bb, obj) {
    return (obj || new _InsertNodeToElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsInsertNodeToElementTemplateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _InsertNodeToElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  templateInstance() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  elementSlotIndex() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  child() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  referenceChild() {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : null;
  }
  static startInsertNodeToElementTemplateCommand(builder) {
    builder.startObject(4);
  }
  static addTemplateInstance(builder, templateInstance) {
    builder.addFieldInt32(0, templateInstance, 0);
  }
  static addElementSlotIndex(builder, elementSlotIndex) {
    builder.addFieldInt32(1, elementSlotIndex, 0);
  }
  static addChild(builder, child) {
    builder.addFieldInt32(2, child, 0);
  }
  static addReferenceChild(builder, referenceChild) {
    builder.addFieldInt32(3, referenceChild, null);
  }
  static endInsertNodeToElementTemplateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createInsertNodeToElementTemplateCommand(builder, templateInstance, elementSlotIndex, child, referenceChild) {
    _InsertNodeToElementTemplateCommand.startInsertNodeToElementTemplateCommand(builder);
    _InsertNodeToElementTemplateCommand.addTemplateInstance(builder, templateInstance);
    _InsertNodeToElementTemplateCommand.addElementSlotIndex(builder, elementSlotIndex);
    _InsertNodeToElementTemplateCommand.addChild(builder, child);
    if (referenceChild !== null)
      _InsertNodeToElementTemplateCommand.addReferenceChild(builder, referenceChild);
    return _InsertNodeToElementTemplateCommand.endInsertNodeToElementTemplateCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/invoke-uimethod-command.ts
var InvokeUIMethodCommand = class _InvokeUIMethodCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsInvokeUIMethodCommand(bb, obj) {
    return (obj || new _InvokeUIMethodCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsInvokeUIMethodCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _InvokeUIMethodCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  e() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  method(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  params(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  callback() {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startInvokeUIMethodCommand(builder) {
    builder.startObject(4);
  }
  static addE(builder, e) {
    builder.addFieldInt32(0, e, 0);
  }
  static addMethod(builder, methodOffset) {
    builder.addFieldOffset(1, methodOffset, 0);
  }
  static addParams(builder, paramsOffset) {
    builder.addFieldOffset(2, paramsOffset, 0);
  }
  static addCallback(builder, callback) {
    builder.addFieldInt32(3, callback, 0);
  }
  static endInvokeUIMethodCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/is-template-element-command.ts
var IsTemplateElementCommand = class _IsTemplateElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsIsTemplateElementCommand(bb, obj) {
    return (obj || new _IsTemplateElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsIsTemplateElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _IsTemplateElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startIsTemplateElementCommand(builder) {
    builder.startObject(1);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static endIsTemplateElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createIsTemplateElementCommand(builder, ele) {
    _IsTemplateElementCommand.startIsTemplateElementCommand(builder);
    _IsTemplateElementCommand.addEle(builder, ele);
    return _IsTemplateElementCommand.endIsTemplateElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/last-element-command.ts
var LastElementCommand = class _LastElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsLastElementCommand(bb, obj) {
    return (obj || new _LastElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsLastElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _LastElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startLastElementCommand(builder) {
    builder.startObject(1);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static endLastElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createLastElementCommand(builder, current) {
    _LastElementCommand.startLastElementCommand(builder);
    _LastElementCommand.addCurrent(builder, current);
    return _LastElementCommand.endLastElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/load-lepus-chunk-command.ts
var LoadLepusChunkCommand = class _LoadLepusChunkCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsLoadLepusChunkCommand(bb, obj) {
    return (obj || new _LoadLepusChunkCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsLoadLepusChunkCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _LoadLepusChunkCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  cfg(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startLoadLepusChunkCommand(builder) {
    builder.startObject(2);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(0, nameOffset, 0);
  }
  static addCfg(builder, cfgOffset) {
    builder.addFieldOffset(1, cfgOffset, 0);
  }
  static endLoadLepusChunkCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/mark-part-element-command.ts
var MarkPartElementCommand = class _MarkPartElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsMarkPartElementCommand(bb, obj) {
    return (obj || new _MarkPartElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsMarkPartElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _MarkPartElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  key(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startMarkPartElementCommand(builder) {
    builder.startObject(2);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static addKey(builder, keyOffset) {
    builder.addFieldOffset(1, keyOffset, 0);
  }
  static endMarkPartElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createMarkPartElementCommand(builder, ele, keyOffset) {
    _MarkPartElementCommand.startMarkPartElementCommand(builder);
    _MarkPartElementCommand.addEle(builder, ele);
    _MarkPartElementCommand.addKey(builder, keyOffset);
    return _MarkPartElementCommand.endMarkPartElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/mark-template-element-command.ts
var MarkTemplateElementCommand = class _MarkTemplateElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsMarkTemplateElementCommand(bb, obj) {
    return (obj || new _MarkTemplateElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsMarkTemplateElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _MarkTemplateElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startMarkTemplateElementCommand(builder) {
    builder.startObject(1);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static endMarkTemplateElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createMarkTemplateElementCommand(builder, ele) {
    _MarkTemplateElementCommand.startMarkTemplateElementCommand(builder);
    _MarkTemplateElementCommand.addEle(builder, ele);
    return _MarkTemplateElementCommand.endMarkTemplateElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/mark-timing-command.ts
var MarkTimingCommand = class _MarkTimingCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsMarkTimingCommand(bb, obj) {
    return (obj || new _MarkTimingCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsMarkTimingCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _MarkTimingCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  pipeLineId(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  timingFlag(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startMarkTimingCommand(builder) {
    builder.startObject(2);
  }
  static addPipeLineId(builder, pipeLineIdOffset) {
    builder.addFieldOffset(0, pipeLineIdOffset, 0);
  }
  static addTimingFlag(builder, timingFlagOffset) {
    builder.addFieldOffset(1, timingFlagOffset, 0);
  }
  static endMarkTimingCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createMarkTimingCommand(builder, pipeLineIdOffset, timingFlagOffset) {
    _MarkTimingCommand.startMarkTimingCommand(builder);
    _MarkTimingCommand.addPipeLineId(builder, pipeLineIdOffset);
    _MarkTimingCommand.addTimingFlag(builder, timingFlagOffset);
    return _MarkTimingCommand.endMarkTimingCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/next-element-command.ts
var NextElementCommand = class _NextElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsNextElementCommand(bb, obj) {
    return (obj || new _NextElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsNextElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _NextElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startNextElementCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endNextElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createNextElementCommand(builder, node) {
    _NextElementCommand.startNextElementCommand(builder);
    _NextElementCommand.addNode(builder, node);
    return _NextElementCommand.endNextElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/on-lifecycle-event-command.ts
var OnLifecycleEventCommand = class _OnLifecycleEventCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsOnLifecycleEventCommand(bb, obj) {
    return (obj || new _OnLifecycleEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsOnLifecycleEventCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _OnLifecycleEventCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  args(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startOnLifecycleEventCommand(builder) {
    builder.startObject(1);
  }
  static addArgs(builder, argsOffset) {
    builder.addFieldOffset(0, argsOffset, 0);
  }
  static endOnLifecycleEventCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createOnLifecycleEventCommand(builder, argsOffset) {
    _OnLifecycleEventCommand.startOnLifecycleEventCommand(builder);
    _OnLifecycleEventCommand.addArgs(builder, argsOffset);
    return _OnLifecycleEventCommand.endOnLifecycleEventCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/on-pipeline-start-command.ts
var OnPipelineStartCommand = class _OnPipelineStartCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsOnPipelineStartCommand(bb, obj) {
    return (obj || new _OnPipelineStartCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsOnPipelineStartCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _OnPipelineStartCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  pipeLineId(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  pipeLineOrigin(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startOnPipelineStartCommand(builder) {
    builder.startObject(2);
  }
  static addPipeLineId(builder, pipeLineIdOffset) {
    builder.addFieldOffset(0, pipeLineIdOffset, 0);
  }
  static addPipeLineOrigin(builder, pipeLineOriginOffset) {
    builder.addFieldOffset(1, pipeLineOriginOffset, 0);
  }
  static endOnPipelineStartCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createOnPipelineStartCommand(builder, pipeLineIdOffset, pipeLineOriginOffset) {
    _OnPipelineStartCommand.startOnPipelineStartCommand(builder);
    _OnPipelineStartCommand.addPipeLineId(builder, pipeLineIdOffset);
    _OnPipelineStartCommand.addPipeLineOrigin(builder, pipeLineOriginOffset);
    return _OnPipelineStartCommand.endOnPipelineStartCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/query-component-command.ts
var QueryComponentCommand = class _QueryComponentCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsQueryComponentCommand(bb, obj) {
    return (obj || new _QueryComponentCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsQueryComponentCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _QueryComponentCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  source(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  callback() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : null;
  }
  static startQueryComponentCommand(builder) {
    builder.startObject(2);
  }
  static addSource(builder, sourceOffset) {
    builder.addFieldOffset(0, sourceOffset, 0);
  }
  static addCallback(builder, callback) {
    builder.addFieldInt32(1, callback, null);
  }
  static endQueryComponentCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createQueryComponentCommand(builder, sourceOffset, callback) {
    _QueryComponentCommand.startQueryComponentCommand(builder);
    _QueryComponentCommand.addSource(builder, sourceOffset);
    if (callback !== null)
      _QueryComponentCommand.addCallback(builder, callback);
    return _QueryComponentCommand.endQueryComponentCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/query-selector-all-command.ts
var QuerySelectorAllCommand = class _QuerySelectorAllCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsQuerySelectorAllCommand(bb, obj) {
    return (obj || new _QuerySelectorAllCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsQuerySelectorAllCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _QuerySelectorAllCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  root() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  cssSelector(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  params(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startQuerySelectorAllCommand(builder) {
    builder.startObject(3);
  }
  static addRoot(builder, root) {
    builder.addFieldInt32(0, root, 0);
  }
  static addCssSelector(builder, cssSelectorOffset) {
    builder.addFieldOffset(1, cssSelectorOffset, 0);
  }
  static addParams(builder, paramsOffset) {
    builder.addFieldOffset(2, paramsOffset, 0);
  }
  static endQuerySelectorAllCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/query-selector-command.ts
var QuerySelectorCommand = class _QuerySelectorCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsQuerySelectorCommand(bb, obj) {
    return (obj || new _QuerySelectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsQuerySelectorCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _QuerySelectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  root() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  cssSelector(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  params(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startQuerySelectorCommand(builder) {
    builder.startObject(3);
  }
  static addRoot(builder, root) {
    builder.addFieldInt32(0, root, 0);
  }
  static addCssSelector(builder, cssSelectorOffset) {
    builder.addFieldOffset(1, cssSelectorOffset, 0);
  }
  static addParams(builder, paramsOffset) {
    builder.addFieldOffset(2, paramsOffset, 0);
  }
  static endQuerySelectorCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/release-element-command.ts
var ReleaseElementCommand = class _ReleaseElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsReleaseElementCommand(bb, obj) {
    return (obj || new _ReleaseElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsReleaseElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ReleaseElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startReleaseElementCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endReleaseElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createReleaseElementCommand(builder, node) {
    _ReleaseElementCommand.startReleaseElementCommand(builder);
    _ReleaseElementCommand.addNode(builder, node);
    return _ReleaseElementCommand.endReleaseElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/remove-element-command.ts
var RemoveElementCommand = class _RemoveElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsRemoveElementCommand(bb, obj) {
    return (obj || new _RemoveElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsRemoveElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _RemoveElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parent() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startRemoveElementCommand(builder) {
    builder.startObject(2);
  }
  static addParent(builder, parent) {
    builder.addFieldInt32(0, parent, 0);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(1, current, 0);
  }
  static endRemoveElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createRemoveElementCommand(builder, parent, current) {
    _RemoveElementCommand.startRemoveElementCommand(builder);
    _RemoveElementCommand.addParent(builder, parent);
    _RemoveElementCommand.addCurrent(builder, current);
    return _RemoveElementCommand.endRemoveElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/remove-event-listener-command.ts
var RemoveEventListenerCommand = class _RemoveEventListenerCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsRemoveEventListenerCommand(bb, obj) {
    return (obj || new _RemoveEventListenerCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsRemoveEventListenerCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _RemoveEventListenerCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  name(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  callback() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  options(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startRemoveEventListenerCommand(builder) {
    builder.startObject(4);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addName(builder, nameOffset) {
    builder.addFieldOffset(1, nameOffset, 0);
  }
  static addCallback(builder, callback) {
    builder.addFieldInt32(2, callback, 0);
  }
  static addOptions(builder, optionsOffset) {
    builder.addFieldOffset(3, optionsOffset, 0);
  }
  static endRemoveEventListenerCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/remove-event-listeners-command.ts
var RemoveEventListenersCommand = class _RemoveEventListenersCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsRemoveEventListenersCommand(bb, obj) {
    return (obj || new _RemoveEventListenersCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsRemoveEventListenersCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _RemoveEventListenersCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startRemoveEventListenersCommand(builder) {
    builder.startObject(1);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static endRemoveEventListenersCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createRemoveEventListenersCommand(builder, node) {
    _RemoveEventListenersCommand.startRemoveEventListenersCommand(builder);
    _RemoveEventListenersCommand.addNode(builder, node);
    return _RemoveEventListenersCommand.endRemoveEventListenersCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/remove-gesture-detector-command.ts
var RemoveGestureDetectorCommand = class _RemoveGestureDetectorCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsRemoveGestureDetectorCommand(bb, obj) {
    return (obj || new _RemoveGestureDetectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsRemoveGestureDetectorCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _RemoveGestureDetectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  gestureId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startRemoveGestureDetectorCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addGestureId(builder, gestureId) {
    builder.addFieldInt32(1, gestureId, 0);
  }
  static endRemoveGestureDetectorCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createRemoveGestureDetectorCommand(builder, node, gestureId) {
    _RemoveGestureDetectorCommand.startRemoveGestureDetectorCommand(builder);
    _RemoveGestureDetectorCommand.addNode(builder, node);
    _RemoveGestureDetectorCommand.addGestureId(builder, gestureId);
    return _RemoveGestureDetectorCommand.endRemoveGestureDetectorCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/remove-node-from-element-template-command.ts
var RemoveNodeFromElementTemplateCommand = class _RemoveNodeFromElementTemplateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsRemoveNodeFromElementTemplateCommand(bb, obj) {
    return (obj || new _RemoveNodeFromElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsRemoveNodeFromElementTemplateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _RemoveNodeFromElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  templateInstance() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  elementSlotIndex() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  child() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startRemoveNodeFromElementTemplateCommand(builder) {
    builder.startObject(3);
  }
  static addTemplateInstance(builder, templateInstance) {
    builder.addFieldInt32(0, templateInstance, 0);
  }
  static addElementSlotIndex(builder, elementSlotIndex) {
    builder.addFieldInt32(1, elementSlotIndex, 0);
  }
  static addChild(builder, child) {
    builder.addFieldInt32(2, child, 0);
  }
  static endRemoveNodeFromElementTemplateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createRemoveNodeFromElementTemplateCommand(builder, templateInstance, elementSlotIndex, child) {
    _RemoveNodeFromElementTemplateCommand.startRemoveNodeFromElementTemplateCommand(builder);
    _RemoveNodeFromElementTemplateCommand.addTemplateInstance(builder, templateInstance);
    _RemoveNodeFromElementTemplateCommand.addElementSlotIndex(builder, elementSlotIndex);
    _RemoveNodeFromElementTemplateCommand.addChild(builder, child);
    return _RemoveNodeFromElementTemplateCommand.endRemoveNodeFromElementTemplateCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/replace-element-command.ts
var ReplaceElementCommand = class _ReplaceElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsReplaceElementCommand(bb, obj) {
    return (obj || new _ReplaceElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsReplaceElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ReplaceElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  newElement() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  oldElement() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startReplaceElementCommand(builder) {
    builder.startObject(2);
  }
  static addNewElement(builder, newElement) {
    builder.addFieldInt32(0, newElement, 0);
  }
  static addOldElement(builder, oldElement) {
    builder.addFieldInt32(1, oldElement, 0);
  }
  static endReplaceElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createReplaceElementCommand(builder, newElement, oldElement) {
    _ReplaceElementCommand.startReplaceElementCommand(builder);
    _ReplaceElementCommand.addNewElement(builder, newElement);
    _ReplaceElementCommand.addOldElement(builder, oldElement);
    return _ReplaceElementCommand.endReplaceElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/replace-elements-command.ts
var ReplaceElementsCommand = class _ReplaceElementsCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsReplaceElementsCommand(bb, obj) {
    return (obj || new _ReplaceElementsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsReplaceElementsCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ReplaceElementsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  parent() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  insertedChildren(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new ElementReferences()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  removedChildren(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new ElementReferences()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startReplaceElementsCommand(builder) {
    builder.startObject(3);
  }
  static addParent(builder, parent) {
    builder.addFieldInt32(0, parent, 0);
  }
  static addInsertedChildren(builder, insertedChildrenOffset) {
    builder.addFieldOffset(1, insertedChildrenOffset, 0);
  }
  static addRemovedChildren(builder, removedChildrenOffset) {
    builder.addFieldOffset(2, removedChildrenOffset, 0);
  }
  static endReplaceElementsCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/report-error-command.ts
var ReportErrorCommand = class _ReportErrorCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsReportErrorCommand(bb, obj) {
    return (obj || new _ReportErrorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsReportErrorCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ReportErrorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  err(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  info(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startReportErrorCommand(builder) {
    builder.startObject(2);
  }
  static addErr(builder, errOffset) {
    builder.addFieldOffset(0, errOffset, 0);
  }
  static addInfo(builder, infoOffset) {
    builder.addFieldOffset(1, infoOffset, 0);
  }
  static endReportErrorCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/serialize-element-template-command.ts
var SerializeElementTemplateCommand = class _SerializeElementTemplateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSerializeElementTemplateCommand(bb, obj) {
    return (obj || new _SerializeElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSerializeElementTemplateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SerializeElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  templateInstance() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startSerializeElementTemplateCommand(builder) {
    builder.startObject(1);
  }
  static addTemplateInstance(builder, templateInstance) {
    builder.addFieldInt32(0, templateInstance, 0);
  }
  static endSerializeElementTemplateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createSerializeElementTemplateCommand(builder, templateInstance) {
    _SerializeElementTemplateCommand.startSerializeElementTemplateCommand(builder);
    _SerializeElementTemplateCommand.addTemplateInstance(builder, templateInstance);
    return _SerializeElementTemplateCommand.endSerializeElementTemplateCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-attribute-command.ts
var SetAttributeCommand = class _SetAttributeCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetAttributeCommand(bb, obj) {
    return (obj || new _SetAttributeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetAttributeCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetAttributeCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  attrName(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetAttributeCommand(builder) {
    builder.startObject(3);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static addAttrName(builder, attrNameOffset) {
    builder.addFieldOffset(1, attrNameOffset, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(2, valueOffset, 0);
  }
  static endSetAttributeCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-attribute-of-element-template-command.ts
var SetAttributeOfElementTemplateCommand = class _SetAttributeOfElementTemplateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetAttributeOfElementTemplateCommand(bb, obj) {
    return (obj || new _SetAttributeOfElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetAttributeOfElementTemplateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetAttributeOfElementTemplateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  templateInstance() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  attrSlotIndex() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetAttributeOfElementTemplateCommand(builder) {
    builder.startObject(3);
  }
  static addTemplateInstance(builder, templateInstance) {
    builder.addFieldInt32(0, templateInstance, 0);
  }
  static addAttrSlotIndex(builder, attrSlotIndex) {
    builder.addFieldInt32(1, attrSlotIndex, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(2, valueOffset, 0);
  }
  static endSetAttributeOfElementTemplateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-cssid-command.ts
var SetCSSIdCommand = class _SetCSSIdCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetCSSIdCommand(bb, obj) {
    return (obj || new _SetCSSIdCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetCSSIdCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetCSSIdCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new ElementReferences()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  cssId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  entryName(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startSetCSSIdCommand(builder) {
    builder.startObject(3);
  }
  static addNode(builder, nodeOffset) {
    builder.addFieldOffset(0, nodeOffset, 0);
  }
  static addCssId(builder, cssId) {
    builder.addFieldInt32(1, cssId, 0);
  }
  static addEntryName(builder, entryNameOffset) {
    builder.addFieldOffset(2, entryNameOffset, 0);
  }
  static endSetCSSIdCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createSetCSSIdCommand(builder, nodeOffset, cssId, entryNameOffset) {
    _SetCSSIdCommand.startSetCSSIdCommand(builder);
    _SetCSSIdCommand.addNode(builder, nodeOffset);
    _SetCSSIdCommand.addCssId(builder, cssId);
    _SetCSSIdCommand.addEntryName(builder, entryNameOffset);
    return _SetCSSIdCommand.endSetCSSIdCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-classes-command.ts
var SetClassesCommand = class _SetClassesCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetClassesCommand(bb, obj) {
    return (obj || new _SetClassesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetClassesCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetClassesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  current() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  className(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startSetClassesCommand(builder) {
    builder.startObject(2);
  }
  static addCurrent(builder, current) {
    builder.addFieldInt32(0, current, 0);
  }
  static addClassName(builder, classNameOffset) {
    builder.addFieldOffset(1, classNameOffset, 0);
  }
  static endSetClassesCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createSetClassesCommand(builder, current, classNameOffset) {
    _SetClassesCommand.startSetClassesCommand(builder);
    _SetClassesCommand.addCurrent(builder, current);
    _SetClassesCommand.addClassName(builder, classNameOffset);
    return _SetClassesCommand.endSetClassesCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-config-command.ts
var SetConfigCommand = class _SetConfigCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetConfigCommand(bb, obj) {
    return (obj || new _SetConfigCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetConfigCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetConfigCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  ele() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  config(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetConfigCommand(builder) {
    builder.startObject(2);
  }
  static addEle(builder, ele) {
    builder.addFieldInt32(0, ele, 0);
  }
  static addConfig(builder, configOffset) {
    builder.addFieldOffset(1, configOffset, 0);
  }
  static endSetConfigCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-dataset-command.ts
var SetDatasetCommand = class _SetDatasetCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetDatasetCommand(bb, obj) {
    return (obj || new _SetDatasetCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetDatasetCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetDatasetCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetDatasetCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(1, valueOffset, 0);
  }
  static endSetDatasetCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-events-command.ts
var SetEventsCommand = class _SetEventsCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetEventsCommand(bb, obj) {
    return (obj || new _SetEventsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetEventsCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetEventsCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  events(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetEventsCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addEvents(builder, eventsOffset) {
    builder.addFieldOffset(1, eventsOffset, 0);
  }
  static endSetEventsCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-gesture-detector-command.ts
var SetGestureDetectorCommand = class _SetGestureDetectorCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetGestureDetectorCommand(bb, obj) {
    return (obj || new _SetGestureDetectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetGestureDetectorCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetGestureDetectorCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  gestureId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  gestureType() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readFloat64(this.bb_pos + offset) : 0;
  }
  config(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  relationMap(index) {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.readFloat64(this.bb.__vector(this.bb_pos + offset) + index * 8) : 0;
  }
  relationMapLength() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  relationMapArray() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? new Float64Array(this.bb.bytes().buffer, this.bb.bytes().byteOffset + this.bb.__vector(this.bb_pos + offset), this.bb.__vector_len(this.bb_pos + offset)) : null;
  }
  static startSetGestureDetectorCommand(builder) {
    builder.startObject(5);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addGestureId(builder, gestureId) {
    builder.addFieldInt32(1, gestureId, 0);
  }
  static addGestureType(builder, gestureType) {
    builder.addFieldFloat64(2, gestureType, 0);
  }
  static addConfig(builder, configOffset) {
    builder.addFieldOffset(3, configOffset, 0);
  }
  static addRelationMap(builder, relationMapOffset) {
    builder.addFieldOffset(4, relationMapOffset, 0);
  }
  static createRelationMapVector(builder, data) {
    builder.startVector(8, data.length, 8);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addFloat64(data[i]);
    }
    return builder.endVector();
  }
  static startRelationMapVector(builder, numElems) {
    builder.startVector(8, numElems, 8);
  }
  static endSetGestureDetectorCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-gesture-state-command.ts
var SetGestureStateCommand = class _SetGestureStateCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetGestureStateCommand(bb, obj) {
    return (obj || new _SetGestureStateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetGestureStateCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetGestureStateCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  gestureId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  state() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readFloat64(this.bb_pos + offset) : 0;
  }
  static startSetGestureStateCommand(builder) {
    builder.startObject(3);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addGestureId(builder, gestureId) {
    builder.addFieldInt32(1, gestureId, 0);
  }
  static addState(builder, state) {
    builder.addFieldFloat64(2, state, 0);
  }
  static endSetGestureStateCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createSetGestureStateCommand(builder, node, gestureId, state) {
    _SetGestureStateCommand.startSetGestureStateCommand(builder);
    _SetGestureStateCommand.addNode(builder, node);
    _SetGestureStateCommand.addGestureId(builder, gestureId);
    _SetGestureStateCommand.addState(builder, state);
    return _SetGestureStateCommand.endSetGestureStateCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-idcommand.ts
var SetIDCommand = class _SetIDCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetIDCommand(bb, obj) {
    return (obj || new _SetIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetIDCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  id(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startSetIDCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addId(builder, idOffset) {
    builder.addFieldOffset(1, idOffset, 0);
  }
  static endSetIDCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createSetIDCommand(builder, node, idOffset) {
    _SetIDCommand.startSetIDCommand(builder);
    _SetIDCommand.addNode(builder, node);
    _SetIDCommand.addId(builder, idOffset);
    return _SetIDCommand.endSetIDCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-inline-styles-command.ts
var SetInlineStylesCommand = class _SetInlineStylesCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetInlineStylesCommand(bb, obj) {
    return (obj || new _SetInlineStylesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetInlineStylesCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetInlineStylesCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetInlineStylesCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(1, valueOffset, 0);
  }
  static endSetInlineStylesCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-lepus-init-data-command.ts
var SetLepusInitDataCommand = class _SetLepusInitDataCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetLepusInitDataCommand(bb, obj) {
    return (obj || new _SetLepusInitDataCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetLepusInitDataCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetLepusInitDataCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  initData(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetLepusInitDataCommand(builder) {
    builder.startObject(1);
  }
  static addInitData(builder, initDataOffset) {
    builder.addFieldOffset(0, initDataOffset, 0);
  }
  static endSetLepusInitDataCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createSetLepusInitDataCommand(builder, initDataOffset) {
    _SetLepusInitDataCommand.startSetLepusInitDataCommand(builder);
    _SetLepusInitDataCommand.addInitData(builder, initDataOffset);
    return _SetLepusInitDataCommand.endSetLepusInitDataCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-static-style-command.ts
var SetStaticStyleCommand = class _SetStaticStyleCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetStaticStyleCommand(bb, obj) {
    return (obj || new _SetStaticStyleCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetStaticStyleCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetStaticStyleCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  key() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readFloat64(this.bb_pos + offset) : 0;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetStaticStyleCommand(builder) {
    builder.startObject(3);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addKey(builder, key) {
    builder.addFieldFloat64(1, key, 0);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(2, valueOffset, 0);
  }
  static endSetStaticStyleCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/set-style-object-command.ts
var SetStyleObjectCommand = class _SetStyleObjectCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSetStyleObjectCommand(bb, obj) {
    return (obj || new _SetStyleObjectCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSetStyleObjectCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SetStyleObjectCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  elementRef() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  styleObjects(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startSetStyleObjectCommand(builder) {
    builder.startObject(2);
  }
  static addElementRef(builder, elementRef) {
    builder.addFieldInt32(0, elementRef, 0);
  }
  static addStyleObjects(builder, styleObjectsOffset) {
    builder.addFieldOffset(1, styleObjectsOffset, 0);
  }
  static endSetStyleObjectCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/stop-immediate-propagation-command.ts
var StopImmediatePropagationCommand = class _StopImmediatePropagationCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsStopImmediatePropagationCommand(bb, obj) {
    return (obj || new _StopImmediatePropagationCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsStopImmediatePropagationCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _StopImmediatePropagationCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  event(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startStopImmediatePropagationCommand(builder) {
    builder.startObject(1);
  }
  static addEvent(builder, eventOffset) {
    builder.addFieldOffset(0, eventOffset, 0);
  }
  static endStopImmediatePropagationCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createStopImmediatePropagationCommand(builder, eventOffset) {
    _StopImmediatePropagationCommand.startStopImmediatePropagationCommand(builder);
    _StopImmediatePropagationCommand.addEvent(builder, eventOffset);
    return _StopImmediatePropagationCommand.endStopImmediatePropagationCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/stop-propagation-command.ts
var StopPropagationCommand = class _StopPropagationCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsStopPropagationCommand(bb, obj) {
    return (obj || new _StopPropagationCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsStopPropagationCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _StopPropagationCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  event(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startStopPropagationCommand(builder) {
    builder.startObject(1);
  }
  static addEvent(builder, eventOffset) {
    builder.addFieldOffset(0, eventOffset, 0);
  }
  static endStopPropagationCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createStopPropagationCommand(builder, eventOffset) {
    _StopPropagationCommand.startStopPropagationCommand(builder);
    _StopPropagationCommand.addEvent(builder, eventOffset);
    return _StopPropagationCommand.endStopPropagationCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/swap-element-command.ts
var SwapElementCommand = class _SwapElementCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsSwapElementCommand(bb, obj) {
    return (obj || new _SwapElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsSwapElementCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _SwapElementCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  left() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  right() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startSwapElementCommand(builder) {
    builder.startObject(2);
  }
  static addLeft(builder, left) {
    builder.addFieldInt32(0, left, 0);
  }
  static addRight(builder, right) {
    builder.addFieldInt32(1, right, 0);
  }
  static endSwapElementCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createSwapElementCommand(builder, left, right) {
    _SwapElementCommand.startSwapElementCommand(builder);
    _SwapElementCommand.addLeft(builder, left);
    _SwapElementCommand.addRight(builder, right);
    return _SwapElementCommand.endSwapElementCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/update-component-idcommand.ts
var UpdateComponentIDCommand = class _UpdateComponentIDCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsUpdateComponentIDCommand(bb, obj) {
    return (obj || new _UpdateComponentIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsUpdateComponentIDCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _UpdateComponentIDCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  id(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startUpdateComponentIDCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addId(builder, idOffset) {
    builder.addFieldOffset(1, idOffset, 0);
  }
  static endUpdateComponentIDCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createUpdateComponentIDCommand(builder, node, idOffset) {
    _UpdateComponentIDCommand.startUpdateComponentIDCommand(builder);
    _UpdateComponentIDCommand.addNode(builder, node);
    _UpdateComponentIDCommand.addId(builder, idOffset);
    return _UpdateComponentIDCommand.endUpdateComponentIDCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/update-component-info-command.ts
var UpdateComponentInfoCommand = class _UpdateComponentInfoCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsUpdateComponentInfoCommand(bb, obj) {
    return (obj || new _UpdateComponentInfoCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsUpdateComponentInfoCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _UpdateComponentInfoCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  params(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startUpdateComponentInfoCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addParams(builder, paramsOffset) {
    builder.addFieldOffset(1, paramsOffset, 0);
  }
  static endUpdateComponentInfoCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/update-for-child-count-command.ts
var UpdateForChildCountCommand = class _UpdateForChildCountCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsUpdateForChildCountCommand(bb, obj) {
    return (obj || new _UpdateForChildCountCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsUpdateForChildCountCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _UpdateForChildCountCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  childCount() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startUpdateForChildCountCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addChildCount(builder, childCount) {
    builder.addFieldInt32(1, childCount, 0);
  }
  static endUpdateForChildCountCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createUpdateForChildCountCommand(builder, node, childCount) {
    _UpdateForChildCountCommand.startUpdateForChildCountCommand(builder);
    _UpdateForChildCountCommand.addNode(builder, node);
    _UpdateForChildCountCommand.addChildCount(builder, childCount);
    return _UpdateForChildCountCommand.endUpdateForChildCountCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/update-if-node-index-command.ts
var UpdateIfNodeIndexCommand = class _UpdateIfNodeIndexCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsUpdateIfNodeIndexCommand(bb, obj) {
    return (obj || new _UpdateIfNodeIndexCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsUpdateIfNodeIndexCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _UpdateIfNodeIndexCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  ifIndex() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startUpdateIfNodeIndexCommand(builder) {
    builder.startObject(2);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addIfIndex(builder, ifIndex) {
    builder.addFieldInt32(1, ifIndex, 0);
  }
  static endUpdateIfNodeIndexCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createUpdateIfNodeIndexCommand(builder, node, ifIndex) {
    _UpdateIfNodeIndexCommand.startUpdateIfNodeIndexCommand(builder);
    _UpdateIfNodeIndexCommand.addNode(builder, node);
    _UpdateIfNodeIndexCommand.addIfIndex(builder, ifIndex);
    return _UpdateIfNodeIndexCommand.endUpdateIfNodeIndexCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/update-list-callbacks-command.ts
var UpdateListCallbacksCommand = class _UpdateListCallbacksCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsUpdateListCallbacksCommand(bb, obj) {
    return (obj || new _UpdateListCallbacksCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsUpdateListCallbacksCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _UpdateListCallbacksCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  node() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  componentAtIndex() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  enqueueComponent() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  componentAtIndexes() {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startUpdateListCallbacksCommand(builder) {
    builder.startObject(4);
  }
  static addNode(builder, node) {
    builder.addFieldInt32(0, node, 0);
  }
  static addComponentAtIndex(builder, componentAtIndex) {
    builder.addFieldInt32(1, componentAtIndex, 0);
  }
  static addEnqueueComponent(builder, enqueueComponent) {
    builder.addFieldInt32(2, enqueueComponent, 0);
  }
  static addComponentAtIndexes(builder, componentAtIndexes) {
    builder.addFieldInt32(3, componentAtIndexes, 0);
  }
  static endUpdateListCallbacksCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createUpdateListCallbacksCommand(builder, node, componentAtIndex, enqueueComponent, componentAtIndexes) {
    _UpdateListCallbacksCommand.startUpdateListCallbacksCommand(builder);
    _UpdateListCallbacksCommand.addNode(builder, node);
    _UpdateListCallbacksCommand.addComponentAtIndex(builder, componentAtIndex);
    _UpdateListCallbacksCommand.addEnqueueComponent(builder, enqueueComponent);
    _UpdateListCallbacksCommand.addComponentAtIndexes(builder, componentAtIndexes);
    return _UpdateListCallbacksCommand.endUpdateListCallbacksCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/update-style-object-command.ts
var UpdateStyleObjectCommand = class _UpdateStyleObjectCommand {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsUpdateStyleObjectCommand(bb, obj) {
    return (obj || new _UpdateStyleObjectCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsUpdateStyleObjectCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _UpdateStyleObjectCommand()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  styleObjectRef(obj) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  styleObject(obj) {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? (obj || new Payload()).__init(this.bb.__indirect(this.bb_pos + offset), this.bb) : null;
  }
  static startUpdateStyleObjectCommand(builder) {
    builder.startObject(2);
  }
  static addStyleObjectRef(builder, styleObjectRefOffset) {
    builder.addFieldOffset(0, styleObjectRefOffset, 0);
  }
  static addStyleObject(builder, styleObjectOffset) {
    builder.addFieldOffset(1, styleObjectOffset, 0);
  }
  static endUpdateStyleObjectCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/command.ts
var Command = class _Command {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCommand(bb, obj) {
    return (obj || new _Command()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCommand(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _Command()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  resultSlot() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 4294967295;
  }
  resultNodeId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  resultNodeIds(index) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb.__vector(this.bb_pos + offset) + index * 4) : 0;
  }
  resultNodeIdsLength() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  resultNodeIdsArray() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? new Uint32Array(this.bb.bytes().buffer, this.bb.bytes().byteOffset + this.bb.__vector(this.bb_pos + offset), this.bb.__vector_len(this.bb_pos + offset)) : null;
  }
  listenerId() {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  operationType() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.readUint8(this.bb_pos + offset) : 0 /* NONE */;
  }
  operation(obj) {
    const offset = this.bb.__offset(this.bb_pos, 14);
    return offset ? this.bb.__union(obj, this.bb_pos + offset) : null;
  }
  static startCommand(builder) {
    builder.startObject(6);
  }
  static addResultSlot(builder, resultSlot) {
    builder.addFieldInt32(0, resultSlot, 4294967295);
  }
  static addResultNodeId(builder, resultNodeId) {
    builder.addFieldInt32(1, resultNodeId, 0);
  }
  static addResultNodeIds(builder, resultNodeIdsOffset) {
    builder.addFieldOffset(2, resultNodeIdsOffset, 0);
  }
  static createResultNodeIdsVector(builder, data) {
    builder.startVector(4, data.length, 4);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addInt32(data[i]);
    }
    return builder.endVector();
  }
  static startResultNodeIdsVector(builder, numElems) {
    builder.startVector(4, numElems, 4);
  }
  static addListenerId(builder, listenerId) {
    builder.addFieldInt32(3, listenerId, 0);
  }
  static addOperationType(builder, operationType) {
    builder.addFieldInt8(4, operationType, 0 /* NONE */);
  }
  static addOperation(builder, operationOffset) {
    builder.addFieldOffset(5, operationOffset, 0);
  }
  static endCommand(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createCommand(builder, resultSlot, resultNodeId, resultNodeIdsOffset, listenerId, operationType, operationOffset) {
    _Command.startCommand(builder);
    _Command.addResultSlot(builder, resultSlot);
    _Command.addResultNodeId(builder, resultNodeId);
    _Command.addResultNodeIds(builder, resultNodeIdsOffset);
    _Command.addListenerId(builder, listenerId);
    _Command.addOperationType(builder, operationType);
    _Command.addOperation(builder, operationOffset);
    return _Command.endCommand(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/command-batch.ts
var CommandBatch = class _CommandBatch {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsCommandBatch(bb, obj) {
    return (obj || new _CommandBatch()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsCommandBatch(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _CommandBatch()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  sessionId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  sequence() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  commands(index, obj) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? (obj || new Command()).__init(this.bb.__indirect(this.bb.__vector(this.bb_pos + offset) + index * 4), this.bb) : null;
  }
  commandsLength() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  finalCommit() {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? !!this.bb.readInt8(this.bb_pos + offset) : true;
  }
  static startCommandBatch(builder) {
    builder.startObject(4);
  }
  static addSessionId(builder, sessionId) {
    builder.addFieldInt32(0, sessionId, 0);
  }
  static addSequence(builder, sequence) {
    builder.addFieldInt32(1, sequence, 0);
  }
  static addCommands(builder, commandsOffset) {
    builder.addFieldOffset(2, commandsOffset, 0);
  }
  static createCommandsVector(builder, data) {
    builder.startVector(4, data.length, 4);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addOffset(data[i]);
    }
    return builder.endVector();
  }
  static startCommandsVector(builder, numElems) {
    builder.startVector(4, numElems, 4);
  }
  static addFinalCommit(builder, finalCommit) {
    builder.addFieldInt8(3, +finalCommit, 1);
  }
  static endCommandBatch(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createCommandBatch(builder, sessionId, sequence, commandsOffset, finalCommit) {
    _CommandBatch.startCommandBatch(builder);
    _CommandBatch.addSessionId(builder, sessionId);
    _CommandBatch.addSequence(builder, sequence);
    _CommandBatch.addCommands(builder, commandsOffset);
    _CommandBatch.addFinalCommit(builder, finalCommit);
    return _CommandBatch.endCommandBatch(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/event-message.ts
var EventMessage = class _EventMessage {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsEventMessage(bb, obj) {
    return (obj || new _EventMessage()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsEventMessage(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _EventMessage()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  sessionId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  listenerId() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  callbackId() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  contentType(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  payload(index) {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.readUint8(this.bb.__vector(this.bb_pos + offset) + index) : 0;
  }
  payloadLength() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  payloadArray() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? new Uint8Array(this.bb.bytes().buffer, this.bb.bytes().byteOffset + this.bb.__vector(this.bb_pos + offset), this.bb.__vector_len(this.bb_pos + offset)) : null;
  }
  static startEventMessage(builder) {
    builder.startObject(5);
  }
  static addSessionId(builder, sessionId) {
    builder.addFieldInt32(0, sessionId, 0);
  }
  static addListenerId(builder, listenerId) {
    builder.addFieldInt32(1, listenerId, 0);
  }
  static addCallbackId(builder, callbackId) {
    builder.addFieldInt32(2, callbackId, 0);
  }
  static addContentType(builder, contentTypeOffset) {
    builder.addFieldOffset(3, contentTypeOffset, 0);
  }
  static addPayload(builder, payloadOffset) {
    builder.addFieldOffset(4, payloadOffset, 0);
  }
  static createPayloadVector(builder, data) {
    builder.startVector(1, data.length, 1);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addInt8(data[i]);
    }
    return builder.endVector();
  }
  static startPayloadVector(builder, numElems) {
    builder.startVector(1, numElems, 1);
  }
  static endEventMessage(builder) {
    const offset = builder.endObject();
    builder.requiredField(offset, 10);
    return offset;
  }
  static createEventMessage(builder, sessionId, listenerId, callbackId, contentTypeOffset, payloadOffset) {
    _EventMessage.startEventMessage(builder);
    _EventMessage.addSessionId(builder, sessionId);
    _EventMessage.addListenerId(builder, listenerId);
    _EventMessage.addCallbackId(builder, callbackId);
    _EventMessage.addContentType(builder, contentTypeOffset);
    _EventMessage.addPayload(builder, payloadOffset);
    return _EventMessage.endEventMessage(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/element-id-result.ts
var ElementIdResult = class _ElementIdResult {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsElementIdResult(bb, obj) {
    return (obj || new _ElementIdResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsElementIdResult(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ElementIdResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  value() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  static startElementIdResult(builder) {
    builder.startObject(1);
  }
  static addValue(builder, value) {
    builder.addFieldInt32(0, value, 0);
  }
  static endElementIdResult(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createElementIdResult(builder, value) {
    _ElementIdResult.startElementIdResult(builder);
    _ElementIdResult.addValue(builder, value);
    return _ElementIdResult.endElementIdResult(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/element-ids-result.ts
var ElementIdsResult = class _ElementIdsResult {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsElementIdsResult(bb, obj) {
    return (obj || new _ElementIdsResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsElementIdsResult(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ElementIdsResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  values(index) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb.__vector(this.bb_pos + offset) + index * 4) : 0;
  }
  valuesLength() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  valuesArray() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? new Uint32Array(this.bb.bytes().buffer, this.bb.bytes().byteOffset + this.bb.__vector(this.bb_pos + offset), this.bb.__vector_len(this.bb_pos + offset)) : null;
  }
  static startElementIdsResult(builder) {
    builder.startObject(1);
  }
  static addValues(builder, valuesOffset) {
    builder.addFieldOffset(0, valuesOffset, 0);
  }
  static createValuesVector(builder, data) {
    builder.startVector(4, data.length, 4);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addInt32(data[i]);
    }
    return builder.endVector();
  }
  static startValuesVector(builder, numElems) {
    builder.startVector(4, numElems, 4);
  }
  static endElementIdsResult(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createElementIdsResult(builder, valuesOffset) {
    _ElementIdsResult.startElementIdsResult(builder);
    _ElementIdsResult.addValues(builder, valuesOffset);
    return _ElementIdsResult.endElementIdsResult(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/number-result.ts
var NumberResult = class _NumberResult {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsNumberResult(bb, obj) {
    return (obj || new _NumberResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsNumberResult(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _NumberResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  value() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readFloat64(this.bb_pos + offset) : 0;
  }
  static startNumberResult(builder) {
    builder.startObject(1);
  }
  static addValue(builder, value) {
    builder.addFieldFloat64(0, value, 0);
  }
  static endNumberResult(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createNumberResult(builder, value) {
    _NumberResult.startNumberResult(builder);
    _NumberResult.addValue(builder, value);
    return _NumberResult.endNumberResult(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/string-result.ts
var StringResult = class _StringResult {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsStringResult(bb, obj) {
    return (obj || new _StringResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsStringResult(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _StringResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  value(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  static startStringResult(builder) {
    builder.startObject(1);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(0, valueOffset, 0);
  }
  static endStringResult(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createStringResult(builder, valueOffset) {
    _StringResult.startStringResult(builder);
    _StringResult.addValue(builder, valueOffset);
    return _StringResult.endStringResult(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/strings-result.ts
var StringsResult = class _StringsResult {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsStringsResult(bb, obj) {
    return (obj || new _StringsResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsStringsResult(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _StringsResult()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  values(index, optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__string(this.bb.__vector(this.bb_pos + offset) + index * 4, optionalEncoding) : null;
  }
  valuesLength() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  static startStringsResult(builder) {
    builder.startObject(1);
  }
  static addValues(builder, valuesOffset) {
    builder.addFieldOffset(0, valuesOffset, 0);
  }
  static createValuesVector(builder, data) {
    builder.startVector(4, data.length, 4);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addOffset(data[i]);
    }
    return builder.endVector();
  }
  static startValuesVector(builder, numElems) {
    builder.startVector(4, numElems, 4);
  }
  static endStringsResult(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createStringsResult(builder, valuesOffset) {
    _StringsResult.startStringsResult(builder);
    _StringsResult.addValues(builder, valuesOffset);
    return _StringsResult.endStringsResult(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/result-item.ts
var ResultItem = class _ResultItem {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsResultItem(bb, obj) {
    return (obj || new _ResultItem()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsResultItem(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ResultItem()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  slot() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  status() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint16(this.bb_pos + offset) : 0 /* OK */;
  }
  message(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  valueKind() {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.readUint8(this.bb_pos + offset) : 0 /* NONE */;
  }
  valueType() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.readUint8(this.bb_pos + offset) : 0 /* NONE */;
  }
  value(obj) {
    const offset = this.bb.__offset(this.bb_pos, 14);
    return offset ? this.bb.__union(obj, this.bb_pos + offset) : null;
  }
  static startResultItem(builder) {
    builder.startObject(6);
  }
  static addSlot(builder, slot) {
    builder.addFieldInt32(0, slot, 0);
  }
  static addStatus(builder, status) {
    builder.addFieldInt16(1, status, 0 /* OK */);
  }
  static addMessage(builder, messageOffset) {
    builder.addFieldOffset(2, messageOffset, 0);
  }
  static addValueKind(builder, valueKind) {
    builder.addFieldInt8(3, valueKind, 0 /* NONE */);
  }
  static addValueType(builder, valueType) {
    builder.addFieldInt8(4, valueType, 0 /* NONE */);
  }
  static addValue(builder, valueOffset) {
    builder.addFieldOffset(5, valueOffset, 0);
  }
  static endResultItem(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createResultItem(builder, slot, status, messageOffset, valueKind, valueType, valueOffset) {
    _ResultItem.startResultItem(builder);
    _ResultItem.addSlot(builder, slot);
    _ResultItem.addStatus(builder, status);
    _ResultItem.addMessage(builder, messageOffset);
    _ResultItem.addValueKind(builder, valueKind);
    _ResultItem.addValueType(builder, valueType);
    _ResultItem.addValue(builder, valueOffset);
    return _ResultItem.endResultItem(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/response-batch.ts
var ResponseBatch = class _ResponseBatch {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsResponseBatch(bb, obj) {
    return (obj || new _ResponseBatch()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsResponseBatch(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _ResponseBatch()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  sessionId() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  sequence() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint32(this.bb_pos + offset) : 0;
  }
  status() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint16(this.bb_pos + offset) : 0 /* OK */;
  }
  message(optionalEncoding) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.__string(this.bb_pos + offset, optionalEncoding) : null;
  }
  results(index, obj) {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? (obj || new ResultItem()).__init(this.bb.__indirect(this.bb.__vector(this.bb_pos + offset) + index * 4), this.bb) : null;
  }
  resultsLength() {
    const offset = this.bb.__offset(this.bb_pos, 12);
    return offset ? this.bb.__vector_len(this.bb_pos + offset) : 0;
  }
  committed() {
    const offset = this.bb.__offset(this.bb_pos, 14);
    return offset ? !!this.bb.readInt8(this.bb_pos + offset) : false;
  }
  static startResponseBatch(builder) {
    builder.startObject(6);
  }
  static addSessionId(builder, sessionId) {
    builder.addFieldInt32(0, sessionId, 0);
  }
  static addSequence(builder, sequence) {
    builder.addFieldInt32(1, sequence, 0);
  }
  static addStatus(builder, status) {
    builder.addFieldInt16(2, status, 0 /* OK */);
  }
  static addMessage(builder, messageOffset) {
    builder.addFieldOffset(3, messageOffset, 0);
  }
  static addResults(builder, resultsOffset) {
    builder.addFieldOffset(4, resultsOffset, 0);
  }
  static createResultsVector(builder, data) {
    builder.startVector(4, data.length, 4);
    for (let i = data.length - 1; i >= 0; i--) {
      builder.addOffset(data[i]);
    }
    return builder.endVector();
  }
  static startResultsVector(builder, numElems) {
    builder.startVector(4, numElems, 4);
  }
  static addCommitted(builder, committed) {
    builder.addFieldInt8(5, +committed, 0);
  }
  static endResponseBatch(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static createResponseBatch(builder, sessionId, sequence, status, messageOffset, resultsOffset, committed) {
    _ResponseBatch.startResponseBatch(builder);
    _ResponseBatch.addSessionId(builder, sessionId);
    _ResponseBatch.addSequence(builder, sequence);
    _ResponseBatch.addStatus(builder, status);
    _ResponseBatch.addMessage(builder, messageOffset);
    _ResponseBatch.addResults(builder, resultsOffset);
    _ResponseBatch.addCommitted(builder, committed);
    return _ResponseBatch.endResponseBatch(builder);
  }
};

// ../../protocol/generated/typescript/lynx/element-bridge/v2/envelope.ts
var Envelope = class _Envelope {
  constructor() {
    __publicField(this, "bb", null);
    __publicField(this, "bb_pos", 0);
  }
  __init(i, bb) {
    this.bb_pos = i;
    this.bb = bb;
    return this;
  }
  static getRootAsEnvelope(bb, obj) {
    return (obj || new _Envelope()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static getSizePrefixedRootAsEnvelope(bb, obj) {
    bb.setPosition(bb.position() + SIZE_PREFIX_LENGTH);
    return (obj || new _Envelope()).__init(bb.readInt32(bb.position()) + bb.position(), bb);
  }
  static bufferHasIdentifier(bb) {
    return bb.__has_identifier("LEB2");
  }
  version() {
    const offset = this.bb.__offset(this.bb_pos, 4);
    return offset ? this.bb.readUint16(this.bb_pos + offset) : 2;
  }
  channel() {
    const offset = this.bb.__offset(this.bb_pos, 6);
    return offset ? this.bb.readUint8(this.bb_pos + offset) : 0 /* NONE */;
  }
  messageType() {
    const offset = this.bb.__offset(this.bb_pos, 8);
    return offset ? this.bb.readUint8(this.bb_pos + offset) : 0 /* NONE */;
  }
  message(obj) {
    const offset = this.bb.__offset(this.bb_pos, 10);
    return offset ? this.bb.__union(obj, this.bb_pos + offset) : null;
  }
  static startEnvelope(builder) {
    builder.startObject(4);
  }
  static addVersion(builder, version) {
    builder.addFieldInt16(0, version, 2);
  }
  static addChannel(builder, channel) {
    builder.addFieldInt8(1, channel, 0 /* NONE */);
  }
  static addMessageType(builder, messageType) {
    builder.addFieldInt8(2, messageType, 0 /* NONE */);
  }
  static addMessage(builder, messageOffset) {
    builder.addFieldOffset(3, messageOffset, 0);
  }
  static endEnvelope(builder) {
    const offset = builder.endObject();
    return offset;
  }
  static finishEnvelopeBuffer(builder, offset) {
    builder.finish(offset, "LEB2");
  }
  static finishSizePrefixedEnvelopeBuffer(builder, offset) {
    builder.finish(offset, "LEB2", true);
  }
  static createEnvelope(builder, version, channel, messageType, messageOffset) {
    _Envelope.startEnvelope(builder);
    _Envelope.addVersion(builder, version);
    _Envelope.addChannel(builder, channel);
    _Envelope.addMessageType(builder, messageType);
    _Envelope.addMessage(builder, messageOffset);
    return _Envelope.endEnvelope(builder);
  }
};

// ../../protocol/generated/typescript/element_api_dispatch.ts
function decodeElementApiCommand(command, decodePayload, decodeReferences2) {
  switch (command.operationType()) {
    case 1 /* CreatePageCommand */: {
      const operation = command.operation(new CreatePageCommand());
      if (operation === null) throw new TypeError("__CreatePage command payload is missing");
      return {
        name: "__CreatePage",
        capability: "create_page",
        available: true,
        resultKind: "element_id",
        args: [{ name: "componentId", kind: "value", value: operation.componentId() }, { name: "cssId", kind: "value", value: operation.cssId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 2 /* CreateComponentCommand */: {
      const operation = command.operation(new CreateComponentCommand());
      if (operation === null) throw new TypeError("__CreateComponent command payload is missing");
      return {
        name: "__CreateComponent",
        capability: "create_component",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "componentId", kind: "value", value: operation.componentId() }, { name: "cssId", kind: "value", value: operation.cssId() }, { name: "entryName", kind: "value", value: operation.entryName() }, { name: "name", kind: "value", value: operation.name() }, { name: "path", kind: "value", value: operation.path() }, { name: "config", kind: "value", value: decodePayload(operation.config()) }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 3 /* CreateViewCommand */: {
      const operation = command.operation(new CreateViewCommand());
      if (operation === null) throw new TypeError("__CreateView command payload is missing");
      return {
        name: "__CreateView",
        capability: "create_view",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 4 /* CreateScrollViewCommand */: {
      const operation = command.operation(new CreateScrollViewCommand());
      if (operation === null) throw new TypeError("__CreateScrollView command payload is missing");
      return {
        name: "__CreateScrollView",
        capability: "create_scroll_view",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 5 /* CreateTextCommand */: {
      const operation = command.operation(new CreateTextCommand());
      if (operation === null) throw new TypeError("__CreateText command payload is missing");
      return {
        name: "__CreateText",
        capability: "create_text",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 6 /* CreateRawTextCommand */: {
      const operation = command.operation(new CreateRawTextCommand());
      if (operation === null) throw new TypeError("__CreateRawText command payload is missing");
      return {
        name: "__CreateRawText",
        capability: "create_raw_text",
        available: true,
        resultKind: "element_id",
        args: [{ name: "text", kind: "value", value: operation.text() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 7 /* CreateImageCommand */: {
      const operation = command.operation(new CreateImageCommand());
      if (operation === null) throw new TypeError("__CreateImage command payload is missing");
      return {
        name: "__CreateImage",
        capability: "create_image",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 8 /* CreateWrapperElementCommand */: {
      const operation = command.operation(new CreateWrapperElementCommand());
      if (operation === null) throw new TypeError("__CreateWrapperElement command payload is missing");
      return {
        name: "__CreateWrapperElement",
        capability: "create_wrapper_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }]
      };
    }
    case 9 /* CreateNonElementCommand */: {
      const operation = command.operation(new CreateNonElementCommand());
      if (operation === null) throw new TypeError("__CreateNonElement command payload is missing");
      return {
        name: "__CreateNonElement",
        capability: "create_non_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }]
      };
    }
    case 10 /* CreateIfCommand */: {
      const operation = command.operation(new CreateIfCommand());
      if (operation === null) throw new TypeError("__CreateIf command payload is missing");
      return {
        name: "__CreateIf",
        capability: "create_if",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 11 /* CreateForCommand */: {
      const operation = command.operation(new CreateForCommand());
      if (operation === null) throw new TypeError("__CreateFor command payload is missing");
      return {
        name: "__CreateFor",
        capability: "create_for",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 12 /* CreateBlockCommand */: {
      const operation = command.operation(new CreateBlockCommand());
      if (operation === null) throw new TypeError("__CreateBlock command payload is missing");
      return {
        name: "__CreateBlock",
        capability: "create_block",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 13 /* CreateListCommand */: {
      const operation = command.operation(new CreateListCommand());
      if (operation === null) throw new TypeError("__CreateList command payload is missing");
      return {
        name: "__CreateList",
        capability: "create_list",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }, { name: "componentAtIndex", kind: "callback", value: operation.componentAtIndex() }, { name: "enqueueComponent", kind: "callback", value: operation.enqueueComponent() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }, { name: "componentAtIndexes", kind: "callback", value: operation.componentAtIndexes() }]
      };
    }
    case 14 /* UpdateListCallbacksCommand */: {
      const operation = command.operation(new UpdateListCallbacksCommand());
      if (operation === null) throw new TypeError("__UpdateListCallbacks command payload is missing");
      return {
        name: "__UpdateListCallbacks",
        capability: "update_list_callbacks",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "componentAtIndex", kind: "callback", value: operation.componentAtIndex() }, { name: "enqueueComponent", kind: "callback", value: operation.enqueueComponent() }, { name: "componentAtIndexes", kind: "callback", value: operation.componentAtIndexes() }]
      };
    }
    case 15 /* CreateElementCommand */: {
      const operation = command.operation(new CreateElementCommand());
      if (operation === null) throw new TypeError("__CreateElement command payload is missing");
      return {
        name: "__CreateElement",
        capability: "create_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "tag", kind: "value", value: operation.tag() }, { name: "comParentUniID", kind: "value", value: operation.comParentUniId() }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 16 /* AppendElementCommand */: {
      const operation = command.operation(new AppendElementCommand());
      if (operation === null) throw new TypeError("__AppendElement command payload is missing");
      return {
        name: "__AppendElement",
        capability: "append_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parent", kind: "node", value: operation.parent() }, { name: "current", kind: "node", value: operation.current() }]
      };
    }
    case 17 /* RemoveElementCommand */: {
      const operation = command.operation(new RemoveElementCommand());
      if (operation === null) throw new TypeError("__RemoveElement command payload is missing");
      return {
        name: "__RemoveElement",
        capability: "remove_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parent", kind: "node", value: operation.parent() }, { name: "current", kind: "node", value: operation.current() }]
      };
    }
    case 18 /* InsertElementBeforeCommand */: {
      const operation = command.operation(new InsertElementBeforeCommand());
      if (operation === null) throw new TypeError("__InsertElementBefore command payload is missing");
      return {
        name: "__InsertElementBefore",
        capability: "insert_element_before",
        available: true,
        resultKind: "element_id",
        args: [{ name: "parent", kind: "node", value: operation.parent() }, { name: "current", kind: "node", value: operation.current() }, { name: "marker", kind: "node", value: operation.marker() }]
      };
    }
    case 19 /* SwapElementCommand */: {
      const operation = command.operation(new SwapElementCommand());
      if (operation === null) throw new TypeError("__SwapElement command payload is missing");
      return {
        name: "__SwapElement",
        capability: "swap_element",
        available: true,
        resultKind: "void",
        args: [{ name: "left", kind: "node", value: operation.left() }, { name: "right", kind: "node", value: operation.right() }]
      };
    }
    case 20 /* ReplaceElementCommand */: {
      const operation = command.operation(new ReplaceElementCommand());
      if (operation === null) throw new TypeError("__ReplaceElement command payload is missing");
      return {
        name: "__ReplaceElement",
        capability: "replace_element",
        available: true,
        resultKind: "void",
        args: [{ name: "newElement", kind: "node", value: operation.newElement() }, { name: "oldElement", kind: "node", value: operation.oldElement() }]
      };
    }
    case 21 /* ReplaceElementsCommand */: {
      const operation = command.operation(new ReplaceElementsCommand());
      if (operation === null) throw new TypeError("__ReplaceElements command payload is missing");
      return {
        name: "__ReplaceElements",
        capability: "replace_elements",
        available: true,
        resultKind: "void",
        args: [{ name: "parent", kind: "node", value: operation.parent() }, { name: "insertedChildren", kind: "node_or_nodes", value: decodeReferences2(operation.insertedChildren()) }, { name: "removedChildren", kind: "node_or_nodes", value: decodeReferences2(operation.removedChildren()) }]
      };
    }
    case 22 /* GetParentCommand */: {
      const operation = command.operation(new GetParentCommand());
      if (operation === null) throw new TypeError("__GetParent command payload is missing");
      return {
        name: "__GetParent",
        capability: "get_parent",
        available: true,
        resultKind: "element_id",
        args: [{ name: "current", kind: "node", value: operation.current() }]
      };
    }
    case 23 /* GetChildrenCommand */: {
      const operation = command.operation(new GetChildrenCommand());
      if (operation === null) throw new TypeError("__GetChildren command payload is missing");
      return {
        name: "__GetChildren",
        capability: "get_children",
        available: true,
        resultKind: "element_ids",
        args: [{ name: "current", kind: "node", value: operation.current() }]
      };
    }
    case 24 /* FirstElementCommand */: {
      const operation = command.operation(new FirstElementCommand());
      if (operation === null) throw new TypeError("__FirstElement command payload is missing");
      return {
        name: "__FirstElement",
        capability: "first_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "current", kind: "node", value: operation.current() }]
      };
    }
    case 25 /* LastElementCommand */: {
      const operation = command.operation(new LastElementCommand());
      if (operation === null) throw new TypeError("__LastElement command payload is missing");
      return {
        name: "__LastElement",
        capability: "last_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "current", kind: "node", value: operation.current() }]
      };
    }
    case 26 /* NextElementCommand */: {
      const operation = command.operation(new NextElementCommand());
      if (operation === null) throw new TypeError("__NextElement command payload is missing");
      return {
        name: "__NextElement",
        capability: "next_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 27 /* GetTagCommand */: {
      const operation = command.operation(new GetTagCommand());
      if (operation === null) throw new TypeError("__GetTag command payload is missing");
      return {
        name: "__GetTag",
        capability: "get_tag",
        available: true,
        resultKind: "string",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 28 /* SetAttributeCommand */: {
      const operation = command.operation(new SetAttributeCommand());
      if (operation === null) throw new TypeError("__SetAttribute command payload is missing");
      return {
        name: "__SetAttribute",
        capability: "set_attribute",
        available: true,
        resultKind: "void",
        args: [{ name: "current", kind: "node", value: operation.current() }, { name: "attrName", kind: "value", value: operation.attrName() }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 29 /* AddClassCommand */: {
      const operation = command.operation(new AddClassCommand());
      if (operation === null) throw new TypeError("__AddClass command payload is missing");
      return {
        name: "__AddClass",
        capability: "add_class",
        available: true,
        resultKind: "void",
        args: [{ name: "current", kind: "node", value: operation.current() }, { name: "className", kind: "value", value: operation.className() }]
      };
    }
    case 30 /* SetClassesCommand */: {
      const operation = command.operation(new SetClassesCommand());
      if (operation === null) throw new TypeError("__SetClasses command payload is missing");
      return {
        name: "__SetClasses",
        capability: "set_classes",
        available: true,
        resultKind: "void",
        args: [{ name: "current", kind: "node", value: operation.current() }, { name: "className", kind: "value", value: operation.className() }]
      };
    }
    case 31 /* GetClassesCommand */: {
      const operation = command.operation(new GetClassesCommand());
      if (operation === null) throw new TypeError("__GetClasses command payload is missing");
      return {
        name: "__GetClasses",
        capability: "get_classes",
        available: true,
        resultKind: "strings",
        args: [{ name: "current", kind: "node", value: operation.current() }]
      };
    }
    case 32 /* SetStaticStyleCommand */: {
      const operation = command.operation(new SetStaticStyleCommand());
      if (operation === null) throw new TypeError("__SetStaticStyle command payload is missing");
      return {
        name: "__SetStaticStyle",
        capability: "set_static_style",
        available: false,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "key", kind: "value", value: operation.key() }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 33 /* SetInlineStylesCommand */: {
      const operation = command.operation(new SetInlineStylesCommand());
      if (operation === null) throw new TypeError("__SetInlineStyles command payload is missing");
      return {
        name: "__SetInlineStyles",
        capability: "set_inline_styles",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 34 /* GetInlineStyleCommand */: {
      const operation = command.operation(new GetInlineStyleCommand());
      if (operation === null) throw new TypeError("__GetInlineStyle command payload is missing");
      return {
        name: "__GetInlineStyle",
        capability: "get_inline_style",
        available: true,
        resultKind: "string",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "propertyId", kind: "value", value: operation.propertyId() }]
      };
    }
    case 35 /* GetInlineStylesCommand */: {
      const operation = command.operation(new GetInlineStylesCommand());
      if (operation === null) throw new TypeError("__GetInlineStyles command payload is missing");
      return {
        name: "__GetInlineStyles",
        capability: "get_inline_styles",
        available: true,
        resultKind: "string",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 36 /* SetIDCommand */: {
      const operation = command.operation(new SetIDCommand());
      if (operation === null) throw new TypeError("__SetID command payload is missing");
      return {
        name: "__SetID",
        capability: "set_id",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "id", kind: "value", value: operation.id() }]
      };
    }
    case 37 /* GetIDCommand */: {
      const operation = command.operation(new GetIDCommand());
      if (operation === null) throw new TypeError("__GetID command payload is missing");
      return {
        name: "__GetID",
        capability: "get_id",
        available: true,
        resultKind: "string",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 38 /* SetCSSIdCommand */: {
      const operation = command.operation(new SetCSSIdCommand());
      if (operation === null) throw new TypeError("__SetCSSId command payload is missing");
      return {
        name: "__SetCSSId",
        capability: "set_cssid",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node_or_nodes", value: decodeReferences2(operation.node()) }, { name: "cssId", kind: "value", value: operation.cssId() }, { name: "entryName", kind: "value", value: operation.entryName() }]
      };
    }
    case 39 /* AddEventCommand */: {
      const operation = command.operation(new AddEventCommand());
      if (operation === null) throw new TypeError("__AddEvent command payload is missing");
      return {
        name: "__AddEvent",
        capability: "add_event",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "type", kind: "value", value: operation.valueType() }, { name: "name", kind: "value", value: operation.name() }, { name: "func", kind: "callback", value: operation.func() }]
      };
    }
    case 40 /* SetEventsCommand */: {
      const operation = command.operation(new SetEventsCommand());
      if (operation === null) throw new TypeError("__SetEvents command payload is missing");
      return {
        name: "__SetEvents",
        capability: "set_events",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "events", kind: "value", value: decodePayload(operation.events()) }]
      };
    }
    case 41 /* GetEventCommand */: {
      const operation = command.operation(new GetEventCommand());
      if (operation === null) throw new TypeError("__GetEvent command payload is missing");
      return {
        name: "__GetEvent",
        capability: "get_event",
        available: true,
        resultKind: "payload",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "name", kind: "value", value: operation.name() }, { name: "type", kind: "value", value: operation.valueType() }]
      };
    }
    case 42 /* GetEventsCommand */: {
      const operation = command.operation(new GetEventsCommand());
      if (operation === null) throw new TypeError("__GetEvents command payload is missing");
      return {
        name: "__GetEvents",
        capability: "get_events",
        available: true,
        resultKind: "payload",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 43 /* AddEventListenerCommand */: {
      const operation = command.operation(new AddEventListenerCommand());
      if (operation === null) throw new TypeError("__AddEventListener command payload is missing");
      return {
        name: "__AddEventListener",
        capability: "add_event_listener",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "name", kind: "value", value: operation.name() }, { name: "callback", kind: "callback", value: operation.callback() }, { name: "options", kind: "value", value: decodePayload(operation.options()) }]
      };
    }
    case 44 /* RemoveEventListenerCommand */: {
      const operation = command.operation(new RemoveEventListenerCommand());
      if (operation === null) throw new TypeError("__RemoveEventListener command payload is missing");
      return {
        name: "__RemoveEventListener",
        capability: "remove_event_listener",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "name", kind: "value", value: operation.name() }, { name: "callback", kind: "callback", value: operation.callback() }, { name: "options", kind: "value", value: decodePayload(operation.options()) }]
      };
    }
    case 45 /* RemoveEventListenersCommand */: {
      const operation = command.operation(new RemoveEventListenersCommand());
      if (operation === null) throw new TypeError("__RemoveEventListeners command payload is missing");
      return {
        name: "__RemoveEventListeners",
        capability: "remove_event_listeners",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 46 /* CreateEventCommand */: {
      const operation = command.operation(new CreateEventCommand());
      if (operation === null) throw new TypeError("__CreateEvent command payload is missing");
      return {
        name: "__CreateEvent",
        capability: "create_event",
        available: true,
        resultKind: "payload",
        args: [{ name: "type", kind: "value", value: decodePayload(operation.valueType()) }, { name: "name", kind: "value", value: operation.name() }, { name: "options", kind: "value", value: decodePayload(operation.options()) }, { name: "detail", kind: "value", value: decodePayload(operation.detail()) }]
      };
    }
    case 47 /* DispatchEventCommand */: {
      const operation = command.operation(new DispatchEventCommand());
      if (operation === null) throw new TypeError("__DispatchEvent command payload is missing");
      return {
        name: "__DispatchEvent",
        capability: "dispatch_event",
        available: true,
        resultKind: "boolean",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "event", kind: "value", value: decodePayload(operation.event()) }]
      };
    }
    case 48 /* StopPropagationCommand */: {
      const operation = command.operation(new StopPropagationCommand());
      if (operation === null) throw new TypeError("__StopPropagation command payload is missing");
      return {
        name: "__StopPropagation",
        capability: "stop_propagation",
        available: true,
        resultKind: "void",
        args: [{ name: "event", kind: "value", value: decodePayload(operation.event()) }]
      };
    }
    case 49 /* StopImmediatePropagationCommand */: {
      const operation = command.operation(new StopImmediatePropagationCommand());
      if (operation === null) throw new TypeError("__StopImmediatePropagation command payload is missing");
      return {
        name: "__StopImmediatePropagation",
        capability: "stop_immediate_propagation",
        available: true,
        resultKind: "void",
        args: [{ name: "event", kind: "value", value: decodePayload(operation.event()) }]
      };
    }
    case 50 /* AddDatasetCommand */: {
      const operation = command.operation(new AddDatasetCommand());
      if (operation === null) throw new TypeError("__AddDataset command payload is missing");
      return {
        name: "__AddDataset",
        capability: "add_dataset",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "key", kind: "value", value: operation.key() }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 51 /* SetDatasetCommand */: {
      const operation = command.operation(new SetDatasetCommand());
      if (operation === null) throw new TypeError("__SetDataset command payload is missing");
      return {
        name: "__SetDataset",
        capability: "set_dataset",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 52 /* GetDatasetCommand */: {
      const operation = command.operation(new GetDatasetCommand());
      if (operation === null) throw new TypeError("__GetDataset command payload is missing");
      return {
        name: "__GetDataset",
        capability: "get_dataset",
        available: true,
        resultKind: "payload",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 53 /* GetDataByKeyCommand */: {
      const operation = command.operation(new GetDataByKeyCommand());
      if (operation === null) throw new TypeError("__GetDataByKey command payload is missing");
      return {
        name: "__GetDataByKey",
        capability: "get_data_by_key",
        available: true,
        resultKind: "payload",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "key", kind: "value", value: operation.key() }]
      };
    }
    case 54 /* GetElementUniqueIDCommand */: {
      const operation = command.operation(new GetElementUniqueIDCommand());
      if (operation === null) throw new TypeError("__GetElementUniqueID command payload is missing");
      return {
        name: "__GetElementUniqueID",
        capability: "get_element_unique_id",
        available: true,
        resultKind: "number",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 55 /* ElementIsEqualCommand */: {
      const operation = command.operation(new ElementIsEqualCommand());
      if (operation === null) throw new TypeError("__ElementIsEqual command payload is missing");
      return {
        name: "__ElementIsEqual",
        capability: "element_is_equal",
        available: true,
        resultKind: "boolean",
        args: [{ name: "left", kind: "node", value: operation.left() }, { name: "right", kind: "node", value: operation.right() }]
      };
    }
    case 56 /* UpdateComponentIDCommand */: {
      const operation = command.operation(new UpdateComponentIDCommand());
      if (operation === null) throw new TypeError("__UpdateComponentID command payload is missing");
      return {
        name: "__UpdateComponentID",
        capability: "update_component_id",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "id", kind: "value", value: operation.id() }]
      };
    }
    case 57 /* GetComponentIDCommand */: {
      const operation = command.operation(new GetComponentIDCommand());
      if (operation === null) throw new TypeError("__GetComponentID command payload is missing");
      return {
        name: "__GetComponentID",
        capability: "get_component_id",
        available: true,
        resultKind: "string",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 58 /* UpdateComponentInfoCommand */: {
      const operation = command.operation(new UpdateComponentInfoCommand());
      if (operation === null) throw new TypeError("__UpdateComponentInfo command payload is missing");
      return {
        name: "__UpdateComponentInfo",
        capability: "update_component_info",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "params", kind: "value", value: decodePayload(operation.params()) }]
      };
    }
    case 59 /* FlushElementTreeCommand */: {
      const operation = command.operation(new FlushElementTreeCommand());
      if (operation === null) throw new TypeError("__FlushElementTree command payload is missing");
      return {
        name: "__FlushElementTree",
        capability: "flush_element_tree",
        available: true,
        resultKind: "void",
        args: [{ name: "element", kind: "node", value: operation.element() }, { name: "options", kind: "callback", value: operation.options() }]
      };
    }
    case 60 /* AsyncResolveElementCommand */: {
      const operation = command.operation(new AsyncResolveElementCommand());
      if (operation === null) throw new TypeError("__AsyncResolveElement command payload is missing");
      return {
        name: "__AsyncResolveElement",
        capability: "async_resolve_element",
        available: true,
        resultKind: "void",
        args: [{ name: "element", kind: "node", value: operation.element() }]
      };
    }
    case 61 /* AsyncResolveSubtreeCommand */: {
      const operation = command.operation(new AsyncResolveSubtreeCommand());
      if (operation === null) throw new TypeError("__AsyncResolveSubtree command payload is missing");
      return {
        name: "__AsyncResolveSubtree",
        capability: "async_resolve_subtree",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }]
      };
    }
    case 62 /* OnLifecycleEventCommand */: {
      const operation = command.operation(new OnLifecycleEventCommand());
      if (operation === null) throw new TypeError("__OnLifecycleEvent command payload is missing");
      return {
        name: "__OnLifecycleEvent",
        capability: "on_lifecycle_event",
        available: true,
        resultKind: "void",
        args: [{ name: "args", kind: "value", value: decodePayload(operation.args()) }]
      };
    }
    case 63 /* ReportErrorCommand */: {
      const operation = command.operation(new ReportErrorCommand());
      if (operation === null) throw new TypeError("_ReportError command payload is missing");
      return {
        name: "_ReportError",
        capability: "report_error",
        available: true,
        resultKind: "void",
        args: [{ name: "err", kind: "value", value: decodePayload(operation.err()) }, { name: "info", kind: "value", value: decodePayload(operation.info()) }]
      };
    }
    case 64 /* ElementFromBinaryCommand */: {
      const operation = command.operation(new ElementFromBinaryCommand());
      if (operation === null) throw new TypeError("__ElementFromBinary command payload is missing");
      return {
        name: "__ElementFromBinary",
        capability: "element_from_binary",
        available: true,
        resultKind: "element_ids",
        args: [{ name: "elementTemplateKey", kind: "value", value: operation.elementTemplateKey() }, { name: "parentComponentUniId", kind: "value", value: operation.parentComponentUniId() }]
      };
    }
    case 65 /* CreateElementTemplateCommand */: {
      const operation = command.operation(new CreateElementTemplateCommand());
      if (operation === null) throw new TypeError("__CreateElementTemplate command payload is missing");
      return {
        name: "__CreateElementTemplate",
        capability: "create_element_template",
        available: true,
        resultKind: "element_id",
        args: [{ name: "templateKey", kind: "value", value: operation.templateKey() }, { name: "bundleUrl", kind: "value", value: operation.bundleUrl() }, { name: "attributeSlots", kind: "value", value: decodePayload(operation.attributeSlots()) }, { name: "elementSlots", kind: "node_or_nodes", value: decodeReferences2(operation.elementSlots()) }, { name: "uid", kind: "value", value: decodePayload(operation.uid()) }, { name: "options", kind: "value", value: decodePayload(operation.options()) }]
      };
    }
    case 66 /* CreateTypedElementTemplateCommand */: {
      const operation = command.operation(new CreateTypedElementTemplateCommand());
      if (operation === null) throw new TypeError("__CreateTypedElementTemplate command payload is missing");
      return {
        name: "__CreateTypedElementTemplate",
        capability: "create_typed_element_template",
        available: true,
        resultKind: "element_id",
        args: [{ name: "tag", kind: "value", value: operation.tag() }, { name: "attributes", kind: "value", value: decodePayload(operation.attributes()) }, { name: "elementSlots", kind: "node_or_nodes", value: decodeReferences2(operation.elementSlots()) }, { name: "uid", kind: "value", value: decodePayload(operation.uid()) }, { name: "options", kind: "value", value: decodePayload(operation.options()) }]
      };
    }
    case 67 /* SetAttributeOfElementTemplateCommand */: {
      const operation = command.operation(new SetAttributeOfElementTemplateCommand());
      if (operation === null) throw new TypeError("__SetAttributeOfElementTemplate command payload is missing");
      return {
        name: "__SetAttributeOfElementTemplate",
        capability: "set_attribute_of_element_template",
        available: true,
        resultKind: "void",
        args: [{ name: "templateInstance", kind: "node", value: operation.templateInstance() }, { name: "attrSlotIndex", kind: "value", value: operation.attrSlotIndex() }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 68 /* InsertNodeToElementTemplateCommand */: {
      const operation = command.operation(new InsertNodeToElementTemplateCommand());
      if (operation === null) throw new TypeError("__InsertNodeToElementTemplate command payload is missing");
      return {
        name: "__InsertNodeToElementTemplate",
        capability: "insert_node_to_element_template",
        available: true,
        resultKind: "void",
        args: [{ name: "templateInstance", kind: "node", value: operation.templateInstance() }, { name: "elementSlotIndex", kind: "value", value: operation.elementSlotIndex() }, { name: "child", kind: "node", value: operation.child() }, { name: "referenceChild", kind: "node_or_nodes", value: operation.referenceChild() }]
      };
    }
    case 69 /* RemoveNodeFromElementTemplateCommand */: {
      const operation = command.operation(new RemoveNodeFromElementTemplateCommand());
      if (operation === null) throw new TypeError("__RemoveNodeFromElementTemplate command payload is missing");
      return {
        name: "__RemoveNodeFromElementTemplate",
        capability: "remove_node_from_element_template",
        available: true,
        resultKind: "void",
        args: [{ name: "templateInstance", kind: "node", value: operation.templateInstance() }, { name: "elementSlotIndex", kind: "value", value: operation.elementSlotIndex() }, { name: "child", kind: "node", value: operation.child() }]
      };
    }
    case 70 /* SerializeElementTemplateCommand */: {
      const operation = command.operation(new SerializeElementTemplateCommand());
      if (operation === null) throw new TypeError("__SerializeElementTemplate command payload is missing");
      return {
        name: "__SerializeElementTemplate",
        capability: "serialize_element_template",
        available: true,
        resultKind: "payload",
        args: [{ name: "templateInstance", kind: "node", value: operation.templateInstance() }]
      };
    }
    case 71 /* GetTemplatePartsCommand */: {
      const operation = command.operation(new GetTemplatePartsCommand());
      if (operation === null) throw new TypeError("__GetTemplateParts command payload is missing");
      return {
        name: "__GetTemplateParts",
        capability: "get_template_parts",
        available: true,
        resultKind: "element_id",
        args: [{ name: "ele", kind: "node", value: operation.ele() }]
      };
    }
    case 72 /* CloneElementCommand */: {
      const operation = command.operation(new CloneElementCommand());
      if (operation === null) throw new TypeError("__CloneElement command payload is missing");
      return {
        name: "__CloneElement",
        capability: "clone_element",
        available: true,
        resultKind: "element_id",
        args: [{ name: "ele", kind: "node", value: operation.ele() }, { name: "options", kind: "value", value: decodePayload(operation.options()) }]
      };
    }
    case 73 /* IsTemplateElementCommand */: {
      const operation = command.operation(new IsTemplateElementCommand());
      if (operation === null) throw new TypeError("__IsTemplateElement command payload is missing");
      return {
        name: "__IsTemplateElement",
        capability: "is_template_element",
        available: true,
        resultKind: "boolean",
        args: [{ name: "ele", kind: "node", value: operation.ele() }]
      };
    }
    case 74 /* MarkTemplateElementCommand */: {
      const operation = command.operation(new MarkTemplateElementCommand());
      if (operation === null) throw new TypeError("__MarkTemplateElement command payload is missing");
      return {
        name: "__MarkTemplateElement",
        capability: "mark_template_element",
        available: true,
        resultKind: "void",
        args: [{ name: "ele", kind: "node", value: operation.ele() }]
      };
    }
    case 75 /* MarkPartElementCommand */: {
      const operation = command.operation(new MarkPartElementCommand());
      if (operation === null) throw new TypeError("__MarkPartElement command payload is missing");
      return {
        name: "__MarkPartElement",
        capability: "mark_part_element",
        available: true,
        resultKind: "void",
        args: [{ name: "ele", kind: "node", value: operation.ele() }, { name: "key", kind: "value", value: operation.key() }]
      };
    }
    case 76 /* QuerySelectorCommand */: {
      const operation = command.operation(new QuerySelectorCommand());
      if (operation === null) throw new TypeError("__QuerySelector command payload is missing");
      return {
        name: "__QuerySelector",
        capability: "query_selector",
        available: true,
        resultKind: "element_id",
        args: [{ name: "root", kind: "node", value: operation.root() }, { name: "cssSelector", kind: "value", value: operation.cssSelector() }, { name: "params", kind: "value", value: decodePayload(operation.params()) }]
      };
    }
    case 77 /* QuerySelectorAllCommand */: {
      const operation = command.operation(new QuerySelectorAllCommand());
      if (operation === null) throw new TypeError("__QuerySelectorAll command payload is missing");
      return {
        name: "__QuerySelectorAll",
        capability: "query_selector_all",
        available: true,
        resultKind: "element_ids",
        args: [{ name: "root", kind: "node", value: operation.root() }, { name: "cssSelector", kind: "value", value: operation.cssSelector() }, { name: "params", kind: "value", value: decodePayload(operation.params()) }]
      };
    }
    case 78 /* AddConfigCommand */: {
      const operation = command.operation(new AddConfigCommand());
      if (operation === null) throw new TypeError("__AddConfig command payload is missing");
      return {
        name: "__AddConfig",
        capability: "add_config",
        available: true,
        resultKind: "void",
        args: [{ name: "ele", kind: "node", value: operation.ele() }, { name: "key", kind: "value", value: operation.key() }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 79 /* SetConfigCommand */: {
      const operation = command.operation(new SetConfigCommand());
      if (operation === null) throw new TypeError("__SetConfig command payload is missing");
      return {
        name: "__SetConfig",
        capability: "set_config",
        available: true,
        resultKind: "void",
        args: [{ name: "ele", kind: "node", value: operation.ele() }, { name: "config", kind: "value", value: decodePayload(operation.config()) }]
      };
    }
    case 80 /* GetConfigCommand */: {
      const operation = command.operation(new GetConfigCommand());
      if (operation === null) throw new TypeError("__GetConfig command payload is missing");
      return {
        name: "__GetConfig",
        capability: "get_config",
        available: true,
        resultKind: "payload",
        args: [{ name: "ele", kind: "node", value: operation.ele() }]
      };
    }
    case 81 /* QueryComponentCommand */: {
      const operation = command.operation(new QueryComponentCommand());
      if (operation === null) throw new TypeError("__QueryComponent command payload is missing");
      return {
        name: "__QueryComponent",
        capability: "query_component",
        available: true,
        resultKind: "payload",
        args: [{ name: "source", kind: "value", value: operation.source() }, { name: "callback", kind: "callback", value: operation.callback() }]
      };
    }
    case 82 /* AddInlineStyleCommand */: {
      const operation = command.operation(new AddInlineStyleCommand());
      if (operation === null) throw new TypeError("__AddInlineStyle command payload is missing");
      return {
        name: "__AddInlineStyle",
        capability: "add_inline_style",
        available: true,
        resultKind: "void",
        args: [{ name: "e", kind: "node", value: operation.e() }, { name: "key", kind: "value", value: decodePayload(operation.key()) }, { name: "value", kind: "value", value: decodePayload(operation.value()) }]
      };
    }
    case 83 /* GetAttributeByNameCommand */: {
      const operation = command.operation(new GetAttributeByNameCommand());
      if (operation === null) throw new TypeError("__GetAttributeByName command payload is missing");
      return {
        name: "__GetAttributeByName",
        capability: "get_attribute_by_name",
        available: true,
        resultKind: "payload",
        args: [{ name: "e", kind: "node", value: operation.e() }, { name: "name", kind: "value", value: operation.name() }]
      };
    }
    case 84 /* GetAttributeNamesCommand */: {
      const operation = command.operation(new GetAttributeNamesCommand());
      if (operation === null) throw new TypeError("__GetAttributeNames command payload is missing");
      return {
        name: "__GetAttributeNames",
        capability: "get_attribute_names",
        available: true,
        resultKind: "strings",
        args: [{ name: "e", kind: "node", value: operation.e() }]
      };
    }
    case 85 /* GetAttributesCommand */: {
      const operation = command.operation(new GetAttributesCommand());
      if (operation === null) throw new TypeError("__GetAttributes command payload is missing");
      return {
        name: "__GetAttributes",
        capability: "get_attributes",
        available: true,
        resultKind: "payload",
        args: [{ name: "e", kind: "node", value: operation.e() }]
      };
    }
    case 86 /* GetPageElementCommand */: {
      const operation = command.operation(new GetPageElementCommand());
      if (operation === null) throw new TypeError("__GetPageElement command payload is missing");
      return {
        name: "__GetPageElement",
        capability: "get_page_element",
        available: true,
        resultKind: "element_id",
        args: []
      };
    }
    case 87 /* InvokeUIMethodCommand */: {
      const operation = command.operation(new InvokeUIMethodCommand());
      if (operation === null) throw new TypeError("__InvokeUIMethod command payload is missing");
      return {
        name: "__InvokeUIMethod",
        capability: "invoke_uimethod",
        available: true,
        resultKind: "element_ids",
        args: [{ name: "e", kind: "node", value: operation.e() }, { name: "method", kind: "value", value: operation.method() }, { name: "params", kind: "value", value: decodePayload(operation.params()) }, { name: "callback", kind: "callback", value: operation.callback() }]
      };
    }
    case 88 /* LoadLepusChunkCommand */: {
      const operation = command.operation(new LoadLepusChunkCommand());
      if (operation === null) throw new TypeError("__LoadLepusChunk command payload is missing");
      return {
        name: "__LoadLepusChunk",
        capability: "load_lepus_chunk",
        available: true,
        resultKind: "void",
        args: [{ name: "name", kind: "value", value: operation.name() }, { name: "cfg", kind: "value", value: decodePayload(operation.cfg()) }]
      };
    }
    case 89 /* CreateGestureDetectorCommand */: {
      const operation = command.operation(new CreateGestureDetectorCommand());
      if (operation === null) throw new TypeError("__CreateGestureDetector command payload is missing");
      return {
        name: "__CreateGestureDetector",
        capability: "create_gesture_detector",
        available: false,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "gestureID", kind: "value", value: operation.gestureId() }, { name: "gestureType", kind: "value", value: operation.gestureType() }, { name: "config", kind: "value", value: decodePayload(operation.config()) }, { name: "relationMap", kind: "value", value: Array.from({ length: operation.relationMapLength() }, (_, index) => operation.relationMap(index)) }]
      };
    }
    case 90 /* SetGestureDetectorCommand */: {
      const operation = command.operation(new SetGestureDetectorCommand());
      if (operation === null) throw new TypeError("__SetGestureDetector command payload is missing");
      return {
        name: "__SetGestureDetector",
        capability: "set_gesture_detector",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "gestureID", kind: "value", value: operation.gestureId() }, { name: "gestureType", kind: "value", value: operation.gestureType() }, { name: "config", kind: "value", value: decodePayload(operation.config()) }, { name: "relationMap", kind: "value", value: Array.from({ length: operation.relationMapLength() }, (_, index) => operation.relationMap(index)) }]
      };
    }
    case 91 /* RemoveGestureDetectorCommand */: {
      const operation = command.operation(new RemoveGestureDetectorCommand());
      if (operation === null) throw new TypeError("__RemoveGestureDetector command payload is missing");
      return {
        name: "__RemoveGestureDetector",
        capability: "remove_gesture_detector",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "gestureID", kind: "value", value: operation.gestureId() }]
      };
    }
    case 92 /* SetGestureStateCommand */: {
      const operation = command.operation(new SetGestureStateCommand());
      if (operation === null) throw new TypeError("__SetGestureState command payload is missing");
      return {
        name: "__SetGestureState",
        capability: "set_gesture_state",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "gestureID", kind: "value", value: operation.gestureId() }, { name: "state", kind: "value", value: operation.state() }]
      };
    }
    case 93 /* ConsumeGestureCommand */: {
      const operation = command.operation(new ConsumeGestureCommand());
      if (operation === null) throw new TypeError("__ConsumeGesture command payload is missing");
      return {
        name: "__ConsumeGesture",
        capability: "consume_gesture",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "gestureID", kind: "value", value: operation.gestureId() }, { name: "options", kind: "value", value: decodePayload(operation.options()) }]
      };
    }
    case 94 /* GeneratePipelineOptionsCommand */: {
      const operation = command.operation(new GeneratePipelineOptionsCommand());
      if (operation === null) throw new TypeError("__GeneratePipelineOptions command payload is missing");
      return {
        name: "__GeneratePipelineOptions",
        capability: "generate_pipeline_options",
        available: false,
        resultKind: "payload",
        args: []
      };
    }
    case 95 /* OnPipelineStartCommand */: {
      const operation = command.operation(new OnPipelineStartCommand());
      if (operation === null) throw new TypeError("__OnPipelineStart command payload is missing");
      return {
        name: "__OnPipelineStart",
        capability: "on_pipeline_start",
        available: false,
        resultKind: "void",
        args: [{ name: "pipeLineId", kind: "value", value: operation.pipeLineId() }, { name: "pipeLineOrigin", kind: "value", value: operation.pipeLineOrigin() }]
      };
    }
    case 96 /* BindPipelineIDWithTimingFlagCommand */: {
      const operation = command.operation(new BindPipelineIDWithTimingFlagCommand());
      if (operation === null) throw new TypeError("__BindPipelineIDWithTimingFlag command payload is missing");
      return {
        name: "__BindPipelineIDWithTimingFlag",
        capability: "bind_pipeline_idwith_timing_flag",
        available: false,
        resultKind: "void",
        args: [{ name: "pipeLineId", kind: "value", value: operation.pipeLineId() }, { name: "timingFlag", kind: "value", value: operation.timingFlag() }]
      };
    }
    case 97 /* MarkTimingCommand */: {
      const operation = command.operation(new MarkTimingCommand());
      if (operation === null) throw new TypeError("__MarkTiming command payload is missing");
      return {
        name: "__MarkTiming",
        capability: "mark_timing",
        available: false,
        resultKind: "void",
        args: [{ name: "pipeLineId", kind: "value", value: operation.pipeLineId() }, { name: "timingFlag", kind: "value", value: operation.timingFlag() }]
      };
    }
    case 98 /* AddTimingListenerCommand */: {
      const operation = command.operation(new AddTimingListenerCommand());
      if (operation === null) throw new TypeError("__AddTimingListener command payload is missing");
      return {
        name: "__AddTimingListener",
        capability: "add_timing_listener",
        available: false,
        resultKind: "void",
        args: []
      };
    }
    case 99 /* SetLepusInitDataCommand */: {
      const operation = command.operation(new SetLepusInitDataCommand());
      if (operation === null) throw new TypeError("__SetLepusInitData command payload is missing");
      return {
        name: "__SetLepusInitData",
        capability: "set_lepus_init_data",
        available: true,
        resultKind: "void",
        args: [{ name: "initData", kind: "value", value: decodePayload(operation.initData()) }]
      };
    }
    case 100 /* GetElementByUniqueIDCommand */: {
      const operation = command.operation(new GetElementByUniqueIDCommand());
      if (operation === null) throw new TypeError("__GetElementByUniqueID command payload is missing");
      return {
        name: "__GetElementByUniqueID",
        capability: "get_element_by_unique_id",
        available: true,
        resultKind: "element_id",
        args: [{ name: "elementId", kind: "value", value: operation.elementId() }]
      };
    }
    case 101 /* UpdateIfNodeIndexCommand */: {
      const operation = command.operation(new UpdateIfNodeIndexCommand());
      if (operation === null) throw new TypeError("__UpdateIfNodeIndex command payload is missing");
      return {
        name: "__UpdateIfNodeIndex",
        capability: "update_if_node_index",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "ifIndex", kind: "value", value: operation.ifIndex() }]
      };
    }
    case 102 /* UpdateForChildCountCommand */: {
      const operation = command.operation(new UpdateForChildCountCommand());
      if (operation === null) throw new TypeError("__UpdateForChildCount command payload is missing");
      return {
        name: "__UpdateForChildCount",
        capability: "update_for_child_count",
        available: true,
        resultKind: "void",
        args: [{ name: "node", kind: "node", value: operation.node() }, { name: "childCount", kind: "value", value: operation.childCount() }]
      };
    }
    case 103 /* CreateStyleObjectCommand */: {
      const operation = command.operation(new CreateStyleObjectCommand());
      if (operation === null) throw new TypeError("__CreateStyleObject command payload is missing");
      return {
        name: "__CreateStyleObject",
        capability: "create_style_object",
        available: true,
        resultKind: "payload",
        args: [{ name: "styleObject", kind: "value", value: decodePayload(operation.styleObject()) }]
      };
    }
    case 104 /* SetStyleObjectCommand */: {
      const operation = command.operation(new SetStyleObjectCommand());
      if (operation === null) throw new TypeError("__SetStyleObject command payload is missing");
      return {
        name: "__SetStyleObject",
        capability: "set_style_object",
        available: true,
        resultKind: "void",
        args: [{ name: "elementRef", kind: "node", value: operation.elementRef() }, { name: "styleObjects", kind: "value", value: decodePayload(operation.styleObjects()) }]
      };
    }
    case 105 /* UpdateStyleObjectCommand */: {
      const operation = command.operation(new UpdateStyleObjectCommand());
      if (operation === null) throw new TypeError("__UpdateStyleObject command payload is missing");
      return {
        name: "__UpdateStyleObject",
        capability: "update_style_object",
        available: true,
        resultKind: "void",
        args: [{ name: "styleObjectRef", kind: "value", value: decodePayload(operation.styleObjectRef()) }, { name: "styleObject", kind: "value", value: decodePayload(operation.styleObject()) }]
      };
    }
    case 106 /* ElementAnimateCommand */: {
      const operation = command.operation(new ElementAnimateCommand());
      if (operation === null) throw new TypeError("__ElementAnimate command payload is missing");
      return {
        name: "__ElementAnimate",
        capability: "element_animate",
        available: true,
        resultKind: "void",
        args: [{ name: "element", kind: "node", value: operation.element() }, { name: "args", kind: "value", value: decodePayload(operation.args()) }]
      };
    }
    case 107 /* CreateFrameCommand */: {
      const operation = command.operation(new CreateFrameCommand());
      if (operation === null) throw new TypeError("__CreateFrame command payload is missing");
      return {
        name: "__CreateFrame",
        capability: "create_frame",
        available: true,
        resultKind: "element_id",
        args: [{ name: "comParentUniID", kind: "value", value: operation.comParentUniId() }, { name: "options", kind: "value", value: decodePayload(operation.options()) }]
      };
    }
    default:
      throw new TypeError(`unknown Element command ${command.operationType()}`);
  }
}

// src/wire.mts
var PROTOCOL_VERSION = 2;
var NULL_CONTENT_TYPE = "application/vnd.lynx-element-bridge.null";
var TEXT_CONTENT_TYPE = "text/plain;charset=utf-8";
function normalizeByteArray(input) {
  if (input instanceof Uint8Array) {
    return input;
  }
  if (input instanceof ArrayBuffer) {
    return new Uint8Array(input);
  }
  if (input === null || input === void 0) {
    throw new TypeError("response must be a ByteArray");
  }
  const length = input.length;
  if (!Number.isInteger(length) || length < 0) {
    throw new TypeError("response ByteArray has no readable length");
  }
  const bytes = new Uint8Array(length);
  for (let index = 0; index < bytes.length; index += 1) {
    const byte = input[index];
    if (!Number.isInteger(byte) || byte < 0 || byte > 255) {
      throw new TypeError(`response ByteArray byte ${index} is invalid`);
    }
    bytes[index] = byte;
  }
  return bytes;
}
function decodeBridgeEnvelope(input, rootId) {
  const bytes = normalizeByteArray(input);
  const buffer = new ByteBuffer(bytes);
  if (!Envelope.bufferHasIdentifier(buffer)) {
    throw new TypeError("response is not a LEB2 FlatBuffer");
  }
  const envelope = Envelope.getRootAsEnvelope(buffer);
  if (envelope.version() !== PROTOCOL_VERSION) {
    throw new TypeError(`response.version must be ${PROTOCOL_VERSION}`);
  }
  if (envelope.channel() === 1 /* COMMAND */ && envelope.messageType() === 2 /* CommandBatch */) {
    const batch = envelope.message(new CommandBatch());
    if (batch === null || !batch.finalCommit()) {
      throw new TypeError("command batch must end at a final commit boundary");
    }
    const operations = [];
    for (let index = 0; index < batch.commandsLength(); index += 1) {
      const command = batch.commands(index);
      if (command === null) {
        throw new TypeError(`command ${index} is missing`);
      }
      operations.push(decodeCommand(command));
    }
    operations.push({ op: "flush", root: rootId });
    return {
      version: PROTOCOL_VERSION,
      ok: true,
      operations,
      session: batch.sessionId(),
      sequence: batch.sequence()
    };
  }
  if (envelope.channel() === 2 /* RESULT */ && envelope.messageType() === 4 /* ResponseBatch */) {
    const response = envelope.message(new ResponseBatch());
    if (response === null) {
      throw new TypeError("result envelope has no ResponseBatch");
    }
    return {
      version: PROTOCOL_VERSION,
      ok: response.status() === 0 /* OK */,
      status: response.status(),
      error: response.message() || "native bridge failure",
      operations: [],
      results: Array.from(
        { length: response.resultsLength() },
        (_, index) => {
          const result = response.results(index);
          if (result === null) throw new TypeError(`result ${index} is missing`);
          return __spreadValues({
            slot: result.slot(),
            status: result.status(),
            message: result.message() || void 0
          }, decodeHostResultValue(result));
        }
      ),
      session: response.sessionId(),
      sequence: response.sequence()
    };
  }
  if (envelope.channel() === 3 /* EVENT */ && envelope.messageType() === 5 /* EventMessage */) {
    const event = envelope.message(new EventMessage());
    if (event === null) {
      throw new TypeError("event envelope has no EventMessage");
    }
    return {
      version: PROTOCOL_VERSION,
      ok: true,
      operations: [],
      session: event.sessionId(),
      sequence: 0,
      event: {
        listener: event.listenerId(),
        callback: event.callbackId(),
        contentType: requiredString(event.contentType(), "EventMessage.contentType"),
        payload: event.payloadArray() || new Uint8Array()
      }
    };
  }
  throw new TypeError("envelope channel and message do not match");
}
function decodeHostResultValue(result) {
  switch (result.valueKind()) {
    case 0 /* NONE */:
      return { resultKind: "void" };
    case 1 /* ELEMENT_ID */: {
      const value = result.value(new ElementIdResult());
      return { resultKind: "element_id", value: value == null ? void 0 : value.value() };
    }
    case 2 /* ELEMENT_IDS */: {
      const value = result.value(new ElementIdsResult());
      return {
        resultKind: "element_ids",
        value: Array.from({ length: (value == null ? void 0 : value.valuesLength()) || 0 }, (_, index) => value == null ? void 0 : value.values(index))
      };
    }
    case 3 /* STRING */: {
      const value = result.value(new StringResult());
      return { resultKind: "string", value: value == null ? void 0 : value.value() };
    }
    case 4 /* STRINGS */: {
      const value = result.value(new StringsResult());
      return {
        resultKind: "strings",
        value: Array.from({ length: (value == null ? void 0 : value.valuesLength()) || 0 }, (_, index) => value == null ? void 0 : value.values(index))
      };
    }
    case 5 /* BOOLEAN */: {
      const value = result.value(new BooleanResult());
      return { resultKind: "boolean", value: value == null ? void 0 : value.value() };
    }
    case 6 /* NUMBER */: {
      const value = result.value(new NumberResult());
      return { resultKind: "number", value: value == null ? void 0 : value.value() };
    }
    case 7 /* PAYLOAD */: {
      const value = result.value(new Payload());
      return { resultKind: "payload", value: decodePayloadValue(value) };
    }
    default:
      throw new TypeError(`unsupported result kind ${result.valueKind()}`);
  }
}
function decodeCommand(command) {
  switch (command.operationType()) {
    case 15 /* CreateElementCommand */: {
      const operation = command.operation(new CreateElementCommand());
      return {
        op: "create_element",
        node: command.resultNodeId(),
        tag: requiredString(operation == null ? void 0 : operation.tag(), "CreateElement.tag")
      };
    }
    case 6 /* CreateRawTextCommand */: {
      const operation = command.operation(new CreateRawTextCommand());
      return {
        op: "create_text",
        node: command.resultNodeId(),
        text: requiredString(operation == null ? void 0 : operation.text(), "CreateRawText.text")
      };
    }
    case 16 /* AppendElementCommand */: {
      const operation = command.operation(new AppendElementCommand());
      return { op: "insert_before", parent: operation == null ? void 0 : operation.parent(), child: operation == null ? void 0 : operation.current(), reference: null };
    }
    case 18 /* InsertElementBeforeCommand */: {
      const operation = command.operation(new InsertElementBeforeCommand());
      return {
        op: "insert_before",
        parent: operation == null ? void 0 : operation.parent(),
        child: operation == null ? void 0 : operation.current(),
        reference: operation == null ? void 0 : operation.marker()
      };
    }
    case 17 /* RemoveElementCommand */: {
      const operation = command.operation(new RemoveElementCommand());
      return { op: "remove", parent: operation == null ? void 0 : operation.parent(), child: operation == null ? void 0 : operation.current() };
    }
    case 108 /* ReleaseElementCommand */: {
      const operation = command.operation(new ReleaseElementCommand());
      return { op: "destroy_node", node: operation == null ? void 0 : operation.node() };
    }
    case 28 /* SetAttributeCommand */: {
      const operation = command.operation(new SetAttributeCommand());
      const payload = operation == null ? void 0 : operation.value();
      return {
        op: "set_attribute",
        node: operation == null ? void 0 : operation.current(),
        name: requiredString(operation == null ? void 0 : operation.attrName(), "SetAttribute.attrName"),
        value: decodeAttribute(payload)
      };
    }
    case 43 /* AddEventListenerCommand */: {
      const operation = command.operation(new AddEventListenerCommand());
      return {
        op: "add_event_listener",
        node: operation == null ? void 0 : operation.node(),
        listener: command.listenerId(),
        callback: operation == null ? void 0 : operation.callback(),
        name: requiredString(operation == null ? void 0 : operation.name(), "AddEventListener.name")
      };
    }
    case 44 /* RemoveEventListenerCommand */: {
      const operation = command.operation(new RemoveEventListenerCommand());
      return {
        op: "remove_event_listener",
        node: operation == null ? void 0 : operation.node(),
        listener: command.listenerId(),
        callback: operation == null ? void 0 : operation.callback(),
        name: requiredString(operation == null ? void 0 : operation.name(), "RemoveEventListener.name")
      };
    }
    case 27 /* GetTagCommand */: {
      const operation = command.operation(new GetTagCommand());
      return { op: "get_tag", node: operation == null ? void 0 : operation.node(), result_slot: command.resultSlot() };
    }
    default: {
      const operation = decodeElementApiCommand(command, decodePayloadValue, decodeReferences);
      return __spreadProps(__spreadValues({
        op: "element_api"
      }, operation), {
        result_slot: command.resultSlot(),
        result_node: command.resultNodeId(),
        result_nodes: Array.from(
          { length: command.resultNodeIdsLength() },
          (_, index) => command.resultNodeIds(index)
        ),
        listener: command.listenerId()
      });
    }
  }
}
function requiredString(value, field) {
  if (typeof value !== "string") {
    throw new TypeError(`${field} is missing`);
  }
  return value;
}
function decodeAttribute(payload) {
  if (payload === null || payload === void 0) {
    throw new TypeError("SetAttribute.value is missing");
  }
  const contentType = payload.contentType();
  if (contentType === NULL_CONTENT_TYPE) {
    return null;
  }
  if (contentType !== TEXT_CONTENT_TYPE) {
    throw new TypeError(`unsupported attribute content type ${contentType}`);
  }
  return new TextDecoder().decode(payload.bytesArray() || new Uint8Array());
}
function decodePayloadValue(payload) {
  if (!(payload instanceof Payload)) {
    return null;
  }
  const contentType = payload.contentType();
  const bytes = payload.bytesArray() || new Uint8Array();
  if (contentType === NULL_CONTENT_TYPE) return null;
  if (contentType === TEXT_CONTENT_TYPE) return new TextDecoder().decode(bytes);
  if (contentType === "application/json") return JSON.parse(new TextDecoder().decode(bytes));
  return { contentType, bytes };
}
function decodeReferences(references) {
  if (references === null || typeof references !== "object") return null;
  const value = references;
  if (value.cardinality() === 0) return null;
  if (value.cardinality() === 1) return value.one();
  return Array.from({ length: value.manyLength() }, (_, index) => value.many(index));
}
function encodeTestBatch(operations, root = 1) {
  const builder = new Builder(1024);
  const offsets = operations.filter((operation) => operation.op !== "flush").map((operation) => encodeTestCommand(builder, operation));
  const commands = CommandBatch.createCommandsVector(builder, offsets);
  const batch = CommandBatch.createCommandBatch(builder, 1, 1, commands, true);
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    1 /* COMMAND */,
    2 /* CommandBatch */,
    batch
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  void root;
  return builder.asUint8Array();
}
function encodeTestFailure(status, error) {
  const builder = new Builder(256);
  const message = builder.createString(error);
  const response = ResponseBatch.createResponseBatch(builder, 0, 0, status, message, 0, false);
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    2 /* RESULT */,
    4 /* ResponseBatch */,
    response
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  return builder.asUint8Array();
}
function encodeHostResponse(session, sequence, results) {
  const builder = new Builder(512);
  const offsets = results.map((result) => encodeHostResult(builder, result));
  const resultVector = ResponseBatch.createResultsVector(builder, offsets);
  const response = ResponseBatch.createResponseBatch(
    builder,
    session,
    sequence,
    0 /* OK */,
    0,
    resultVector,
    true
  );
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    2 /* RESULT */,
    4 /* ResponseBatch */,
    response
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  return exactArrayBuffer(builder.asUint8Array());
}
function encodeHostEvent(session, listener, callback, eventData) {
  const builder = new Builder(256);
  const contentType = builder.createString("application/json");
  const payload = EventMessage.createPayloadVector(
    builder,
    new TextEncoder().encode(JSON.stringify(eventData != null ? eventData : null))
  );
  const event = EventMessage.createEventMessage(
    builder,
    session,
    listener,
    callback,
    contentType,
    payload
  );
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    3 /* EVENT */,
    5 /* EventMessage */,
    event
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  return exactArrayBuffer(builder.asUint8Array());
}
function exactArrayBuffer(bytes) {
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
}
function encodeHostResult(builder, result) {
  const message = result.message === void 0 ? 0 : builder.createString(result.message);
  let valueKind = 0 /* NONE */;
  let valueType = 0 /* NONE */;
  let value = 0;
  if (result.status === 0 /* OK */) {
    switch (result.resultKind) {
      case "void":
        break;
      case "element_id":
        valueKind = 1 /* ELEMENT_ID */;
        valueType = 1 /* ElementIdResult */;
        value = ElementIdResult.createElementIdResult(builder, result.value);
        break;
      case "element_ids": {
        valueKind = 2 /* ELEMENT_IDS */;
        valueType = 2 /* ElementIdsResult */;
        const values = ElementIdsResult.createValuesVector(builder, result.value);
        value = ElementIdsResult.createElementIdsResult(builder, values);
        break;
      }
      case "string": {
        valueKind = 3 /* STRING */;
        valueType = 3 /* StringResult */;
        const string = builder.createString(result.value);
        value = StringResult.createStringResult(builder, string);
        break;
      }
      case "strings": {
        valueKind = 4 /* STRINGS */;
        valueType = 4 /* StringsResult */;
        const strings = result.value.map((item) => builder.createString(item));
        const values = StringsResult.createValuesVector(builder, strings);
        value = StringsResult.createStringsResult(builder, values);
        break;
      }
      case "boolean":
        valueKind = 5 /* BOOLEAN */;
        valueType = 5 /* BooleanResult */;
        value = BooleanResult.createBooleanResult(builder, result.value);
        break;
      case "number":
        valueKind = 6 /* NUMBER */;
        valueType = 6 /* NumberResult */;
        value = NumberResult.createNumberResult(builder, result.value);
        break;
      case "payload": {
        valueKind = 7 /* PAYLOAD */;
        valueType = 7 /* Payload */;
        const contentType = builder.createString("application/json");
        const bytes = Payload.createBytesVector(
          builder,
          new TextEncoder().encode(JSON.stringify(result.value))
        );
        value = Payload.createPayload(builder, contentType, bytes);
        break;
      }
      default:
        throw new TypeError(`unsupported result kind ${result.resultKind}`);
    }
  }
  return ResultItem.createResultItem(
    builder,
    result.slot,
    result.status,
    message,
    valueKind,
    valueType,
    value
  );
}
function encodeTestCommand(builder, operation) {
  let operationType;
  let operationOffset;
  let resultNodeId = 0;
  let listenerId = 0;
  switch (operation.op) {
    case "create_element": {
      const tag = builder.createString(operation.tag);
      CreateElementCommand.startCreateElementCommand(builder);
      CreateElementCommand.addTag(builder, tag);
      operationOffset = CreateElementCommand.endCreateElementCommand(builder);
      operationType = 15 /* CreateElementCommand */;
      resultNodeId = operation.node;
      break;
    }
    case "create_text": {
      const text = builder.createString(operation.text);
      CreateRawTextCommand.startCreateRawTextCommand(builder);
      CreateRawTextCommand.addText(builder, text);
      operationOffset = CreateRawTextCommand.endCreateRawTextCommand(builder);
      operationType = 6 /* CreateRawTextCommand */;
      resultNodeId = operation.node;
      break;
    }
    case "insert_before": {
      if (operation.reference === null) {
        AppendElementCommand.startAppendElementCommand(builder);
        AppendElementCommand.addParent(builder, operation.parent);
        AppendElementCommand.addCurrent(builder, operation.child);
        operationOffset = AppendElementCommand.endAppendElementCommand(builder);
        operationType = 16 /* AppendElementCommand */;
      } else {
        InsertElementBeforeCommand.startInsertElementBeforeCommand(builder);
        InsertElementBeforeCommand.addParent(builder, operation.parent);
        InsertElementBeforeCommand.addCurrent(builder, operation.child);
        InsertElementBeforeCommand.addMarker(builder, operation.reference);
        operationOffset = InsertElementBeforeCommand.endInsertElementBeforeCommand(builder);
        operationType = 18 /* InsertElementBeforeCommand */;
      }
      break;
    }
    case "remove": {
      RemoveElementCommand.startRemoveElementCommand(builder);
      RemoveElementCommand.addParent(builder, operation.parent);
      RemoveElementCommand.addCurrent(builder, operation.child);
      operationOffset = RemoveElementCommand.endRemoveElementCommand(builder);
      operationType = 17 /* RemoveElementCommand */;
      break;
    }
    case "destroy_node": {
      ReleaseElementCommand.startReleaseElementCommand(builder);
      ReleaseElementCommand.addNode(builder, operation.node);
      operationOffset = ReleaseElementCommand.endReleaseElementCommand(builder);
      operationType = 108 /* ReleaseElementCommand */;
      break;
    }
    case "set_attribute": {
      const name = builder.createString(operation.name);
      const value = operation.value;
      const contentType = builder.createString(value === null ? NULL_CONTENT_TYPE : TEXT_CONTENT_TYPE);
      const bytes = value === null ? 0 : Payload.createBytesVector(builder, new TextEncoder().encode(value));
      const payload = Payload.createPayload(builder, contentType, bytes);
      SetAttributeCommand.startSetAttributeCommand(builder);
      SetAttributeCommand.addCurrent(builder, operation.node);
      SetAttributeCommand.addAttrName(builder, name);
      SetAttributeCommand.addValue(builder, payload);
      operationOffset = SetAttributeCommand.endSetAttributeCommand(builder);
      operationType = 28 /* SetAttributeCommand */;
      break;
    }
    case "add_event_listener": {
      const name = builder.createString(operation.name);
      AddEventListenerCommand.startAddEventListenerCommand(builder);
      AddEventListenerCommand.addNode(builder, operation.node);
      AddEventListenerCommand.addName(builder, name);
      AddEventListenerCommand.addCallback(builder, operation.callback || operation.listener);
      operationOffset = AddEventListenerCommand.endAddEventListenerCommand(builder);
      operationType = 43 /* AddEventListenerCommand */;
      listenerId = operation.listener;
      break;
    }
    case "remove_event_listener": {
      const name = builder.createString(operation.name || "tap");
      RemoveEventListenerCommand.startRemoveEventListenerCommand(builder);
      RemoveEventListenerCommand.addNode(builder, operation.node);
      RemoveEventListenerCommand.addName(builder, name);
      RemoveEventListenerCommand.addCallback(builder, operation.callback || operation.listener);
      operationOffset = RemoveEventListenerCommand.endRemoveEventListenerCommand(builder);
      operationType = 44 /* RemoveEventListenerCommand */;
      listenerId = operation.listener;
      break;
    }
    case "get_tag": {
      GetTagCommand.startGetTagCommand(builder);
      GetTagCommand.addNode(builder, operation.node);
      operationOffset = GetTagCommand.endGetTagCommand(builder);
      operationType = 27 /* GetTagCommand */;
      break;
    }
    case "get_classes": {
      operationOffset = GetClassesCommand.createGetClassesCommand(
        builder,
        operation.node
      );
      operationType = 31 /* GetClassesCommand */;
      break;
    }
    case "set_static_style": {
      SetStaticStyleCommand.startSetStaticStyleCommand(builder);
      SetStaticStyleCommand.addNode(builder, operation.node);
      SetStaticStyleCommand.addKey(builder, operation.key);
      operationOffset = SetStaticStyleCommand.endSetStaticStyleCommand(builder);
      operationType = 32 /* SetStaticStyleCommand */;
      break;
    }
    default:
      throw new TypeError(`cannot encode test operation ${operation.op}`);
  }
  Command.startCommand(builder);
  if (operation.result_slot !== void 0) {
    Command.addResultSlot(builder, operation.result_slot);
  }
  if (resultNodeId !== 0) {
    Command.addResultNodeId(builder, resultNodeId);
  }
  if (listenerId !== 0) {
    Command.addListenerId(builder, listenerId);
  }
  Command.addOperationType(builder, operationType);
  Command.addOperation(builder, operationOffset);
  return Command.endCommand(builder);
}
export {
  PROTOCOL_VERSION,
  decodeBridgeEnvelope,
  encodeHostEvent,
  encodeHostResponse,
  encodeTestBatch,
  encodeTestFailure,
  normalizeByteArray
};
