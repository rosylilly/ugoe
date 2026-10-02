// ビューポートのスクリーンショット (表示中のモデルを PNG で保存する、T0420。docs/editor.md#スクリーンショット)。
//
// 画像は canvas の読み戻し (`toBlob`・`drawImage`) ではなく `WebPuppet.read_pixels()` で取る (ViewportController.capture)。
// canvas と同じ大きさ (描画バッファの画素 = CSS ピクセル × devicePixelRatio)・ビュー・背景・光源で別のテクスチャに描いた、
// ストレートアルファの RGBA (行は上から下)。WebGL2・WebGPU のどちらでも同じに取れる。
//
// PNG への変換はブラウザの 2D canvas の `toBlob` で行う (新しい依存を入れない)。ここの関数は DOM・IPC に触れず、
// canvas を作る関数を受け取る (screenshot.test.ts で偽物を渡す)。保存の流れは saveScreenshot.ts。
import { safeFileName } from "../filePaths";

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
    throw new Error(`invalid image size: ${width}×${height}`);
  if (pixels.length !== width * height * 4)
    throw new Error(
      `the pixels do not match the size: ${pixels.length} bytes for ${width}×${height} RGBA (${width * height * 4} bytes)`,
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
  if (context === null) throw new Error("cannot encode PNG: 2D canvas is not available");
  // putImageData はストレートアルファの値を受け取る (read_pixels と同じ)。
  const image = context.createImageData(shot.width, shot.height);
  image.data.set(shot.pixels);
  context.putImageData(image, 0, 0);
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (blob === null) throw new Error("cannot encode PNG");
  return new Uint8Array(await blob.arrayBuffer());
}

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

/**
 * 保存のダイアログの既定のファイル名: `<プロジェクト名>-<年月日>-<時分秒>.png` (時刻は地方時)。
 * プロジェクト名のうちファイル名に使えない文字は `_` にし、空なら `Untitled` (filePaths.ts の `safeFileName`)。
 */
export function screenshotFileName(projectName: string | null, now: Date): string {
  const base = safeFileName(projectName ?? "");
  const date = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}`;
  const time = `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `${base}-${date}-${time}.png`;
}

/** バックエンドの `write_png_file` に保存先のパスを渡すヘッダー (src-tauri/src/screenshot.rs の `PATH_HEADER` と同じ)。 */
export const PNG_PATH_HEADER = "x-ugoe-path";

/** ダイアログのフィルタ。 */
export const PNG_FILTER = { name: "PNG image (.png)", extensions: ["png"] };
