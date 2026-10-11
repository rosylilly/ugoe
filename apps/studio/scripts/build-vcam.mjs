#!/usr/bin/env node
// 仮想カメラのメディアソースの DLL (ugoe_studio_vcam.dll) を作る (`pnpm vcam`)。Windows 向けのビルドの前に
// tauri.windows.conf.json の beforeBuildCommand / beforeDevCommand が呼び、インストーラ (src-tauri/windows/hooks.nsh) が入れる。
//
// - 出力は <リポジトリ>/target/vcam/x86_64-pc-windows-msvc/release/ugoe_studio_vcam.dll。ほかのビルドと RUSTFLAGS が違うので
//   target を分ける。
// - Frame Server (OS のサービス) が読み込むので、VC++ のランタイムに頼らないよう CRT を静的にリンクする。
// - Windows では cargo、ほかの OS では cargo-xwin でクロスビルドする (apps/editor/README.md の Windows 向けのクロスビルドと同じ道具)。
// - 署名のコマンド (UGOE_WINDOWS_SIGN_COMMAND) があれば DLL に署名する。DLL はインストーラのフックが入れるので、Tauri の
//   signCommand は署名しない (docs/release.md#デスクトップアプリの署名)。無ければ署名しない。

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { signWindowsFile } from "../../../scripts/lib/app-signing.mjs";
import { exportMbxPath } from "../../../scripts/lib/mbx.mjs";

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
exportMbxPath();

const root = join(import.meta.dirname, "..", "..", "..");
const target = "x86_64-pc-windows-msvc";
const args = [
  "build",
  "-p",
  "ugoe-studio-vcam",
  "--lib",
  "--release",
  "--target",
  target,
  "--target-dir",
  join(root, "target", "vcam"),
];
const [command, commandArgs] =
  process.platform === "win32" ? ["cargo", args] : ["cargo", ["xwin", ...args]];
const rustflags = [process.env.RUSTFLAGS, "-C target-feature=+crt-static"]
  .filter(Boolean)
  .join(" ");
const result = spawnSync(command, commandArgs, {
  cwd: root,
  stdio: "inherit",
  env: { ...process.env, RUSTFLAGS: rustflags },
});
if (result.error) {
  console.error(`error: ${result.error.message}`);
  process.exit(1);
}
if (result.status !== 0) process.exit(result.status ?? 1);
const dll = join(root, "target", "vcam", target, "release", "ugoe_studio_vcam.dll");
// インストーラのフックはこの場所の DLL を入れる。無ければ makensis より先に、ここで分かるように止まる
if (!existsSync(dll)) {
  console.error(`error: cargo finished but ${dll} was not built`);
  process.exit(1);
}
try {
  if (signWindowsFile(dll)) console.error(`signed ${dll}`);
} catch (e) {
  console.error(`error: ${e.message}`);
  process.exit(1);
}
