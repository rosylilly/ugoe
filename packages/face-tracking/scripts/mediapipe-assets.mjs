#!/usr/bin/env node
// フェイストラッキング (MediaPipe Face Landmarker と、体の Pose Landmarker) の実行に要るファイルを、アプリの public/mediapipe/ に用意する
// (ビューアとエディタの `pnpm mediapipe`。`pnpm dev` と `pnpm build` の前にも実行される)。
// 実行時にネットワークへ取りに行かないため。
//
// - wasm: npm パッケージ @mediapipe/tasks-vision の wasm/ から写す (SIMD 版と、SIMD の無い環境向けの版)。
// - ライセンスの表記: このパッケージの third-party-notices.txt を出力先に写す (アプリがライセンスとして表示する)。
// - モデル (face_landmarker.task・pose_landmarker_lite.task): 版を固定した URL から取得し、SHA-256 を検証する。
//   リポジトリにはコミットしない (public/mediapipe/ は gitignore 済み)。検証済みのファイルがあれば取得しない。
//
//   ugoe-mediapipe-assets <出力先>              取得できなければ失敗する (リリースビルド・CI)
//   ugoe-mediapipe-assets <出力先> --optional   取得できなくても警告だけで続ける (オフラインでの開発)
//
// 出力先は実行したディレクトリからの相対パス (アプリでは public/mediapipe)。
//
// モデルを更新するときは MODELS の url と sha256 を一緒に書き換える。

import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";

/**
 * 同梱するモデル。ライセンスはどれも Apache-2.0 (third-party-notices.txt)。
 *
 * - Face Landmarker (float16、版 1、3.6 MiB)。
 * - Pose Landmarker lite (float16、版 1、5.5 MiB、T0645)。体の向きと肩に使う。同じ版の full は 9.0 MiB
 *   (SHA-256 5134a3aa…)、heavy は 29.2 MiB (64437af8…)。重く、試した画像では lite と比べて安定しない
 *   ところもあったので (docs/tasks/T0645-pose-tracking.md のメモ)、lite だけを同梱する。
 */
const MODELS = [
  {
    name: "face_landmarker.task",
    url: "https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task",
    sha256: "64184e229b263107bc2b804c6625db1341ff2bb731874b0bcc2fe6544e0bc9ff",
  },
  {
    name: "pose_landmarker_lite.task",
    url: "https://storage.googleapis.com/mediapipe-models/pose_landmarker/pose_landmarker_lite/float16/1/pose_landmarker_lite.task",
    sha256: "59929e1d1ee95287735ddd833b19cf4ac46d29bc7afddbbf6753c459690d574a",
  },
];
/** tasks-vision が読む wasm (FilesetResolver.forVisionTasks が SIMD の有無で選ぶ)。 */
const WASM_FILES = [
  "vision_wasm_internal.js",
  "vision_wasm_internal.wasm",
  "vision_wasm_nosimd_internal.js",
  "vision_wasm_nosimd_internal.wasm",
];

const args = process.argv.slice(2);
const optional = args.includes("--optional");
const outArg = args.find((a) => !a.startsWith("--"));
if (outArg === undefined) {
  console.error("usage: ugoe-mediapipe-assets <output directory> [--optional]");
  process.exit(2);
}
/** @mediapipe/tasks-vision に依存しているこのパッケージ (packages/face-tracking)。 */
const packageDir = join(import.meta.dirname, "..");
const outDir = resolve(outArg);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function copyWasm() {
  const require = createRequire(join(packageDir, "package.json"));
  const wasmOut = join(outDir, "wasm");
  mkdirSync(wasmOut, { recursive: true });
  for (const name of WASM_FILES) {
    // パッケージの exports で、wasm/ のファイルは直下の名前で公開されている。
    copyFileSync(require.resolve(`@mediapipe/tasks-vision/${name}`), join(wasmOut, name));
  }
}

async function fetchModel({ name, url, sha256: expected }) {
  const path = join(outDir, name);
  if (existsSync(path) && sha256(readFileSync(path)) === expected) return;
  console.log(`mediapipe: downloading ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`);
  const bytes = Buffer.from(await res.arrayBuffer());
  const actual = sha256(bytes);
  if (actual !== expected) {
    // 取り違えや改ざんの可能性があるので、--optional でも失敗にする。
    console.error(`error: SHA-256 mismatch for ${name}: ${actual}`);
    process.exit(1);
  }
  mkdirSync(outDir, { recursive: true });
  writeFileSync(`${path}.tmp`, bytes);
  renameSync(`${path}.tmp`, path);
}

copyWasm();
copyFileSync(join(packageDir, "third-party-notices.txt"), join(outDir, "third-party-notices.txt"));
for (const model of MODELS) {
  try {
    await fetchModel(model);
  } catch (e) {
    const text = `could not download ${model.name}: ${e instanceof Error ? e.message : e}`;
    if (!optional) {
      console.error(`error: ${text}`);
      process.exit(1);
    }
    console.warn(`warning: ${text} (tracking with this model will be unavailable)`);
  }
}
