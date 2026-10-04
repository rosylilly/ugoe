// スクリーンショット (表示中のモデルを PNG で保存する、T0420) の画面の側で、エディタとビューアが共有するもの (T0730)。
//
// 画像は canvas の読み戻し (`toBlob`・`drawImage`) ではなく `WebPuppet.read_pixels()` で取る (ビューアは `Player.capture`、
// エディタは `ViewportController.capture`)。canvas と同じ大きさ (描画バッファの画素 = CSS ピクセル × devicePixelRatio)・
// ビュー・背景・光源で別のテクスチャに描いた、ストレートアルファの RGBA (行は上から下)。WebGPU の canvas は読み戻すと
// 空になることがあるが、こちらは WebGL2・WebGPU のどちらでも同じに取れる。
//
// PNG への変換はブラウザの 2D canvas の `toBlob` で行う (新しい依存を入れない)。ここの関数は DOM に直接触れず、
// canvas を作る関数を受け取る (screenshot.test.ts で偽物を渡す)。既定のファイル名の元の名前・保存のダイアログ・
// バックエンドへの書き込みはアプリが持つ。
import { t } from "./i18n";

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

/** 画素の数と大きさが合っているかを調べる。合わなければ例外 (文は今の言語。画面にそのまま出す)。 */
export function checkScreenshot(shot: Screenshot): void {
  const { width, height, pixels } = shot;
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1)
    throw new Error(t("screenshot.invalidSize", { width, height }));
  if (pixels.length !== width * height * 4)
    throw new Error(
      t("screenshot.pixelCount", {
        bytes: pixels.length,
        width,
        height,
        expected: width * height * 4,
      }),
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
  if (context === null) throw new Error(t("screenshot.noCanvas"));
  // putImageData はストレートアルファの値を受け取る (read_pixels と同じ)。
  const image = context.createImageData(shot.width, shot.height);
  image.data.set(shot.pixels);
  context.putImageData(image, 0, 0);
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (blob === null) throw new Error(t("screenshot.encodeFailed"));
  return new Uint8Array(await blob.arrayBuffer());
}

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

/**
 * 保存のダイアログの既定のファイル名: `<base>-<年月日>-<時分秒>.png` (時刻は地方時)。
 * `base` (モデルやプロジェクトの名前から、ファイル名に使えない文字を除いたもの) はアプリが決める。
 */
export function timestampedPngName(base: string, now: Date): string {
  const date = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}`;
  const time = `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `${base}-${date}-${time}.png`;
}

/**
 * バックエンドの `write_png_file` に保存先のパスを渡すヘッダー (値は `encodeURIComponent` で符号化したパス)。
 * エディタの `src-tauri/src/screenshot.rs` の `PATH_HEADER`・ビューアの `src-tauri/src/lib.rs` の `PNG_PATH_HEADER` と同じ。
 */
export const PNG_PATH_HEADER = "x-ugoe-path";
