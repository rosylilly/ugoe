#!/usr/bin/env node
// `just editor-build-windows` の実体。Linux (WSL2 を含む) から Windows x86_64 向けのエディタを
// クロスビルドする (NSIS のインストーラと実行ファイル。署名なし)。手順は apps/editor/README.md。
//
//   node scripts/editor-windows.mjs [tauri build の追加の引数...]
//
// 必要な道具がそろっているかを先に調べ、足りなければ入れ方を示して止まる。
// MSVC の CRT と Windows SDK は cargo-xwin が初回に自分のキャッシュ (~/.cache/cargo-xwin) へ
// ダウンロードする (Microsoft のライセンスへの同意は利用者の責任。リポジトリには置かない)。

import { spawnSync } from "node:child_process";
import { accessSync, constants, readFileSync } from "node:fs";
import { delimiter, join } from "node:path";

const root = join(import.meta.dirname, "..");
const target = "x86_64-pc-windows-msvc";

/** `rust-toolchain.toml` で固定された toolchain の版。 */
function pinnedToolchain() {
  const toml = readFileSync(join(root, "rust-toolchain.toml"), "utf8");
  const m = toml.match(/^channel\s*=\s*"([^"]+)"/m);
  if (!m) {
    console.error("error: channel is not found in rust-toolchain.toml");
    process.exit(1);
  }
  return m[1];
}

/** `PATH` からコマンドを探す。 */
function onPath(name) {
  return (process.env.PATH ?? "").split(delimiter).some((dir) => {
    try {
      accessSync(join(dir, name), constants.X_OK);
      return true;
    } catch {
      return false;
    }
  });
}

/** コマンドの出力 (失敗したら null)。 */
function output(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: root, encoding: "utf8" });
  return r.status === 0 ? r.stdout : null;
}

/** 足りない道具と、その入れ方の一覧。 */
function missingTools() {
  const missing = [];
  const toolchain = pinnedToolchain();
  const targets = output("rustup", ["target", "list", "--installed", "--toolchain", toolchain]);
  if (targets === null) {
    missing.push(["rustup", "https://rustup.rs/"]);
  } else if (!targets.split(/\s+/).includes(target)) {
    missing.push([
      `Rust target ${target} (toolchain ${toolchain})`,
      `rustup target add ${target} --toolchain ${toolchain}`,
    ]);
  }
  if (output("cargo", ["xwin", "--version"]) === null) {
    missing.push(["cargo-xwin", "cargo install cargo-xwin --locked"]);
  }
  // Debian / Ubuntu のパッケージ名。clang は C のコード、lld-link はリンク、
  // llvm-rc と llvm-lib はアイコンなどのリソースと静的ライブラリ、makensis はインストーラに使う
  const commands = [
    ["clang", "clang"],
    ["lld-link", "lld"],
    ["llvm-rc", "llvm"],
    ["llvm-lib", "llvm"],
    ["makensis", "nsis"],
  ];
  for (const [command, pkg] of commands) {
    if (!onPath(command)) missing.push([command, `sudo apt install ${pkg}`]);
  }
  return missing;
}

if (process.platform === "win32") {
  console.error(
    "error: on Windows, build natively with `just editor-build` (see apps/editor/README.md).",
  );
  process.exit(1);
}

const missing = missingTools();
if (missing.length > 0) {
  console.error(
    "error: the Windows cross build needs the following tools (see apps/editor/README.md):",
  );
  for (const [tool, install] of missing) console.error(`  ${tool}: ${install}`);
  process.exit(1);
}

// tauri.conf.json の beforeBuildCommand で `just wasm` も走る。
// MSI (WiX) は Windows でしか作れないので NSIS だけを作る
const args = [
  "--filter",
  "@ugoe/editor",
  "tauri",
  "build",
  "--runner",
  "cargo-xwin",
  "--target",
  target,
  "--bundles",
  "nsis",
  ...process.argv.slice(2),
];
const r = spawnSync("pnpm", args, { cwd: root, stdio: "inherit" });
if (r.error) {
  console.error(`error: failed to run pnpm: ${r.error.message}`);
  process.exit(1);
}
process.exit(r.status ?? 1);
