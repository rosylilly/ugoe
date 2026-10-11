#!/usr/bin/env node

// ランタイムを外へ出す MIT の配布物 (npm の @ugoe/runtime・Unity の UPM パッケージ・Godot のアドオン・C ABI のアーカイブ) が
// 同梱する、Rust のクレートのライセンスの表記を生成する。`--check` は生成したものとコミットしたものを突き合わせ、食い違えば
// 失敗する (`just licenses`)。手順は docs/release.md#mit-の配布物。
//
// 表記はアプリの表記の Rust のクレートの節 (scripts/third-party-notices.mjs) と同じ道具と形で作る: 配布物に入るクレート
// (`manifest` のクレートを `targets` のどれかに向けてビルドしたときに入るもの) を cargo-about にかけ、クレートの一覧と
// ライセンスの文を並べる (scripts/lib/rust-notices.mjs・scripts/about.toml)。MPL-2.0 で使うクレート (Godot の godot-rust) が
// あれば、その節のソースの在り処 (MPL-2.0 の 3.2 の求め) を足す。
//
// 使い方: `node scripts/dist-notices.mjs` (書き出す。`just notices`)・`node scripts/dist-notices.mjs --check` (検査。`just licenses`)・
// `--only <名前>` (1 つの配布物だけ。複数可)

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import {
  excludedSummary,
  renderRustNotices,
  rustNoticeSources,
  rustNotices,
} from "./lib/rust-notices.mjs";

const root = join(import.meta.dirname, "..");

/** Unity のネイティブライブラリのターゲット (.github/workflows/unity.yml の native の matrix。macOS は universal の 2 つ)。 */
const UNITY_TARGETS = [
  "x86_64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
  "aarch64-linux-android",
  "armv7-linux-androideabi",
  "x86_64-linux-android",
  "aarch64-apple-ios",
];

/** C ABI のアーカイブのターゲット (scripts/ffi-dist.mjs の PLATFORMS。macOS は universal の 2 つ)。 */
const FFI_TARGETS = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
];

/** Godot のアドオンのライブラリのターゲット (bindings/godot/example/addons/ugoe/ugoe.gdextension の libraries)。 */
const GODOT_TARGETS = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "x86_64-pc-windows-msvc",
  "aarch64-apple-darwin",
  "x86_64-apple-darwin",
];

/**
 * 表記を同梱する配布物。`manifest` は配布物に入るライブラリのクレート、`targets` はビルドするターゲット、`platforms` は表記に書く
 * その説明、`license` は ugoe 自身のライセンスの文の在り処、`out` は表記の置き場 (リポジトリのルートから)、`markdown` は Markdown として書く (Unity の慣例の名前)。
 */
export const DISTS = [
  {
    name: "web",
    title: "@ugoe/runtime",
    library: "the WebAssembly module (dist/wasm/ugoe_wasm_bg.wasm)",
    license: "the LICENSE file",
    platforms: "the wasm32-unknown-unknown build",
    manifest: "crates/ugoe-wasm/Cargo.toml",
    targets: ["wasm32-unknown-unknown"],
    out: "bindings/web/THIRD-PARTY-NOTICES.txt",
  },
  {
    name: "unity",
    title: "the ugoe Unity package (com.ugoe.unity)",
    library: "the native libraries (Runtime/Plugins/)",
    license: "LICENSE.md",
    platforms: "the builds for Windows, macOS, Linux, Android and iOS together",
    manifest: "crates/ugoe-ffi/Cargo.toml",
    targets: UNITY_TARGETS,
    out: "bindings/unity/com.ugoe.unity/Third Party Notices.md",
    markdown: true,
  },
  {
    name: "godot",
    title: "the ugoe Godot add-on (addons/ugoe)",
    library: "the GDExtension libraries (addons/ugoe/bin/)",
    license: "https://github.com/rosylilly/ugoe/blob/main/LICENSE",
    platforms: "the builds for Windows, macOS and Linux together",
    manifest: "bindings/godot/Cargo.toml",
    targets: GODOT_TARGETS,
    out: "bindings/godot/example/addons/ugoe/THIRD-PARTY-NOTICES.txt",
  },
  {
    // scripts/ffi-dist.mjs がアーカイブに詰める
    name: "ffi",
    title: "the ugoe C ABI archives (ugoe-ffi)",
    library: "the shared and static libraries (lib/)",
    license: "the LICENSE file",
    platforms: "the builds for Windows, macOS and Linux together",
    manifest: "crates/ugoe-ffi/Cargo.toml",
    targets: FFI_TARGETS,
    out: "crates/ugoe-ffi/THIRD-PARTY-NOTICES.txt",
  },
];

/** MPL-2.0 で使うクレートの、ソースの在り処の行 (無ければ空)。 */
export function mplSourceLines(crates) {
  const mpl = crates.filter((c) => c.used.includes("MPL-2.0"));
  if (mpl.length === 0) return [];
  const lines = [
    "",
    "",
    "Source code of the crates used under the Mozilla Public License 2.0",
    "------------------------------------------------------------------",
    "",
    "The following crates are used under the Mozilla Public License 2.0 (MPL-2.0) and",
    "are compiled into this distribution without modification. As required by",
    "section 3.2 of the MPL-2.0, their source code is available from crates.io (the",
    "exact versions below) and from their repositories:",
  ];
  for (const c of mpl) {
    lines.push(
      "",
      `${c.name} ${c.version}`,
      `  https://crates.io/crates/${c.name}/${c.version}`,
      `  https://static.crates.io/crates/${c.name}/${c.name}-${c.version}.crate`,
      `  ${c.source}`,
    );
  }
  return lines;
}

/** 配布物の表記の全文を組み立てる。`rust` は rustNotices (scripts/lib/rust-notices.mjs) の結果。 */
export function renderDistNotices(dist, rust) {
  const title = `Third-party software in ${dist.title}`;
  const intro = [
    `ugoe itself is licensed under the MIT License (see ${dist.license}).`,
    `This file lists the Rust crates compiled into ${dist.library},`,
    "which are distributed under their own licenses. It is generated from the crates",
    "of the distribution (scripts/dist-notices.mjs in the ugoe repository).",
  ];
  const body = [
    "Rust crates compiled into the distribution",
    "------------------------------------------",
    "",
    `The following Rust crates are compiled into ${dist.library}`,
    `(${dist.platforms}).`,
    "Where a crate is offered under a choice of licenses, the license under which it is",
    "used is given in parentheses. The license texts follow the list, each with the",
    "crates it applies to.",
    ...renderRustNotices(rust),
    ...mplSourceLines(rust.crates),
  ];
  if (dist.markdown) {
    // 一覧と文は整形済みの文なので、Markdown では 1 つのコードブロックに入れる (行の折り返しと字下げを保つ)
    return `${[`# ${title}`, "", ...intro, "", "```text", ...body, "```"].join("\n")}\n`;
  }
  return `${[title, "=".repeat(title.length), "", ...intro, "", "", ...body].join("\n")}\n`;
}

/** 配布物の表記を生成する。`log` は除いたクレートの数を受け取る。 */
export function generate(dist, log = () => {}) {
  const { about, compileOnly } = rustNoticeSources(root, dist.manifest, { targets: dist.targets });
  const rust = rustNotices(about, { compileOnly });
  log(excludedSummary(rust));
  return renderDistNotices(dist, rust);
}

function main(argv) {
  const check = argv.includes("--check");
  const only = argv.flatMap((a, i) => (argv[i - 1] === "--only" ? [a] : []));
  const unknown = only.filter((name) => !DISTS.some((d) => d.name === name));
  if (unknown.length > 0) {
    console.error(`error: unknown distribution ${unknown.join(", ")}`);
    process.exitCode = 2;
    return;
  }
  const dists = only.length > 0 ? DISTS.filter((d) => only.includes(d.name)) : DISTS;
  let failed = false;
  for (const dist of dists) {
    const path = join(root, dist.out);
    const text = generate(dist, (line) => check || console.log(`${dist.out}: ${line}`));
    const current = existsSync(path) ? readFileSync(path, "utf8") : undefined;
    if (current === text) continue;
    if (check) {
      console.error(
        `error: ${dist.out} が依存と食い違う (just notices で生成し直してコミットする。docs/release.md#mit-の配布物)`,
      );
      failed = true;
    } else {
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, text);
      console.log(`wrote ${dist.out}`);
    }
  }
  if (check && !failed) console.log(`distribution notices ok (${dists.length} distributions)`);
  process.exitCode = failed ? 1 : 0;
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
