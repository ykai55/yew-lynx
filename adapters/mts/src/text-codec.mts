function appendCodePoint(output: string[], codePoint: number) {
  if (codePoint <= 0xffff) {
    output.push(String.fromCharCode(codePoint));
    return;
  }
  const value = codePoint - 0x10000;
  output.push(String.fromCharCode(0xd800 + (value >> 10), 0xdc00 + (value & 0x3ff)));
}

export class TextEncoder {
  readonly encoding = "utf-8";

  encode(input = ""): Uint8Array {
    const bytes: number[] = [];
    for (let index = 0; index < input.length; index += 1) {
      let codePoint = input.charCodeAt(index);
      if (codePoint >= 0xd800 && codePoint <= 0xdbff) {
        const trailing = input.charCodeAt(index + 1);
        if (trailing >= 0xdc00 && trailing <= 0xdfff) {
          codePoint = 0x10000 + ((codePoint - 0xd800) << 10) + trailing - 0xdc00;
          index += 1;
        } else {
          codePoint = 0xfffd;
        }
      } else if (codePoint >= 0xdc00 && codePoint <= 0xdfff) {
        codePoint = 0xfffd;
      }

      if (codePoint <= 0x7f) {
        bytes.push(codePoint);
      } else if (codePoint <= 0x7ff) {
        bytes.push(0xc0 | (codePoint >> 6), 0x80 | (codePoint & 0x3f));
      } else if (codePoint <= 0xffff) {
        bytes.push(
          0xe0 | (codePoint >> 12),
          0x80 | ((codePoint >> 6) & 0x3f),
          0x80 | (codePoint & 0x3f),
        );
      } else {
        bytes.push(
          0xf0 | (codePoint >> 18),
          0x80 | ((codePoint >> 12) & 0x3f),
          0x80 | ((codePoint >> 6) & 0x3f),
          0x80 | (codePoint & 0x3f),
        );
      }
    }
    return new Uint8Array(bytes);
  }
}

export class TextDecoder {
  readonly encoding = "utf-8";
  readonly fatal = false;
  readonly ignoreBOM = true;

  decode(input: ArrayBuffer | ArrayBufferView = new Uint8Array()): string {
    const bytes = input instanceof ArrayBuffer
      ? new Uint8Array(input)
      : new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
    const output: string[] = [];
    for (let index = 0; index < bytes.length;) {
      const leading = bytes[index];
      if (leading <= 0x7f) {
        appendCodePoint(output, leading);
        index += 1;
        continue;
      }

      const width = leading >= 0xc2 && leading <= 0xdf
        ? 2
        : leading >= 0xe0 && leading <= 0xef
          ? 3
          : leading >= 0xf0 && leading <= 0xf4
            ? 4
            : 0;
      if (width === 0 || index + width > bytes.length) {
        appendCodePoint(output, 0xfffd);
        index += 1;
        continue;
      }

      let codePoint = leading & (0x7f >> width);
      let valid = true;
      for (let offset = 1; offset < width; offset += 1) {
        const continuation = bytes[index + offset];
        if ((continuation & 0xc0) !== 0x80) {
          valid = false;
          break;
        }
        codePoint = (codePoint << 6) | (continuation & 0x3f);
      }
      const minimum = width === 2 ? 0x80 : width === 3 ? 0x800 : 0x10000;
      if (
        !valid
        || codePoint < minimum
        || codePoint > 0x10ffff
        || (codePoint >= 0xd800 && codePoint <= 0xdfff)
      ) {
        appendCodePoint(output, 0xfffd);
        index += 1;
        continue;
      }
      appendCodePoint(output, codePoint);
      index += width;
    }
    return output.join("");
  }
}
