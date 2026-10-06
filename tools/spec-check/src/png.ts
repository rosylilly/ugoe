// 例のファイルに入れる小さな PNG を作る (PNG の仕様 https://www.w3.org/TR/png-3/ から)。
// RGBA 8 ビットの画像 (フィルタは None、IDAT は 1 つ) と、色の種類・ビット深度・チャンクを指定した画像を書き、
// テクスチャの読み方 (spec/rendering.md#textures) どおりに読む。

import { crc32, deflateSync, inflateSync } from "node:zlib";

function chunk(type: string, data: Uint8Array): Uint8Array {
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  for (let i = 0; i < 4; i++) out[4 + i] = type.charCodeAt(i);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));
  return out;
}

/** 幅 `width`・高さ `height` を 1 色 `rgba` で塗った PNG。 */
export function solidPng(width: number, height: number, rgba: [number, number, number, number]) {
  const pixels = new Uint8Array(4 * width * height);
  for (let i = 0; i < width * height; i++) pixels.set(rgba, 4 * i);
  return rgbaPng(width, height, pixels);
}

/** 幅 `width`・高さ `height` の RGBA 8 ビットの画素 (行ごと、左上から) の PNG。 */
export function rgbaPng(width: number, height: number, pixels: ArrayLike<number>) {
  const ihdr = new Uint8Array(13);
  const view = new DataView(ihdr.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  ihdr.set([8, 6, 0, 0, 0], 8); // 8 ビット・RGBA・deflate・フィルタ 0・インターレースなし
  const raw = new Uint8Array(height * (1 + 4 * width));
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < 4 * width; x++)
      raw[y * (1 + 4 * width) + 1 + x] = pixels[4 * width * y + x];
  }
  const parts = [
    new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", new Uint8Array(deflateSync(raw))),
    chunk("IEND", new Uint8Array(0)),
  ];
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

/**
 * 例のための PNG の中身の記述 (spec/examples/README.md#file-examples の `png`)。
 * `samples` は行ごと・左上から、画素ごとのサンプルの値 (ビット深度のまま。インデックスカラーはパレットの番号)。
 */
export interface PngSpec {
  color_type: 0 | 2 | 3 | 4 | 6;
  bit_depth: 1 | 2 | 4 | 8 | 16;
  /** 1 なら Adam7。 */
  interlace?: 0 | 1;
  /** `PLTE` の各項目の [r, g, b]。 */
  palette?: [number, number, number][];
  /** `tRNS` の値 (インデックスカラーは各項目の alpha、グレースケールは [g]、RGB は [r, g, b])。 */
  transparency?: number[];
  /** `gAMA` の値 (ガンマの 100000 倍)。 */
  gamma?: number;
  /** `cHRM` の 8 つの値 (100000 倍)。 */
  chromaticities?: number[];
  /** `sRGB` のレンダリングインテント。 */
  srgb?: number;
  samples: number[];
}

const CHANNELS = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 } as const;

/** Adam7 の 7 つのパス: [x の始まり, y の始まり, x の間隔, y の間隔]。 */
const ADAM7 = [
  [0, 0, 8, 8],
  [4, 0, 8, 8],
  [0, 4, 4, 8],
  [2, 0, 4, 4],
  [0, 2, 2, 4],
  [1, 0, 2, 2],
  [0, 1, 1, 2],
] as const;

/** インターレースなしなら画像全体、Adam7 なら 7 つのパスの、部分画像の画素の位置 (行ごと)。 */
function subImages(width: number, height: number, interlace: number): [number, number][][][] {
  const passes = interlace === 1 ? ADAM7 : [[0, 0, 1, 1] as const];
  return passes.map(([x0, y0, dx, dy]) => {
    const rows: [number, number][][] = [];
    for (let y = y0; y < height; y += dy) {
      const row: [number, number][] = [];
      for (let x = x0; x < width; x += dx) row.push([x, y]);
      if (row.length > 0) rows.push(row);
    }
    return rows;
  });
}

function paeth(a: number, b: number, c: number): number {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
}

/** フィルタの種類 `filter` の予測 (a = 左、b = 上、c = 左上)。 */
function predict(filter: number, a: number, b: number, c: number): number {
  if (filter === 0) return 0;
  if (filter === 1) return a;
  if (filter === 2) return b;
  if (filter === 3) return (a + b) >> 1;
  if (filter === 4) return paeth(a, b, c);
  throw new Error(`unknown PNG filter ${filter}`);
}

/**
 * 記述 `spec` の PNG を作る。走査線のフィルタは 0〜4 を順に使い (パスごとに前の行を空に戻す)、
 * IDAT は 2 つに分け、色のチャンクは IHDR の直後に置く。
 */
export function encodePng(width: number, height: number, spec: PngSpec): Uint8Array {
  const channels = CHANNELS[spec.color_type];
  const depth = spec.bit_depth;
  if (spec.samples.length !== width * height * channels) {
    throw new Error(`${spec.samples.length} samples for ${width}x${height}x${channels}`);
  }
  const bpp = Math.max(1, (channels * depth) / 8);
  const raw: number[] = [];
  let line = 0;
  for (const rows of subImages(width, height, spec.interlace ?? 0)) {
    let prior: number[] = [];
    for (const row of rows) {
      // サンプルを詰める (8 ビット未満は上位のビットから、16 ビットは上位のバイトから)
      const bytes = new Array<number>(Math.ceil((row.length * channels * depth) / 8)).fill(0);
      let bit = 0;
      for (const [x, y] of row) {
        for (let k = 0; k < channels; k++) {
          const v = spec.samples[(y * width + x) * channels + k];
          if (depth === 16) {
            bytes[bit / 8] = v >> 8;
            bytes[bit / 8 + 1] = v & 0xff;
          } else if (depth === 8) bytes[bit / 8] = v;
          else bytes[bit >> 3] |= v << (8 - depth - (bit & 7));
          bit += depth;
        }
      }
      const filter = line++ % 5;
      raw.push(filter);
      bytes.forEach((v, i) => {
        const a = i >= bpp ? bytes[i - bpp] : 0;
        const b = prior[i] ?? 0;
        const c = i >= bpp ? (prior[i - bpp] ?? 0) : 0;
        raw.push((v - predict(filter, a, b, c)) & 0xff);
      });
      prior = bytes;
    }
  }
  const ihdr = new Uint8Array(13);
  const view = new DataView(ihdr.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  ihdr.set([depth, spec.color_type, 0, 0, spec.interlace ?? 0], 8);
  const u32s = (vs: number[]) => {
    const out = new Uint8Array(4 * vs.length);
    vs.forEach((v, i) => {
      new DataView(out.buffer).setUint32(4 * i, v);
    });
    return out;
  };
  const parts = [
    new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
  ];
  if (spec.gamma !== undefined) parts.push(chunk("gAMA", u32s([spec.gamma])));
  if (spec.chromaticities) parts.push(chunk("cHRM", u32s(spec.chromaticities)));
  if (spec.srgb !== undefined) parts.push(chunk("sRGB", new Uint8Array([spec.srgb])));
  if (spec.palette) parts.push(chunk("PLTE", new Uint8Array(spec.palette.flat())));
  if (spec.transparency) {
    const t = spec.transparency;
    const data =
      spec.color_type === 3
        ? new Uint8Array(t)
        : new Uint8Array(t.flatMap((v) => [v >> 8, v & 0xff]));
    parts.push(chunk("tRNS", data));
  }
  const idat = new Uint8Array(deflateSync(new Uint8Array(raw)));
  const half = idat.length >> 1;
  parts.push(chunk("IDAT", idat.subarray(0, half)), chunk("IDAT", idat.subarray(half)));
  parts.push(chunk("IEND", new Uint8Array(0)));
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

/**
 * PNG を非乗算の RGBA 8 ビットに読む (spec/rendering.md#textures の規則)。
 * サンプルはそのまま使い、ガンマと色のチャンクは見ない。16 ビットは上位のバイトを取り (丸めない)、
 * 8 ビット未満のグレースケールは `v × 255 / (2^d − 1)` に広げる (パレットの番号は広げない)。
 * グレースケールと RGB の `tRNS` は、ビット深度のままの値 (16 ビットは 16 ビットすべて) で比べる。
 * Adam7 は全体の画像に戻す。APNG のフレーム (`fcTL`・`fdAT`) は読まない。
 */
export function decodePng(bytes: Uint8Array): {
  width: number;
  height: number;
  pixels: Uint8Array;
} {
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (bytes.length < 8 || signature.some((b, i) => bytes[i] !== b)) {
    throw new Error("not a PNG");
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let at = 8;
  let ihdr: Uint8Array | null = null;
  let palette: Uint8Array | null = null;
  let trns: Uint8Array | null = null;
  const idat: Uint8Array[] = [];
  while (at + 12 <= bytes.length) {
    const length = view.getUint32(at);
    const type = String.fromCharCode(...bytes.subarray(at + 4, at + 8));
    if (at + 12 + length > bytes.length) throw new Error(`truncated ${type} chunk`);
    const data = bytes.subarray(at + 8, at + 8 + length);
    if (type === "IHDR") ihdr = data;
    else if (type === "PLTE") palette = data;
    else if (type === "tRNS") trns = data;
    else if (type === "IDAT") idat.push(data);
    else if (type === "IEND") break;
    at += 12 + length;
  }
  if (ihdr?.length !== 13) throw new Error("missing IHDR");
  const h = new DataView(ihdr.buffer, ihdr.byteOffset, 13);
  const width = h.getUint32(0);
  const height = h.getUint32(4);
  const [depth, colorType, , , interlace] = ihdr.subarray(8, 13);
  if (!(colorType in CHANNELS)) throw new Error(`unknown PNG color type ${colorType}`);
  if (![1, 2, 4, 8, 16].includes(depth)) throw new Error(`unknown PNG bit depth ${depth}`);
  if (colorType === 3 && !palette) throw new Error("indexed-color PNG without PLTE");
  const channels = CHANNELS[colorType as keyof typeof CHANNELS];
  const bpp = Math.max(1, (channels * depth) / 8);

  // フィルタを戻し、サンプルを全体の画像の位置に置く
  const samples = new Uint32Array(width * height * channels);
  const raw = new Uint8Array(inflateSync(Buffer.concat(idat)));
  let pos = 0;
  for (const rows of subImages(width, height, interlace)) {
    let prior = new Uint8Array(0);
    for (const row of rows) {
      const stride = Math.ceil((row.length * channels * depth) / 8);
      if (pos + 1 + stride > raw.length) throw new Error("PNG image data too short");
      const filter = raw[pos];
      const line = new Uint8Array(stride);
      for (let i = 0; i < stride; i++) {
        const a = i >= bpp ? line[i - bpp] : 0;
        const b = prior[i] ?? 0;
        const c = i >= bpp ? (prior[i - bpp] ?? 0) : 0;
        line[i] = (raw[pos + 1 + i] + predict(filter, a, b, c)) & 0xff;
      }
      pos += 1 + stride;
      prior = line;
      let bit = 0;
      for (const [x, y] of row) {
        for (let k = 0; k < channels; k++) {
          let v: number;
          if (depth === 16) v = (line[bit / 8] << 8) | line[bit / 8 + 1];
          else if (depth === 8) v = line[bit / 8];
          else v = (line[bit >> 3] >> (8 - depth - (bit & 7))) & ((1 << depth) - 1);
          samples[(y * width + x) * channels + k] = v;
          bit += depth;
        }
      }
    }
  }

  // tRNS のグレースケールと RGB の透明な色 (ビット深度のまま)
  const key =
    trns && (colorType === 0 || colorType === 2)
      ? Array.from({ length: trns.length >> 1 }, (_, i) => (trns[2 * i] << 8) | trns[2 * i + 1])
      : null;
  const to8 = (v: number) =>
    depth === 16 ? v >> 8 : depth === 8 ? v : (v * 255) / (2 ** depth - 1);
  const pixels = new Uint8Array(4 * width * height);
  for (let p = 0; p < width * height; p++) {
    const s = samples.subarray(p * channels, (p + 1) * channels);
    let rgba: number[];
    if (colorType === 3) {
      const i = s[0];
      if (!palette || 3 * i + 3 > palette.length)
        throw new Error(`palette index ${i} out of range`);
      rgba = [...palette.subarray(3 * i, 3 * i + 3), trns && i < trns.length ? trns[i] : 255];
    } else {
      const alpha = key && key.length === channels && key.every((k, q) => k === s[q]) ? 0 : 255;
      if (colorType === 0) rgba = [to8(s[0]), to8(s[0]), to8(s[0]), alpha];
      else if (colorType === 2) rgba = [to8(s[0]), to8(s[1]), to8(s[2]), alpha];
      else if (colorType === 4) rgba = [to8(s[0]), to8(s[0]), to8(s[0]), to8(s[1])];
      else rgba = [...s].map(to8);
    }
    pixels.set(rgba, 4 * p);
  }
  return { width, height, pixels };
}
