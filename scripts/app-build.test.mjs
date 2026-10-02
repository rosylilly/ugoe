// scripts/app-build.mjs の引数と組み合わせの検査のテスト (`just app-build-check`)
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  APPS,
  hostPlatform,
  parseArgs,
  planAll,
  planBuild,
  unsupportedReason,
} from "./app-build.mjs";

test("引数がなければすべてのアプリ、OS は呼ぶ側が今の OS で埋める", () => {
  const parsed = parseArgs([], {});
  assert.deepEqual(parsed.apps, Object.keys(APPS));
  assert.deepEqual(parsed.platforms, []);
  assert.equal(parsed.vcam, false);
  assert.deepEqual(parsed.extra, []);
});

test("TARGET と PLATFORM はカンマ区切りで、大文字小文字を問わず、重複を除く", () => {
  const parsed = parseArgs(["target=Editor,viewer,editor", "PLATFORM=windows,linux"], {});
  assert.deepEqual(parsed.apps, ["editor", "viewer"]);
  assert.deepEqual(parsed.platforms, ["windows", "linux"]);
  assert.deepEqual(parseArgs(["TARGET=all"], {}).apps, Object.keys(APPS));
});

test("PowerShell が分けたカンマ区切りの続きを同じ指定として読む", () => {
  const parsed = parseArgs(["TARGET=editor", "viewer", "PLATFORM=windows", "linux", "--debug"], {});
  assert.deepEqual(parsed.apps, ["editor", "viewer"]);
  assert.deepEqual(parsed.platforms, ["windows", "linux"]);
  assert.deepEqual(parsed.extra, ["--debug"]);
});

test("それ以外の引数は tauri build に渡す", () => {
  const parsed = parseArgs(["--bundles", "deb", "TARGET=viewer", "--config", "a=b", "VCAM=1"], {});
  assert.deepEqual(parsed.extra, ["--bundles", "deb", "--config", "a=b"]);
  assert.equal(parsed.vcam, true);
  assert.equal(parseArgs(["VCAM=0"], {}).vcam, false);
});

test("知らないアプリ・OS・大文字の指定は失敗する", () => {
  assert.match(parseArgs(["TARGET=foo"], {}).error, /unknown TARGET foo/);
  assert.match(parseArgs(["PLATFORM=android"], {}).error, /unknown PLATFORM android/);
  assert.match(parseArgs(["TRAGET=editor"], {}).error, /unknown option TRAGET=/);
});

test("今の OS の名前", () => {
  assert.equal(hostPlatform("linux"), "linux");
  assert.equal(hostPlatform("win32"), "windows");
  assert.equal(hostPlatform("darwin"), "macos");
  assert.equal(hostPlatform("freebsd"), null);
});

test("作れる組み合わせは、同じ OS と Linux から Windows だけ", () => {
  for (const p of ["windows", "macos", "linux"]) assert.equal(unsupportedReason(p, p), null);
  assert.equal(unsupportedReason("linux", "windows"), null);
  assert.match(unsupportedReason("linux", "macos"), /only be built on macOS/);
  assert.match(unsupportedReason("windows", "linux"), /WSL2/);
  assert.match(unsupportedReason("macos", "windows"), /Editor build/);
});

test("作れない組み合わせが 1 つでもあれば、何も作らない", () => {
  const plan = planAll({
    apps: ["editor"],
    platforms: ["linux", "macos"],
    host: "linux",
    vcam: false,
    extra: [],
  });
  assert.equal(plan.builds, undefined);
  assert.equal(plan.errors.length, 1);
  assert.match(plan.errors[0], /^macos from linux/);
});

test("エディタはどの OS でもサイドカーの設定を重ねる", () => {
  const b = planBuild({ app: "editor", platform: "linux", host: "linux" });
  assert.deepEqual(b.args, [
    "--filter",
    "@ugoe/editor",
    "tauri",
    "build",
    "--config",
    "src-tauri/tauri.bundle.conf.json",
  ]);
  assert.equal(b.bundleDir, "target/release/bundle");
  assert.equal(b.dist, "target/dist/editor/linux");
});

test("Linux から Windows 向けは cargo-xwin で NSIS だけを作る", () => {
  const b = planBuild({ app: "studio", platform: "windows", host: "linux", extra: ["--debug"] });
  assert.equal(b.cross, true);
  assert.deepEqual(b.args.slice(4), [
    "--runner",
    "cargo-xwin",
    "--target",
    "x86_64-pc-windows-msvc",
    "--bundles",
    "nsis",
    "--debug",
  ]);
  assert.equal(b.bundleDir, "target/x86_64-pc-windows-msvc/debug/bundle");
});

test("利用者の --target と --bundles があれば、既定のものを足さない", () => {
  const b = planBuild({
    app: "viewer",
    platform: "macos",
    host: "macos",
    extra: ["--target", "aarch64-apple-darwin", "--bundles=dmg"],
  });
  assert.deepEqual(b.args.slice(4), ["--target", "aarch64-apple-darwin", "--bundles=dmg"]);
  assert.equal(b.bundleDir, "target/aarch64-apple-darwin/release/bundle");
});

test("macOS は既定で universal、スタジオは VCAM=1 で Camera Extension の設定を重ねる", () => {
  const plain = planBuild({ app: "studio", platform: "macos", host: "macos" });
  assert.deepEqual(plain.args.slice(4), ["--target", "universal-apple-darwin"]);
  const vcam = planBuild({ app: "studio", platform: "macos", host: "macos", vcam: true });
  assert.deepEqual(vcam.args.slice(4), [
    "--config",
    "src-tauri/tauri.macos-vcam.conf.json",
    "--target",
    "universal-apple-darwin",
  ]);
  // ほかの OS とアプリでは VCAM=1 は何も足さない
  const win = planBuild({ app: "studio", platform: "windows", host: "windows", vcam: true });
  assert.deepEqual(win.args.slice(4), []);
});

test("Windows の上の Windows 向けはネイティブのビルド", () => {
  const b = planBuild({ app: "viewer", platform: "windows", host: "windows" });
  assert.equal(b.cross, false);
  assert.deepEqual(b.args, ["--filter", "@ugoe/viewer", "tauri", "build"]);
});

test("reads options from environment variables when not given as arguments", () => {
  const r = parseArgs(["TARGET=editor"], { DRY_RUN: "1", PLATFORM: "linux", TARGET: "viewer" });
  assert.equal(r.dryRun, true);
  assert.deepEqual(r.platforms, ["linux"]);
  assert.deepEqual(r.apps, ["editor"]);
});
