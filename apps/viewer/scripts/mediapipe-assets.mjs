#!/usr/bin/env node
// フェイストラッキング (MediaPipe Face Landmarker) の実行に要るファイルを public/mediapipe/ に用意する
// (`pnpm mediapipe`。`pnpm dev` と `pnpm build` の前にも実行される)。実行時にネットワークへ取りに行かないため。
//
// - wasm: npm パッケージ @mediapipe/tasks-vision の wasm/ から写す (SIMD 版と、SIMD の無い環境向けの版)。
// - モデル (face_landmarker.task): 版を固定した URL から取得し、SHA-256 を検証する。リポジトリにはコミットしない
//   (public/mediapipe/ は gitignore 済み)。検証済みのファイルがあれば取得しない。
//
//   node scripts/mediapipe-assets.mjs              取得できなければ失敗する (リリースビルド・CI)
//   node scripts/mediapipe-assets.mjs --optional   取得できなくても警告だけで続ける (オフラインでの開発)
//
// モデルを更新するときは MODEL_URL と MODEL_SHA256 を一緒に書き換える。

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
import { join } from "node:path";

/** Face Landmarker のモデル (float16、版 1)。モデルのライセンスは Apache-2.0。 */
const MODEL_URL =
  "https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task";
const MODEL_SHA256 = "64184e229b263107bc2b804c6625db1341ff2bb731874b0bcc2fe6544e0bc9ff";
/** tasks-vision が読む wasm (FilesetResolver.forVisionTasks が SIMD の有無で選ぶ)。 */
const WASM_FILES = [
  "vision_wasm_internal.js",
  "vision_wasm_internal.wasm",
  "vision_wasm_nosimd_internal.js",
  "vision_wasm_nosimd_internal.wasm",
];

const optional = process.argv.includes("--optional");
const appDir = join(import.meta.dirname, "..");
const outDir = join(appDir, "public", "mediapipe");

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function copyWasm() {
  const require = createRequire(join(appDir, "package.json"));
  const wasmOut = join(outDir, "wasm");
  mkdirSync(wasmOut, { recursive: true });
  for (const name of WASM_FILES) {
    // パッケージの exports で、wasm/ のファイルは直下の名前で公開されている。
    copyFileSync(require.resolve(`@mediapipe/tasks-vision/${name}`), join(wasmOut, name));
  }
}

async function fetchModel() {
  const path = join(outDir, "face_landmarker.task");
  if (existsSync(path) && sha256(readFileSync(path)) === MODEL_SHA256) return;
  console.log(`mediapipe: downloading ${MODEL_URL}`);
  const res = await fetch(MODEL_URL);
  if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`);
  const bytes = Buffer.from(await res.arrayBuffer());
  const actual = sha256(bytes);
  if (actual !== MODEL_SHA256) {
    // 取り違えや改ざんの可能性があるので、--optional でも失敗にする。
    console.error(`error: SHA-256 mismatch for face_landmarker.task: ${actual}`);
    process.exit(1);
  }
  mkdirSync(outDir, { recursive: true });
  writeFileSync(`${path}.tmp`, bytes);
  renameSync(`${path}.tmp`, path);
}

copyWasm();
try {
  await fetchModel();
} catch (e) {
  const text = `could not download the face landmarker model: ${e instanceof Error ? e.message : e}`;
  if (!optional) {
    console.error(`error: ${text}`);
    process.exit(1);
  }
  console.warn(`warning: ${text} (face tracking will be unavailable)`);
}
