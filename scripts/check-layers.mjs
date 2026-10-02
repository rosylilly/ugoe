#!/usr/bin/env node
// クレート間の依存方向を検査する。ルールの正は docs/architecture.md#レイヤーと依存ルール。
//
// 新しいクレートを追加したら ALLOWED にエントリを追加すること (未登録クレートはエラー)。

import { execFileSync } from "node:child_process";

const union = (...sets) => new Set(sets.flatMap((s) => [...s]));
const without = (set, ...names) => new Set([...set].filter((n) => !names.includes(n)));

const CORE = new Set(["ugoe-core"]);
const DATA = union(CORE, ["ugoe-model"]);
const EVAL = union(DATA, ["ugoe-runtime"]);
const RUNTIME_TIER = union(EVAL, [
  "ugoe-format",
  "ugoe-physics",
  "ugoe-motion",
  "ugoe-render",
  "ugoe",
]);
const ASSETS = new Set(["ugoe-psd", "ugoe-mesher"]);
const RIGGING = new Set(["ugoe-rigging"]);
// ヘッドレス描画 (ugoe render と MCP サーバの render_png が共有する)
const HEADLESS = new Set(["ugoe-headless"]);
const TOOL_BASE = union(RUNTIME_TIER, ASSETS, RIGGING, HEADLESS, ["ugoe-editor-core"]);

// クレート -> 依存してよい内部クレート
const ALLOWED = {
  // --- ランタイム層: ツール層 (PSD・メッシャ・エディタ・AI) に依存してはならない ---
  "ugoe-core": new Set(),
  "ugoe-model": CORE,
  "ugoe-runtime": DATA,
  "ugoe-format": DATA,
  "ugoe-physics": EVAL,
  "ugoe-motion": EVAL,
  "ugoe-render": EVAL,
  ugoe: without(RUNTIME_TIER, "ugoe"),
  "ugoe-ffi": RUNTIME_TIER,
  "ugoe-wasm": RUNTIME_TIER,
  // ビューア (apps/viewer): 描画は webview の ugoe-wasm が行う。Rust 側はランタイム層だけに依存してよい
  "ugoe-viewer": RUNTIME_TIER,
  // スタジオ (apps/studio、ADR 0028 の 1.): 指揮役はネイティブの Puppet。ランタイム層と ugoe-headless (仮想カメラの常駐の描画) に依存してよい。
  // エディタの中核と AI には依存しない (スタジオはモデルを編集しない)
  "ugoe-studio": union(RUNTIME_TIER, HEADLESS, ["ugoe-studio-vcam"]),
  // スタジオの仮想カメラ (apps/studio/vcam、ADR 0028 の 8.): OS の境界だけを持ち、内部のクレートに依存しない
  // (Windows の Frame Server が読み込む DLL にもなるので、小さく保つ)。unsafe を許す唯一のアプリのクレート
  "ugoe-studio-vcam": new Set(),
  // --- ツール層 ---
  "ugoe-psd": CORE,
  "ugoe-mesher": CORE,
  "ugoe-rigging": union(EVAL, ["ugoe-mesher"]),
  "ugoe-headless": RUNTIME_TIER,
  "ugoe-editor-core": union(RUNTIME_TIER, ASSETS, RIGGING),
  "ugoe-ai": TOOL_BASE,
  "ugoe-cli": union(TOOL_BASE, ["ugoe-ai"]),
  "ugoe-mcp": union(TOOL_BASE, ["ugoe-ai"]),
  "ugoe-editor": union(TOOL_BASE, ["ugoe-ai", "ugoe-mcp"]),
  // 配布物の適合テスト (tests/conformance): 参照は ugoe と ugoe render と同じ関数 (ugoe-headless) で作る
  "ugoe-conformance": union(RUNTIME_TIER, HEADLESS),
};

const meta = JSON.parse(
  execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }),
);
const members = new Set(meta.packages.map((p) => p.name));
const errors = [];
for (const pkg of meta.packages) {
  const allowed = ALLOWED[pkg.name];
  if (allowed === undefined) {
    errors.push(`${pkg.name}: scripts/check-layers.mjs の ALLOWED に未登録`);
    continue;
  }
  for (const dep of pkg.dependencies) {
    if (!members.has(dep.name) || dep.kind === "dev") continue;
    if (!allowed.has(dep.name)) {
      errors.push(`${pkg.name} -> ${dep.name}: 許可されていない依存方向`);
    }
  }
}
for (const e of errors) console.error(`error: ${e}`);
if (errors.length === 0) console.log(`layers ok (${members.size} crates)`);
process.exitCode = errors.length > 0 ? 1 : 0;
