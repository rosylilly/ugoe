// 例のファイルに入れる小さな PNG を作る (PNG の仕様 https://www.w3.org/TR/png-3/ から)。
// RGBA 8 ビットの画像 (フィルタは None、IDAT は 1 つ) を書き、同じ種類の画像を読む。

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
 * RGBA 8 ビット・インターレースなしの PNG を読む (例のファイルの画像を確かめるため)。
 * ほかの色の種類とビット深度は扱わない。
 */
export function decodeRgbaPng(bytes: Uint8Array): {
  width: number;
  height: number;
  pixels: Uint8Array;
} {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let at = 8;
  let width = 0;
  let height = 0;
  const idat: Uint8Array[] = [];
  while (at + 8 <= bytes.length) {
    const length = view.getUint32(at);
    const type = String.fromCharCode(...bytes.subarray(at + 4, at + 8));
    const data = bytes.subarray(at + 8, at + 8 + length);
    if (type === "IHDR") {
      width = view.getUint32(at + 8);
      height = view.getUint32(at + 12);
      const [depth, color, , , interlace] = data.subarray(8, 13);
      if (depth !== 8 || color !== 6 || interlace !== 0) {
        throw new Error("only 8-bit RGBA non-interlaced PNG is supported");
      }
    } else if (type === "IDAT") idat.push(data);
    else if (type === "IEND") break;
    at += 12 + length;
  }
  const raw = new Uint8Array(inflateSync(Buffer.concat(idat)));
  const stride = 4 * width;
  const pixels = new Uint8Array(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let x = 0; x < stride; x++) {
      const a = x >= 4 ? pixels[y * stride + x - 4] : 0;
      const b = y > 0 ? pixels[(y - 1) * stride + x] : 0;
      const c = x >= 4 && y > 0 ? pixels[(y - 1) * stride + x - 4] : 0;
      let pred = 0;
      if (filter === 1) pred = a;
      else if (filter === 2) pred = b;
      else if (filter === 3) pred = (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        pred = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else if (filter !== 0) throw new Error(`unknown PNG filter ${filter}`);
      pixels[y * stride + x] = (line[x] + pred) & 0xff;
    }
  }
  return { width, height, pixels };
}
