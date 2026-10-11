// scripts/app-build.mjs の引数と組み合わせの検査のテスト (`just app-build-check`)
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import {
  APPS,
  hostPlatform,
  parseArgs,
  planAll,
  planBuild,
  SIGNING_CONFIG,
  unsupportedReason,
} from "./app-build.mjs";
import {
  notarytoolAuthArgs,
  signingPlan,
  signingSummary,
  WINDOWS_SIGN_ENV,
  windowsSignArgv,
  withoutEmptySigningEnv,
} from "./lib/app-signing.mjs";

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

// --- 署名 (scripts/lib/app-signing.mjs、docs/release.md#デスクトップアプリの署名) ---

test("署名の環境変数が無ければ、どの OS も署名せずに作る (今までと同じ引数)", () => {
  for (const platform of ["windows", "macos", "linux"]) {
    const plan = signingPlan(platform, {});
    assert.equal(plan.sign, false);
    assert.equal(plan.notarize, false);
    assert.equal(plan.config, null);
    assert.ok(plan.reason);
  }
  const { builds } = planAll({
    apps: ["viewer"],
    platforms: ["windows"],
    host: "windows",
    vcam: false,
    extra: [],
    env: {},
  });
  assert.deepEqual(builds[0].args, ["--filter", "@ugoe/viewer", "tauri", "build"]);
});

test("Windows は署名のコマンドがあれば signCommand の設定を重ねる", () => {
  const env = { [WINDOWS_SIGN_ENV]: " trusted-signing-cli -e https://x -a A -c P -d ugoe %1 " };
  const plan = signingPlan("windows", env);
  assert.equal(plan.sign, true);
  assert.deepEqual(plan.config, {
    bundle: { windows: { signCommand: "trusted-signing-cli -e https://x -a A -c P -d ugoe %1" } },
  });
  const { builds } = planAll({
    apps: ["editor"],
    platforms: ["windows"],
    host: "windows",
    vcam: false,
    extra: [],
    env,
  });
  const args = builds[0].args;
  assert.equal(args.at(-2), "--config");
  assert.ok(args.at(-1).endsWith(SIGNING_CONFIG));
  // Linux 向けには重ねない
  const linux = planAll({ apps: ["editor"], platforms: ["linux"], host: "linux", extra: [], env });
  assert.equal(linux.builds[0].signing.sign, false);
  assert.ok(!linux.builds[0].args.some((a) => a.endsWith(SIGNING_CONFIG)));
});

test("Windows の署名のコマンドに %1 が無ければ、何も作らずに止まる", () => {
  const env = { [WINDOWS_SIGN_ENV]: "signtool sign /a" };
  assert.match(signingPlan("windows", env).error, /%1/);
  const plan = planAll({
    apps: ["editor"],
    platforms: ["windows"],
    host: "windows",
    extra: [],
    env,
  });
  assert.equal(plan.builds, undefined);
  assert.match(plan.errors[0], /^signing for windows/);
});

test("署名のコマンドは Tauri と同じく空白で分け、%1 をパスに替える", () => {
  assert.deepEqual(windowsSignArgv("tool  sign -f %1 --x=%1", "C:/a.dll"), [
    "tool",
    "sign",
    "-f",
    "C:/a.dll",
    "--x=C:/a.dll",
  ]);
});

test("macOS は署名の名義で署名し、公証の資格情報が 1 組揃っていれば公証する", () => {
  const identity = { APPLE_SIGNING_IDENTITY: "Developer ID Application: X (TEAM)" };
  const signOnly = signingPlan("macos", identity);
  assert.equal(signOnly.sign, true);
  assert.equal(signOnly.notarize, false);
  assert.match(signOnly.reason, /notarization/);

  const api = { APPLE_API_ISSUER: "i", APPLE_API_KEY: "k", APPLE_API_KEY_PATH: "/p.p8" };
  const withApi = signingPlan("macos", { ...identity, ...api });
  assert.equal(withApi.notarize, true);
  assert.equal(withApi.reason, null);
  assert.deepEqual(notarytoolAuthArgs(api), ["--issuer", "i", "--key-id", "k", "--key", "/p.p8"]);

  const appleId = { APPLE_ID: "a@b", APPLE_PASSWORD: "pw", APPLE_TEAM_ID: "T" };
  assert.equal(signingPlan("macos", { ...identity, ...appleId }).notarize, true);
  assert.deepEqual(notarytoolAuthArgs(appleId), [
    "--apple-id",
    "a@b",
    "--password",
    "pw",
    "--team-id",
    "T",
  ]);
});

test("macOS の半端な設定 (公証の組の一部・名義の無い資格情報・証明書だけ) は止まる", () => {
  const identity = { APPLE_SIGNING_IDENTITY: "Developer ID Application: X (TEAM)" };
  assert.match(
    signingPlan("macos", { ...identity, APPLE_API_ISSUER: "i", APPLE_API_KEY: "k" }).error,
    /missing APPLE_API_KEY_PATH/,
  );
  assert.match(signingPlan("macos", { APPLE_ID: "a@b" }).error, /APPLE_SIGNING_IDENTITY/);
  assert.match(signingPlan("macos", { APPLE_CERTIFICATE: "x" }).error, /APPLE_SIGNING_IDENTITY/);
  assert.match(
    signingPlan("macos", { ...identity, APPLE_CERTIFICATE: "x" }).error,
    /APPLE_CERTIFICATE_PASSWORD/,
  );
  // 空の値は無いのと同じ (Secrets が無いと GitHub Actions は空の文字列を渡す)
  assert.equal(signingPlan("macos", { APPLE_SIGNING_IDENTITY: "", APPLE_ID: "" }).sign, false);
});

test("ジョブの要約は、アプリと OS ごとに署名と公証の有無を表にする", () => {
  const summary = signingSummary([
    { app: "editor", platform: "macos", plan: { sign: true, notarize: true, reason: null } },
    { app: "editor", platform: "linux", plan: signingPlan("linux", {}) },
  ]);
  assert.match(summary, /\| editor \| macos \| yes \| yes \| {2}\|/);
  assert.match(summary, /\| editor \| linux \| no \| - \| Linux packages are not signed \|/);
});

test("ビルドに渡す前に、空の署名の環境変数を消す (GitHub Actions は無い Secrets を空で渡す)", () => {
  const env = withoutEmptySigningEnv({
    APPLE_SIGNING_IDENTITY: "",
    APPLE_CERTIFICATE: " ",
    AZURE_CLIENT_ID: "id",
    PATH: "",
  });
  assert.deepEqual(env, { AZURE_CLIENT_ID: "id", PATH: "" });
});

test("インストーラのフック (NSIS) の File のパスは `\\` で区切る (Windows の makensis は `/` で区切られたパスを探せない。T0088)", () => {
  const hooks = readFileSync(
    new URL("../apps/studio/src-tauri/windows/hooks.nsh", import.meta.url),
    "utf8",
  );
  const paths = hooks
    .split("\n")
    .map((line) => line.replace(/;.*$/, "").trim())
    .flatMap((line) => line.match(/^(?:!define\s+\S+|File)\s+"([^"]*)"/)?.slice(1) ?? []);
  assert.ok(paths.length > 0);
  for (const path of paths) assert.doesNotMatch(path, /\//, path);
});
