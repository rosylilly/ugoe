#!/usr/bin/env node
// `just test-gpu` 用に、`gpu-tests` feature を持つワークスペースのクレートを探し、
// `cargo test` に渡す引数 (`-p <crate> --features <crate>/gpu-tests` の並び) を出力する。
//
// クレートに `gpu-tests` feature を足すだけで `just test-gpu` と CI の対象になる。
//
// ただし Tauri のアプリ (scripts/ci-groups.mjs で `tauri: true` のグループのクレート。スタジオの仮想カメラの受け手など) は、
// CI (環境変数 `CI` がある) では除く。CI の gpu ジョブは Tauri のシステム依存 (webkit など) を入れないため
// (.github/workflows/release-checks.yml)。手元の `just test-gpu` では含める。

import { execFileSync } from "node:child_process";
import { GROUPS } from "./ci-groups.mjs";
import { exportMbxPath } from "./lib/mbx.mjs";

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
exportMbxPath();

const FEATURE = "gpu-tests";

const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }),
);

const tauriApps = new Set(
  Object.values(GROUPS)
    .filter((g) => g.tauri)
    .flatMap((g) => g.crates),
);
const skipApps = Boolean(process.env.CI);

const found = metadata.packages
  .filter((p) => metadata.workspace_members.includes(p.id) && FEATURE in p.features)
  .map((p) => p.name)
  .sort();
const crates = found.filter((name) => !(skipApps && tauriApps.has(name)));
const skipped = found.filter((name) => !crates.includes(name));
if (skipped.length > 0) {
  console.error(`gpu tests: skipping Tauri apps on CI (no webkit there): ${skipped.join(", ")}`);
}

if (crates.length === 0) {
  console.error(`no workspace crate has a \`${FEATURE}\` feature`);
  process.exit(1);
}

console.log(crates.flatMap((name) => ["-p", name, "--features", `${name}/${FEATURE}`]).join(" "));
