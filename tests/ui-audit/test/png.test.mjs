// Node の側の PNG の読み手 (lib/png.mjs) と、それで読んだフォーカスの前と後の差のテスト (`just ui-audit-check`。T0054)
import assert from "node:assert/strict";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";
import { decodePng } from "../lib/png.mjs";
import { focusDiff } from "../lib/walk.mjs";

const { focusChange } = globalThis.__ugoeAudit;

function chunk(type, body) {
  const head = Buffer.alloc(8);
  head.writeUInt32BE(body.length, 0);
  head.write(type, 4, "latin1");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), body])), 0);
  return Buffer.concat([head, body, crc]);
}

/**
 * 画素 (1 画素 `channels` バイトの行の列) を PNG に。行 `y` のフィルタは `filters[y % filters.length]`
 * (読み手がすべてのフィルタを戻せるかを見るため。符号化の向きのフィルタをかけて書く)。IDAT は 2 つに分ける。
 */
function encodePng(width, height, colorType, channels, pixels, filters = [0]) {
  const stride = width * channels;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    const filter = filters[y % filters.length];
    raw[y * (stride + 1)] = filter;
    for (let x = 0; x < stride; x++) {
      const at = y * stride + x;
      const a = x >= channels ? pixels[at - channels] : 0;
      const b = y > 0 ? pixels[at - stride] : 0;
      const c = x >= channels && y > 0 ? pixels[at - stride - channels] : 0;
      let predicted = 0;
      if (filter === 1) predicted = a;
      else if (filter === 2) predicted = b;
      else if (filter === 3) predicted = (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        predicted = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      raw[y * (stride + 1) + 1 + x] = (pixels[at] - predicted) & 0xff;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = colorType;
  const data = deflateSync(raw);
  const half = data.length >> 1;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", data.subarray(0, half)),
    chunk("IDAT", data.subarray(half)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** 決まった並びの、隣どうしで値の跳ぶ画素 (フィルタの予測が外れる所を作る)。 */
function noise(length, seed = 7) {
  const out = new Uint8Array(length);
  let s = seed;
  for (let i = 0; i < length; i++) {
    s = (s * 1103515245 + 12345) >>> 0;
    out[i] = s >>> 24;
  }
  return out;
}

test("PNG を読む: 5 つのフィルタ (なし・左・上・平均・Paeth) を戻し、IDAT をつなぐ", () => {
  const width = 7;
  const height = 10;
  const rgba = noise(width * height * 4);
  const decoded = decodePng(encodePng(width, height, 6, 4, rgba, [0, 1, 2, 3, 4]));
  assert.equal(decoded.width, width);
  assert.equal(decoded.height, height);
  assert.deepEqual(decoded.data, rgba);
});

test("PNG を読む: RGB・グレー・グレーとアルファは RGBA に広げる (アルファの無いものは 255)", () => {
  const rgb = noise(3 * 2 * 3, 11);
  const fromRgb = decodePng(encodePng(3, 2, 2, 3, rgb, [4, 3]));
  assert.deepEqual(
    [...fromRgb.data.subarray(0, 8)],
    [rgb[0], rgb[1], rgb[2], 255, rgb[3], rgb[4], rgb[5], 255],
  );
  const gray = decodePng(encodePng(2, 1, 0, 1, Uint8Array.from([10, 200])));
  assert.deepEqual([...gray.data], [10, 10, 10, 255, 200, 200, 200, 255]);
  const grayAlpha = decodePng(encodePng(1, 1, 4, 2, Uint8Array.from([30, 128])));
  assert.deepEqual([...grayAlpha.data], [30, 30, 30, 128]);
});

test("PNG を読む: 読めないもの (PNG でない・16 ビット・パレット) は例外", () => {
  assert.throws(() => decodePng(Buffer.from("not a png")), /not a PNG/);
  const png = encodePng(1, 1, 6, 4, Uint8Array.from([1, 2, 3, 4]));
  const deep = Buffer.from(png);
  deep[8 + 8 + 8] = 16;
  assert.throws(() => decodePng(deep), /bit depth 16/);
  const palette = Buffer.from(png);
  palette[8 + 8 + 9] = 3;
  assert.throws(() => decodePng(palette), /color type 3/);
});

test("Node の側で読んだ前と後の差は、画素を直に比べた focusChange と同じ (数は CSS px² に直す)", () => {
  const width = 12;
  const height = 8;
  const before = new Uint8Array(width * height * 4).fill(255);
  const after = Uint8Array.from(before);
  // 外周の 1 画素を濃い青に (はっきり変わった画素)、中の 2 列を薄い灰に (変わったが 3:1 に届かない)
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      if (x === 0 || y === 0 || x === width - 1 || y === height - 1) after.set([37, 99, 235], i);
      else if (x < 3) after.set([200, 200, 200], i);
    }
  }
  const raw = focusChange({ width, height, data: before }, { width, height, data: after });
  const png = (data) => encodePng(width, height, 6, 4, data, [1, 4, 2]);
  assert.deepEqual(focusDiff(png(before), png(after), 1), raw);
  // devicePixelRatio 2 では 1 CSS px が 4 画素
  assert.deepEqual(focusDiff(png(before), png(after), 2), {
    changed: raw.changed / 4,
    strong: raw.strong / 4,
    maxRatio: raw.maxRatio,
  });
  assert.equal(raw.strong, 2 * (width + height) - 4);
  assert.equal(raw.changed, raw.strong + 2 * (height - 2));
});
