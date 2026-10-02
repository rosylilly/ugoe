#!/usr/bin/env node
// macOS の仮想カメラの Camera Extension (app.ugoe.studio.camera.systemextension) を作って署名する (`pnpm vcam:macos`)。
// 仮想カメラを入れたスタジオを作るときに、tauri.macos-vcam.conf.json の beforeBuildCommand が呼ぶ
// (docs/studio.md#macos-camera-extension、docs/release.md#macos-の仮想カメラ)。
//
// - ソースは apps/studio/vcam/macos/ (Swift)。Xcode のプロジェクトは使わず、swiftc で arm64 と x86_64 を作って lipo で束ねる。
// - 出力は <リポジトリ>/target/vcam/macos/ の app.ugoe.studio.camera.systemextension と、スタジオに入れる
//   プロビジョニングプロファイル (studio.provisionprofile)。tauri.macos-vcam.conf.json の bundle.macOS.files がアプリの束に入れる。
// - macOS でだけ動く (Xcode かコマンドラインツールが要る)。
//
// 環境変数:
//   UGOE_APPLE_TEAM_ID              Apple Developer のチーム ID (10 文字)。アプリグループと Mach サービスの名前に使う。
//   APPLE_SIGNING_IDENTITY          署名の証明書の名前 (tauri build と同じ変数。配布は "Developer ID Application: ...")。
//   UGOE_VCAM_PROVISIONING_PROFILE  拡張 (app.ugoe.studio.camera) のプロビジョニングプロファイルのパス。
//   UGOE_STUDIO_PROVISIONING_PROFILE スタジオ (app.ugoe.studio) のプロビジョニングプロファイルのパス。

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dirname, "..", "..", "..");
const sources = join(root, "apps", "studio", "vcam", "macos");
const out = join(root, "target", "vcam", "macos");
const id = "app.ugoe.studio.camera";
const bundle = join(out, `${id}.systemextension`);
const minimumSystem = "12.3";

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

function run(command, args) {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.error) fail(`${command}: ${result.error.message}`);
  if (result.status !== 0) fail(`${command} exited with ${result.status}`);
}

function env(name) {
  const value = process.env[name];
  if (value === undefined || value === "") fail(`set ${name} (see docs/release.md)`);
  return value;
}

if (process.platform !== "darwin") fail("the camera extension can only be built on macOS");
const teamId = env("UGOE_APPLE_TEAM_ID");
if (!/^[A-Z0-9]{10}$/.test(teamId)) fail("UGOE_APPLE_TEAM_ID must be the 10-character team ID");
const identity = env("APPLE_SIGNING_IDENTITY");
const extensionProfile = env("UGOE_VCAM_PROVISIONING_PROFILE");
const studioProfile = env("UGOE_STUDIO_PROVISIONING_PROFILE");

const tauriConfig = JSON.parse(
  readFileSync(join(root, "apps", "studio", "src-tauri", "tauri.conf.json"), "utf8"),
);
// macOS は CFBundleVersion が変わったときだけ拡張を入れ替えるので、ビルドの時刻を使う (YYYYMMDDHHMMSS)。
const build = new Date().toISOString().replace(/[-:T]/g, "").slice(0, 14);
// Info.plist と entitlement の ${TEAM_ID}・${VERSION}・${BUILD} を埋める。
const values = { TEAM_ID: teamId, VERSION: tauriConfig.version, BUILD: build };
const fill = (text) => text.replace(/\$\{(TEAM_ID|VERSION|BUILD)\}/g, (_, key) => values[key]);

rmSync(bundle, { recursive: true, force: true });
mkdirSync(join(bundle, "Contents", "MacOS"), { recursive: true });

// 両方の CPU 向けに作って束ねる。
const swiftFiles = ["Camera.swift", "main.swift"].map((f) => join(sources, "Extension", f));
const slices = ["arm64", "x86_64"].map((arch) => {
  const output = join(out, `${id}-${arch}`);
  run("xcrun", [
    "--sdk",
    "macosx",
    "swiftc",
    "-O",
    "-swift-version",
    "5",
    "-target",
    `${arch}-apple-macos${minimumSystem}`,
    "-module-name",
    "UgoeStudioCamera",
    "-o",
    output,
    ...swiftFiles,
  ]);
  return output;
});
run("xcrun", ["lipo", "-create", "-output", join(bundle, "Contents", "MacOS", id), ...slices]);
for (const slice of slices) rmSync(slice);

writeFileSync(
  join(bundle, "Contents", "Info.plist"),
  fill(readFileSync(join(sources, "Info.plist"), "utf8")),
);
copyFileSync(extensionProfile, join(bundle, "Contents", "embedded.provisionprofile"));
const entitlements = join(out, "extension.entitlements");
writeFileSync(entitlements, fill(readFileSync(join(sources, "Extension.entitlements"), "utf8")));

// 拡張は tauri がアプリを署名する前に、自分の entitlement で署名しておく (アプリの署名は中の署名を変えない)。
run("codesign", [
  "--force",
  "--options",
  "runtime",
  "--timestamp",
  "--entitlements",
  entitlements,
  "--sign",
  identity,
  bundle,
]);
copyFileSync(studioProfile, join(out, "studio.provisionprofile"));
console.log(`built ${bundle}`);
