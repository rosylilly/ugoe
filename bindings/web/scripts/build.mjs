#!/usr/bin/env node
// @ugoe/runtime をビルドする (`pnpm --filter @ugoe/runtime build`、`just web-runtime`)。
//
// 1. ugoe-wasm を大きさを優先したプロファイルでビルドし、JS バインディングを src/wasm/ に生成する (gitignore 済み)
// 2. TypeScript を dist/ に出力する
// 3. 生成したバインディングと wasm を dist/wasm/ に写す
//
// `--no-wasm` を付けると 1 を飛ばし、src/wasm/ にある生成物を使う。`--wasm-only` を付けると 1 だけを行う
// (`just wasm-fresh` が古い src/wasm/ を作り直すとき。docs/testing.md#wasm-の生成物の古さ)。
//
// 1 は生成物の隣に、`just wasm` と同じ入力の指紋 (scripts/wasm.mjs の `fingerprint`。ugoe_wasm.fingerprint) を書く。
// `just wasm-fresh` はこれで src/wasm/ の古さを確かめる。指紋は dist/wasm/ には写さない。
//
// 1 は `just wasm` (scripts/wasm.mjs。エディタとビューアが使う) と次の点が違う (数字は bindings/web/README.md#wasm-の大きさ)。
// - cargo のプロファイル `web-dist` (下の `PROFILE`) を `--config` で定義して使う。release を継承し、
//   opt-level = "z"・lto = "fat"・codegen-units = 1 にする (毎フレーム動くクレートだけ opt-level = 3。`HOT_CRATES`)。
//   ルートの Cargo.toml の release は変えないので、エディタ・ビューアの wasm とは別の成果物
//   (target/wasm32-unknown-unknown/web-dist/) になる。
// - wasm-bindgen に `--remove-name-section` を付け、関数名の節 (パニックのスタックトレース用。約 2〜4MB) を外す。
// - wasm-opt (binaryen) は使わない。生の大きさは約 1 割減るが、gzip した大きさはかえって増えたため。

import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, join } from "node:path";
import { exportMbxPath } from "../../../scripts/lib/mbx.mjs";
import {
  FINGERPRINT_FILE,
  fingerprint,
  requireWasmBindgen,
  WASM_FILE,
  writeFingerprint,
} from "../../../scripts/wasm.mjs";

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
exportMbxPath();

const pkg = join(import.meta.dirname, "..");
const root = join(pkg, "..", "..");
const generated = join(pkg, "src", "wasm");
const dist = join(pkg, "dist");

/** パッケージの wasm だけに使う cargo のプロファイル。 */
const PROFILE = "web-dist";
const PROFILE_CONFIG = {
  inherits: '"release"',
  "opt-level": '"z"',
  lto: '"fat"',
  "codegen-units": "1",
};
/**
 * 毎フレーム動く (変形・物理・モーション・描画の準備) ので、速さを優先する (opt-level = 3) クレート。
 * すべてを "z" にすると評価が約 2.5 倍遅くなった。これらだけを 3 に戻すと release と同じ速さで、
 * 大きさは約 2% 増えるだけだった (bindings/web/README.md#wasm-の大きさ)。
 * libm (ugoe_core::math と glam の三角関数など) も物理が毎フレーム呼ぶので入れる。"z" のままでは物理の段が約 1.3 倍遅かった (T0193)。
 */
const HOT_CRATES = [
  "glam",
  "libm",
  "ugoe",
  "ugoe-core",
  "ugoe-motion",
  "ugoe-physics",
  "ugoe-render",
  "ugoe-runtime",
];

/** コマンドを実行し、失敗したらその終了コードで終える。 */
function run(cmd, args, cwd = pkg) {
  const r = spawnSync(cmd, args, { cwd, stdio: "inherit" });
  if (r.error) {
    console.error(`error: failed to run ${cmd}: ${r.error.message}`);
    process.exit(1);
  }
  if (r.status !== 0) process.exit(r.status ?? 1);
}

function buildWasm() {
  requireWasmBindgen();
  // ビルドの前に求める (ビルドの間にソースが変われば、次の確かめで古いとみなされる。scripts/wasm.mjs と同じ)
  const print = fingerprint();
  const config = [
    ...Object.entries(PROFILE_CONFIG).map(([k, v]) => `profile.${PROFILE}.${k}=${v}`),
    ...HOT_CRATES.map((c) => `profile.${PROFILE}.package.${c}.opt-level=3`),
  ].flatMap((c) => ["--config", c]);
  run(
    "cargo",
    [
      ...config,
      "build",
      "-p",
      "ugoe-wasm",
      "--target",
      "wasm32-unknown-unknown",
      "--profile",
      PROFILE,
    ],
    root,
  );
  // CARGO_TARGET_DIR などで target/ の場所が変わっていても追えるよう、cargo に尋ねる
  const { target_directory } = JSON.parse(
    execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
  const wasm = join(target_directory, "wasm32-unknown-unknown", PROFILE, "ugoe_wasm.wasm");
  rmSync(generated, { recursive: true, force: true });
  run(
    "wasm-bindgen",
    [
      "--target",
      "web",
      "--remove-name-section",
      "--remove-producers-section",
      "--out-dir",
      generated,
      wasm,
    ],
    root,
  );
  writeFingerprint(generated, print);
}

if (!process.argv.includes("--no-wasm")) buildWasm();
if (process.argv.includes("--wasm-only")) {
  console.log(`generated ${generated}`);
  process.exit(0);
}
if (!existsSync(join(generated, WASM_FILE))) {
  console.error(`error: src/wasm/${WASM_FILE} is missing (build without --no-wasm)`);
  process.exit(1);
}

rmSync(dist, { recursive: true, force: true });
const tsc = createRequire(import.meta.url).resolve("typescript/bin/tsc");
run(process.execPath, [tsc, "-p", "tsconfig.build.json"]);
cpSync(generated, join(dist, "wasm"), {
  recursive: true,
  filter: (src) => basename(src) !== FINGERPRINT_FILE,
});
console.log(`built ${dist}`);
