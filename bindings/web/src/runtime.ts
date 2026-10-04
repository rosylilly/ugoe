// wasm の初期化・WebPuppet の作成・モデルの読み込みの薄い補助。

import { type Backend, isBundle } from "./attributes.js";
import { type LoadLimits, limitsJson } from "./limits.js";
import type { UgoeCurveValue } from "./queries.js";
import init, { type InitInput, sample_motion_json, WebPuppet } from "./wasm/ugoe_wasm.js";

/** `initRuntime` の設定。 */
export interface InitOptions {
  /**
   * wasm の場所や中身。省略すると、このパッケージの `dist/wasm/ugoe_wasm_bg.wasm` を
   * (`import.meta.url` からの相対で) 読む。バンドラが wasm を別の場所に置くときに指定する。
   */
  wasm?: InitInput | Promise<InitInput>;
}

let ready: Promise<void> | undefined;

/**
 * wasm を読み込んで初期化する。2 回目以降は最初の呼び出しと同じ Promise を返す
 * (最初の呼び出しの `options` だけが効く)。`createPuppet` と `<ugoe-avatar>` は自分で呼ぶので、
 * wasm の場所を変えるときだけ先に呼べばよい。
 */
export function initRuntime(options: InitOptions = {}): Promise<void> {
  ready ??= init(options.wasm === undefined ? undefined : { module_or_path: options.wasm }).then(
    () => undefined,
    (e: unknown) => {
      // 失敗したら次の呼び出しでやり直せるようにする (ネットワークの一時的な失敗など)。
      ready = undefined;
      throw e;
    },
  );
  return ready;
}

/** `createPuppet` の設定。 */
export interface CreatePuppetOptions {
  /** 既定は `"auto"` (WebGPU を試し、使えなければ WebGL2)。 */
  backend?: Backend;
}

/**
 * `canvas` に描く `WebPuppet` を作る (wasm の初期化も行う)。
 *
 * 1 つの canvas は WebGPU と WebGL2 のどちらか一方にしか使えない。作成に失敗したら、
 * 別の canvas と `backend: "webgl2"` で作り直すこと (`<ugoe-avatar>` はそうしている)。
 */
export async function createPuppet(
  canvas: HTMLCanvasElement,
  options: CreatePuppetOptions = {},
): Promise<WebPuppet> {
  await initRuntime();
  return WebPuppet.create(canvas, options.backend ?? "auto");
}

/** `loadModel`・`fetchModel` の設定。 */
export interface LoadModelOptions {
  /**
   * 読み込みの上限 (T0152)。省略すると既定値。超えたら `name` が `"LoadError"` で `code` が
   * `"read_limit_exceeded"` か `"image_limit_exceeded"` の例外 (`isLimitError` で見分ける)。JSON のモデルには効かない。
   */
  limits?: LoadLimits;
}

/**
 * モデルを読み込む。`.ugoe` バンドル (先頭が `UGOE`) なら `load_bundle`、それ以外は JSON として `load_json`。
 * 戻り値は警告の文 (英語) の配列。構造は `puppet.load_warnings_json()` (crates/ugoe-wasm/README.md#読み込みの警告-t0125)。
 */
export function loadModel(
  puppet: WebPuppet,
  data: ArrayBuffer | Uint8Array | string,
  options: LoadModelOptions = {},
): string[] {
  if (typeof data === "string") return puppet.load_json(data);
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  return isBundle(bytes)
    ? puppet.load_bundle(bytes, limitsJson(options.limits))
    : puppet.load_json(new TextDecoder().decode(bytes));
}

/** `url` のモデルを取ってきて `loadModel` で読み込む。戻り値は警告の文の配列。 */
export async function fetchModel(
  puppet: WebPuppet,
  url: string | URL,
  init?: RequestInit,
  options: LoadModelOptions = {},
): Promise<string[]> {
  const response = await fetch(url, init);
  if (!response.ok)
    throw new Error(`failed to fetch ${url}: ${response.status} ${response.statusText}`);
  return loadModel(puppet, await response.arrayBuffer(), options);
}

/**
 * モーション (`*.motion.json` の形の JSON) の各カーブを時刻 `t` (秒) で評価した値 (フェードなし。`sample_motion_json`)。
 * 値の無いカーブ (未知の対象など) は含めない。wasm を初期化 (`initRuntime`) してから呼ぶ。読めない JSON は例外。
 */
export function sampleMotion(json: string, t: number): UgoeCurveValue[] {
  return JSON.parse(sample_motion_json(json, t)) as UgoeCurveValue[];
}
