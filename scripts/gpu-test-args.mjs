#!/usr/bin/env node
// `just test-gpu` 用に、`gpu-tests` feature を持つワークスペースのクレートを探し、
// `cargo test` に渡す引数 (`-p <crate> --features <crate>/gpu-tests` の並び) を出力する。
//
// クレートに `gpu-tests` feature を足すだけで `just test-gpu` と CI の対象になる。

import { execFileSync } from "node:child_process";

const FEATURE = "gpu-tests";

const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }),
);

const crates = metadata.packages
  .filter((p) => metadata.workspace_members.includes(p.id) && FEATURE in p.features)
  .map((p) => p.name)
  .sort();

if (crates.length === 0) {
  console.error(`no workspace crate has a \`${FEATURE}\` feature`);
  process.exit(1);
}

console.log(crates.flatMap((name) => ["-p", name, "--features", `${name}/${FEATURE}`]).join(" "));
