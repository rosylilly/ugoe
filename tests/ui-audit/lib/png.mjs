// 撮った PNG を Node の側で読む小さな読み手 (Tab のたどりの前と後の画像。lib/walk.mjs。T0054)。
// ページの中で読む (createImageBitmap と canvas) と、止まりごとに base64 で送り返す分も含めて遅いため。
// Chrome の撮る PNG (8 ビット・インターレースなし・グレー・RGB・それぞれのアルファ付き) だけを読み、ほかは例外にする。
import { inflateSync } from "node:zlib";

const SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** 色の型ごとの 1 画素のバイト数 (0: グレー、2: RGB、4: グレーとアルファ、6: RGBA)。 */
const CHANNELS = { 0: 1, 2: 3, 4: 2, 6: 4 };

/** PNG のバイト列を {width, height, data} (data は RGBA の Uint8Array。ImageData と同じ並び) に。 */
export function decodePng(png) {
  const buf = Buffer.isBuffer(png) ? png : Buffer.from(png);
  if (buf.length < 8 || !buf.subarray(0, 8).equals(SIGNATURE)) throw new Error("not a PNG");
  let width = 0;
  let height = 0;
  let colorType = -1;
  const idat = [];
  for (let at = 8; at + 8 <= buf.length; ) {
    const length = buf.readUInt32BE(at);
    const type = buf.toString("latin1", at + 4, at + 8);
    const body = buf.subarray(at + 8, at + 8 + length);
    if (type === "IHDR") {
      width = body.readUInt32BE(0);
      height = body.readUInt32BE(4);
      const depth = body[8];
      colorType = body[9];
      const interlace = body[12];
      if (depth !== 8 || !(colorType in CHANNELS) || interlace !== 0) {
        throw new Error(
          `unsupported PNG (bit depth ${depth}, color type ${colorType}, interlace ${interlace})`,
        );
      }
    } else if (type === "IDAT") {
      idat.push(body);
    } else if (type === "IEND") {
      break;
    }
    at += 12 + length;
  }
  if (colorType < 0) throw new Error("PNG has no IHDR");
  const channels = CHANNELS[colorType];
  const stride = width * channels;
  const raw = inflateSync(Buffer.concat(idat));
  if (raw.length < (stride + 1) * height) throw new Error("PNG data is too short");
  // 行ごとのフィルタを戻す (PNG の仕様 9 章。左 a・上 b・左上 c)
  const pixels = new Uint8Array(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const src = y * (stride + 1) + 1;
    const row = y * stride;
    const prev = row - stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? pixels[row + x - channels] : 0;
      const b = y > 0 ? pixels[prev + x] : 0;
      const c = x >= channels && y > 0 ? pixels[prev + x - channels] : 0;
      let v = raw[src + x];
      if (filter === 1) v += a;
      else if (filter === 2) v += b;
      else if (filter === 3) v += (a + b) >> 1;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      } else if (filter !== 0) throw new Error(`unknown PNG filter ${filter}`);
      pixels[row + x] = v;
    }
  }
  if (colorType === 6) return { width, height, data: pixels };
  const data = new Uint8Array(width * height * 4);
  const gray = channels <= 2;
  for (let i = 0, j = 0; i < pixels.length; i += channels, j += 4) {
    data[j] = pixels[i];
    data[j + 1] = gray ? pixels[i] : pixels[i + 1];
    data[j + 2] = gray ? pixels[i] : pixels[i + 2];
    data[j + 3] = channels === 2 ? pixels[i + 1] : 255;
  }
  return { width, height, data };
}
