// スクリーンショット (表示中のモデルを PNG で保存する、T0420)。
//
// 画像は canvas の読み戻し (`toBlob`・`drawImage`) ではなく `WebPuppet.read_pixels()` で取る。canvas と同じ大きさ
// (描画バッファの画素 = CSS ピクセル × devicePixelRatio)・ビュー・背景・光源で別のテクスチャに描いた、ストレートアルファの
// RGBA (行は上から下)。WebGPU の canvas は読み戻すと空になることがあるが、こちらは WebGL2・WebGPU のどちらでも同じに取れる。
// ビューポートに重ねる HTML (光源の印・透明のときの市松模様) は写らない。透明の背景は透明のまま (アルファ 0) 書く。
//
// PNG への変換はブラウザの 2D canvas の `toBlob` で行う (新しい依存を入れない)。ここの関数は DOM に直接触れず、
// canvas を作る関数を受け取る (screenshot.test.ts で偽物を渡す)。

/** 読み戻した画像。`pixels` はストレートアルファの RGBA (各 8 ビット)、行は上から下。 */
export interface Screenshot {
  width: number;
  height: number;
  pixels: Uint8Array;
}

/** `encodePng` が使う canvas の一部 (`HTMLCanvasElement` を満たす)。 */
export interface PngCanvas {
  width: number;
  height: number;
  getContext(kind: "2d"): PngContext | null;
  toBlob(callback: (blob: Blob | null) => void, type?: string): void;
}

/** `encodePng` が使う 2D の描画の文脈の一部。 */
export interface PngContext {
  createImageData(width: number, height: number): PngImage;
  putImageData(image: PngImage, x: number, y: number): void;
}

/** `createImageData` が返す画像 (`ImageData`) の一部。 */
export interface PngImage {
  readonly data: Uint8ClampedArray;
}

/** 画素の数と大きさが合っているかを調べる。合わなければ例外。 */
export function checkScreenshot(shot: Screenshot): void {
  const { width, height, pixels } = shot;
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1)
    throw new Error(`画像の大きさが正しくありません (${width}×${height})`);
  if (pixels.length !== width * height * 4)
    throw new Error(
      `画素の数が大きさと合いません (${pixels.length} バイト、${width}×${height} の RGBA は ${width * height * 4} バイト)`,
    );
}

/** RGBA の画像を PNG のバイト列にする。`createCanvas` は既定で `<canvas>` を作る。 */
export async function encodePng(
  shot: Screenshot,
  createCanvas: () => PngCanvas = () => document.createElement("canvas"),
): Promise<Uint8Array> {
  checkScreenshot(shot);
  const canvas = createCanvas();
  canvas.width = shot.width;
  canvas.height = shot.height;
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("PNG に変換できません (2D の canvas を使えません)");
  // putImageData はストレートアルファの値を受け取る (read_pixels と同じ)。
  const image = context.createImageData(shot.width, shot.height);
  image.data.set(shot.pixels);
  context.putImageData(image, 0, 0);
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (blob === null) throw new Error("PNG に変換できません");
  return new Uint8Array(await blob.arrayBuffer());
}

/** 既定のファイル名に使えない文字 (Windows の禁止文字と制御文字)。 */
// biome-ignore lint/suspicious/noControlCharactersInRegex: 制御文字を取り除くため
const UNSAFE = /[\\/:*?"<>|\u0000-\u001f]/g;

function pad(value: number, width = 2): string {
  return String(value).padStart(width, "0");
}

/**
 * 保存のダイアログの既定のファイル名: `<モデルの名前>-<年月日>-<時分秒>.png` (時刻は地方時)。
 * モデルの名前はファイル名から拡張子と「 (サンプル)」のような後ろの括弧書きを除き、ファイル名に使えない文字を `_` にする。
 * モデルが無い・名前が空なら `screenshot`。
 */
export function screenshotFileName(modelName: string | null, now: Date): string {
  let base = (modelName ?? "").replace(/\s*\([^)]*\)\s*$/, "").trim();
  base = base.replace(/\.(ugoe|ugoemod|json)$/i, "");
  base = base.replace(UNSAFE, "_").trim();
  if (base === "" || base === "." || base === "..") base = "screenshot";
  const date = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}`;
  const time = `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `${base}-${date}-${time}.png`;
}

/** 拡張子 `.png` (大文字小文字は区別しない) がなければ付け足す (保存のダイアログで拡張子を消されたとき)。 */
export function withPngExtension(path: string): string {
  return /\.png$/i.test(path) ? path : `${path}.png`;
}

/** バックエンドの `write_png_file` に保存先のパスを渡すヘッダー (src-tauri/src/lib.rs の `PNG_PATH_HEADER` と同じ)。 */
export const PNG_PATH_HEADER = "x-ugoe-path";
