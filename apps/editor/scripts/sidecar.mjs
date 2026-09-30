// インストーラに同梱する ugoe-mcp (Tauri のサイドカー) を作る (docs/editor.md#エージェントをつなぐ)。
//
//   node scripts/sidecar.mjs [ターゲットの三つ組]
//
// `tauri build --config src-tauri/tauri.bundle.conf.json` (just editor-build) の beforeBuildCommand から呼ばれる。
// ターゲットは引数、無ければ Tauri が渡す TAURI_ENV_TARGET_TRIPLE、それも無ければ手元の rustc のホスト。
// ugoe-mcp をリリースでビルドし、Tauri の externalBin の名前の規則どおり
// src-tauri/binaries/ugoe-mcp-<三つ組>[.exe] に置く。
// - Linux などから Windows (*-windows-msvc) 向けに作るときは cargo-xwin を使う (just editor-build-windows と同じ)。
// - universal-apple-darwin は aarch64 と x86_64 を作って lipo で 1 つにする。
// tauri.conf.json には externalBin を書かない (cargo check や tauri dev がサイドカーのビルドを待たずに済むように)。
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..");
const binaries = join(here, "..", "src-tauri", "binaries");

function run(cmd, args) {
  console.error(`sidecar: ${cmd} ${args.join(" ")}`);
  const r = spawnSync(cmd, args, { cwd: root, stdio: "inherit" });
  if (r.error) throw new Error(`failed to run ${cmd}: ${r.error.message}`);
  if (r.status !== 0) process.exit(r.status ?? 1);
}

function hostTriple() {
  const r = spawnSync("rustc", ["-vV"], { cwd: root, encoding: "utf8" });
  const line = (r.stdout ?? "").split("\n").find((l) => l.startsWith("host: "));
  if (!line) throw new Error("could not read the host triple from `rustc -vV`");
  return line.slice("host: ".length).trim();
}

/** `triple` 向けに ugoe-mcp をビルドし、できた実行ファイルのパスを返す。 */
function build(triple, host) {
  const exe = triple.includes("windows") ? "ugoe-mcp.exe" : "ugoe-mcp";
  if (triple === host) {
    run("cargo", ["build", "--release", "--locked", "-p", "ugoe-mcp"]);
    return join(root, "target", "release", exe);
  }
  const cross = triple.endsWith("-windows-msvc") && !host.includes("windows");
  const args = ["build", "--release", "--locked", "-p", "ugoe-mcp", "--target", triple];
  run("cargo", cross ? ["xwin", ...args] : args);
  return join(root, "target", triple, "release", exe);
}

const host = hostTriple();
const triple = process.argv[2] ?? process.env.TAURI_ENV_TARGET_TRIPLE ?? host;
mkdirSync(binaries, { recursive: true });
if (triple === "universal-apple-darwin") {
  const parts = ["aarch64-apple-darwin", "x86_64-apple-darwin"].map((t) => build(t, host));
  run("lipo", ["-create", "-output", join(binaries, `ugoe-mcp-${triple}`), ...parts]);
} else {
  const ext = triple.includes("windows") ? ".exe" : "";
  const dest = join(binaries, `ugoe-mcp-${triple}${ext}`);
  copyFileSync(build(triple, host), dest);
  console.error(`sidecar: wrote ${dest}`);
}
