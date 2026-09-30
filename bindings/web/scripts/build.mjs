#!/usr/bin/env node
// @ugoe/runtime をビルドする (`pnpm --filter @ugoe/runtime build`、`just web-runtime`)。
//
// 1. ugoe-wasm を大きさを優先したプロファイルでビルドし、JS バインディングを src/wasm/ に生成する (gitignore 済み)
// 2. TypeScript を dist/ に出力する
// 3. 生成したバインディングと wasm を dist/wasm/ に写す
//
// `--no-wasm` を付けると 1 を飛ばし、src/wasm/ にある生成物を使う。
//
// 1 は `just wasm` (scripts/wasm.mjs。エディタとビューアが使う) と次の点が違う (数字は bindings/web/README.md#wasm-の大きさ)。
// - cargo のプロファイル `web-dist` (下の `PROFILE`) を `--config` で定義して使う。release を継承し、
//   opt-level = "z"・lto = "fat"・codegen-units = 1 にする (毎フレーム動くクレートだけ opt-level = 3。`HOT_CRATES`)。
//   ルートの Cargo.toml の release は変えないので、エディタ・ビューアの wasm とは別の成果物
//   (target/wasm32-unknown-unknown/web-dist/) になる。
// - wasm-bindgen に `--remove-name-section` を付け、関数名の節 (パニックのスタックトレース用。約 2〜4MB) を外す。
// - wasm-opt (binaryen) は使わない。生の大きさは約 1 割減るが、gzip した大きさはかえって増えたため。

import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, readFileSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

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
 */
const HOT_CRATES = [
  "glam",
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

/** Cargo.lock に記録された wasm-bindgen の版 (scripts/wasm.mjs と同じ検査)。 */
function lockedBindgenVersion() {
  const lock = readFileSync(join(root, "Cargo.lock"), "utf8");
  return lock.match(/^name = "wasm-bindgen"\r?\nversion = "([^"]+)"/m)?.[1] ?? null;
}

function buildWasm() {
  const want = lockedBindgenVersion();
  const r = spawnSync("wasm-bindgen", ["--version"], { encoding: "utf8" });
  const have = r.status === 0 ? (r.stdout.trim().split(/\s+/)[1] ?? null) : null;
  if (!want || have !== want) {
    console.error(`error: wasm-bindgen-cli ${want} is required (found: ${have ?? "none"}).`);
    console.error(`  cargo install wasm-bindgen-cli --version ${want} --locked`);
    process.exit(1);
  }
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
}

if (!process.argv.includes("--no-wasm")) buildWasm();
if (!existsSync(join(generated, "ugoe_wasm_bg.wasm"))) {
  console.error("error: src/wasm/ugoe_wasm_bg.wasm is missing (build without --no-wasm)");
  process.exit(1);
}

rmSync(dist, { recursive: true, force: true });
const tsc = createRequire(import.meta.url).resolve("typescript/bin/tsc");
run(process.execPath, [tsc, "-p", "tsconfig.build.json"]);
cpSync(generated, join(dist, "wasm"), { recursive: true });
console.log(`built ${dist}`);
