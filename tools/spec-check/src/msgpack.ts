// MessagePack の読み込み (spec/bundle.md#payload-encoding)。
//
// 仕様 (https://github.com/msgpack/msgpack/blob/master/spec.md) だけから書いた最小の復号器。
// - 整数は `bigint`、浮動小数点数は `number` で返す。整数を要するフィールドで浮動小数点数を
//   拒否するため (spec/model.md#types)、両者を区別して残す。
// - map のキーは文字列だけを受け付け、プレーンなオブジェクトにする (オブジェクトの型はすべて
//   文字列のキーの map)。
// - 入れ子の深さを制限する (spec/bundle.md#messagepack-limits は 128 以上を求める)。

export type MsgValue =
  | null
  | boolean
  | bigint
  | number
  | string
  | Uint8Array
  | MsgValue[]
  | { [key: string]: MsgValue };

export class MsgpackError extends Error {}

/** 入れ子の深さの上限。仕様の下限 128 よりゆとりを持たせる。 */
export const MAX_DEPTH = 256;

const utf8 = new TextDecoder("utf-8", { fatal: true });

class Reader {
  pos = 0;
  readonly view: DataView;
  readonly bytes: Uint8Array;

  constructor(bytes: Uint8Array) {
    this.bytes = bytes;
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }

  need(n: number): void {
    if (this.pos + n > this.bytes.length) {
      throw new MsgpackError(`unexpected end of data at byte ${this.pos}`);
    }
  }

  u8(): number {
    this.need(1);
    return this.view.getUint8(this.pos++);
  }

  u16(): number {
    this.need(2);
    const v = this.view.getUint16(this.pos);
    this.pos += 2;
    return v;
  }

  u32(): number {
    this.need(4);
    const v = this.view.getUint32(this.pos);
    this.pos += 4;
    return v;
  }

  take(n: number): Uint8Array {
    this.need(n);
    const out = this.bytes.subarray(this.pos, this.pos + n);
    this.pos += n;
    return out;
  }
}

/** 先頭のバイトから、値の種類と長さを読む (値の本体は読まない)。 */
type Head =
  | { kind: "nil" }
  | { kind: "bool"; value: boolean }
  | { kind: "int"; value: bigint }
  | { kind: "float"; value: number }
  | { kind: "str"; length: number }
  | { kind: "bin"; length: number }
  | { kind: "array"; length: number }
  | { kind: "map"; length: number }
  | { kind: "ext"; length: number };

function head(r: Reader): Head {
  const b = r.u8();
  if (b <= 0x7f) return { kind: "int", value: BigInt(b) };
  if (b >= 0xe0) return { kind: "int", value: BigInt(b - 0x100) };
  if (b >= 0x80 && b <= 0x8f) return { kind: "map", length: b & 0x0f };
  if (b >= 0x90 && b <= 0x9f) return { kind: "array", length: b & 0x0f };
  if (b >= 0xa0 && b <= 0xbf) return { kind: "str", length: b & 0x1f };
  const v = r.view;
  switch (b) {
    case 0xc0:
      return { kind: "nil" };
    case 0xc2:
      return { kind: "bool", value: false };
    case 0xc3:
      return { kind: "bool", value: true };
    case 0xc4:
      return { kind: "bin", length: r.u8() };
    case 0xc5:
      return { kind: "bin", length: r.u16() };
    case 0xc6:
      return { kind: "bin", length: r.u32() };
    case 0xc7:
      return { kind: "ext", length: r.u8() + 1 };
    case 0xc8:
      return { kind: "ext", length: r.u16() + 1 };
    case 0xc9:
      return { kind: "ext", length: r.u32() + 1 };
    case 0xca: {
      r.need(4);
      const f = v.getFloat32(r.pos);
      r.pos += 4;
      return { kind: "float", value: f };
    }
    case 0xcb: {
      r.need(8);
      const f = v.getFloat64(r.pos);
      r.pos += 8;
      return { kind: "float", value: f };
    }
    case 0xcc:
      return { kind: "int", value: BigInt(r.u8()) };
    case 0xcd:
      return { kind: "int", value: BigInt(r.u16()) };
    case 0xce:
      return { kind: "int", value: BigInt(r.u32()) };
    case 0xcf: {
      r.need(8);
      const i = v.getBigUint64(r.pos);
      r.pos += 8;
      return { kind: "int", value: i };
    }
    case 0xd0: {
      r.need(1);
      return { kind: "int", value: BigInt(v.getInt8(r.pos++)) };
    }
    case 0xd1: {
      r.need(2);
      const i = v.getInt16(r.pos);
      r.pos += 2;
      return { kind: "int", value: BigInt(i) };
    }
    case 0xd2: {
      r.need(4);
      const i = v.getInt32(r.pos);
      r.pos += 4;
      return { kind: "int", value: BigInt(i) };
    }
    case 0xd3: {
      r.need(8);
      const i = v.getBigInt64(r.pos);
      r.pos += 8;
      return { kind: "int", value: i };
    }
    case 0xd4:
      return { kind: "ext", length: 2 };
    case 0xd5:
      return { kind: "ext", length: 3 };
    case 0xd6:
      return { kind: "ext", length: 5 };
    case 0xd7:
      return { kind: "ext", length: 9 };
    case 0xd8:
      return { kind: "ext", length: 17 };
    case 0xd9:
      return { kind: "str", length: r.u8() };
    case 0xda:
      return { kind: "str", length: r.u16() };
    case 0xdb:
      return { kind: "str", length: r.u32() };
    case 0xdc:
      return { kind: "array", length: r.u16() };
    case 0xdd:
      return { kind: "array", length: r.u32() };
    case 0xde:
      return { kind: "map", length: r.u16() };
    case 0xdf:
      return { kind: "map", length: r.u32() };
    default:
      throw new MsgpackError(`invalid type byte 0x${b.toString(16)} at byte ${r.pos - 1}`);
  }
}

function checkDepth(depth: number): void {
  if (depth > MAX_DEPTH) throw new MsgpackError(`nesting deeper than ${MAX_DEPTH} levels`);
}

function readValue(r: Reader, depth: number): MsgValue {
  const h = head(r);
  switch (h.kind) {
    case "nil":
      return null;
    case "bool":
    case "int":
    case "float":
      return h.value;
    case "str":
      return utf8.decode(r.take(h.length));
    case "bin":
      return r.take(h.length).slice();
    case "ext":
      throw new MsgpackError("extension types are not used by the ugoe format");
    case "array": {
      checkDepth(depth + 1);
      // 長さは信用しない (巨大な長さで先に配列を確保しない)
      const out: MsgValue[] = [];
      for (let i = 0; i < h.length; i++) out.push(readValue(r, depth + 1));
      return out;
    }
    case "map": {
      checkDepth(depth + 1);
      const out: { [key: string]: MsgValue } = Object.create(null);
      for (let i = 0; i < h.length; i++) {
        const key = readValue(r, depth + 1);
        if (typeof key !== "string") throw new MsgpackError("map key is not a string");
        if (Object.hasOwn(out, key)) throw new MsgpackError(`duplicate map key "${key}"`);
        out[key] = readValue(r, depth + 1);
      }
      return out;
    }
  }
}

/** 値を解釈せずに読み飛ばす。 */
function skipValue(r: Reader, depth: number): void {
  const h = head(r);
  switch (h.kind) {
    case "str":
    case "bin":
    case "ext":
      r.take(h.length);
      return;
    case "array":
      checkDepth(depth + 1);
      for (let i = 0; i < h.length; i++) skipValue(r, depth + 1);
      return;
    case "map":
      checkDepth(depth + 1);
      for (let i = 0; i < 2 * h.length; i++) skipValue(r, depth + 1);
      return;
    default:
      return;
  }
}

/** ペイロード全体を 1 つの値として復号する。後ろに余りのバイトがあれば誤り。 */
export function decode(bytes: Uint8Array): MsgValue {
  const r = new Reader(bytes);
  const value = readValue(r, 0);
  if (r.pos !== bytes.length) {
    throw new MsgpackError(`${bytes.length - r.pos} trailing bytes after the value`);
  }
  return value;
}

/**
 * ルートの map から 1 つのキーの値だけを、ほかのキーの値を読み飛ばして取り出す
 * (spec/bundle.md#reading の手順 4: `schema_version` を先に読む)。
 * ルートが map でない、キーが文字列でないなど、この方法で読めなければ `undefined`。
 */
export function peekKey(bytes: Uint8Array, key: string): MsgValue | undefined {
  try {
    const r = new Reader(bytes);
    const h = head(r);
    if (h.kind !== "map") return undefined;
    for (let i = 0; i < h.length; i++) {
      const k = readValue(r, 1);
      if (typeof k !== "string") return undefined;
      if (k === key) return readValue(r, 1);
      skipValue(r, 1);
    }
    return undefined;
  } catch {
    return undefined;
  }
}
