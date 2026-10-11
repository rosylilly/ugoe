#!/usr/bin/env node
// GitHub Actions のワークフロー (.github/workflows/) と composite action を actionlint で検査する (T1008。`just actionlint`。
// docs/testing.md#ci-のジョブ)。`run:` のシェルは actionlint から shellcheck を呼んで検査する。
//
// actionlint と shellcheck は版と sha256 を固定し (下の *_VERSION と *_ASSETS)、GitHub の Release から
// `<メインの作業ツリー>/target/tools/<名前>-<版>/` に取得する (scripts/lib/release-tool.mjs。gitleaks と同じ置き場)。
// 手元に入っている actionlint・shellcheck は使わない (版によって指摘が変わり、古い shellcheck の出力は actionlint が読めない)。
// pyflakes (`run:` の Python の検査) は使わない (固定した版を用意していないため)。
//
// 引数はそのまま actionlint に渡す (例: `just actionlint .github/workflows/ci.yml`)。

import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { ensureReleaseTool, platformKey } from "./lib/release-tool.mjs";

const root = join(import.meta.dirname, "..");

/** 固定した actionlint の版。上げるときは ACTIONLINT_ASSETS の sha256 も Release の checksums.txt から写す。 */
export const ACTIONLINT_VERSION = "1.7.12";

/**
 * 実行する機械 (`${process.platform}-${process.arch}`) ごとの actionlint の Release のファイルと sha256
 * (https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_checksums.txt)。
 */
export const ACTIONLINT_ASSETS = {
  "linux-x64": {
    file: "actionlint_1.7.12_linux_amd64.tar.gz",
    sha256: "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8",
    exe: "actionlint",
  },
  "linux-arm64": {
    file: "actionlint_1.7.12_linux_arm64.tar.gz",
    sha256: "325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6",
    exe: "actionlint",
  },
  "darwin-x64": {
    file: "actionlint_1.7.12_darwin_amd64.tar.gz",
    sha256: "5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644",
    exe: "actionlint",
  },
  "darwin-arm64": {
    file: "actionlint_1.7.12_darwin_arm64.tar.gz",
    sha256: "aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f",
    exe: "actionlint",
  },
  "win32-x64": {
    file: "actionlint_1.7.12_windows_amd64.zip",
    sha256: "6e7241b51e6817ea6a047693d8e6fed13b31819c9a0dd6c5a726e1592d22f6e9",
    exe: "actionlint.exe",
  },
};

/**
 * 固定した shellcheck の版。Release に checksums のファイルが無いので、上げるときは各ファイルの sha256 を
 * GitHub の Release の API の `digest` (`gh api repos/koalaman/shellcheck/releases/tags/v<版>`) から写す。
 */
export const SHELLCHECK_VERSION = "0.11.0";

/** 実行する機械ごとの shellcheck の Release のファイルと sha256 (GitHub の Release の API の `digest`)。 */
export const SHELLCHECK_ASSETS = {
  "linux-x64": {
    file: "shellcheck-v0.11.0.linux.x86_64.tar.gz",
    sha256: "b7af85e41cc99489dcc21d66c6d5f3685138f06d34651e6d34b42ec6d54fe6f6",
    exe: "shellcheck-v0.11.0/shellcheck",
  },
  "linux-arm64": {
    file: "shellcheck-v0.11.0.linux.aarch64.tar.gz",
    sha256: "68a8133197a50beb8803f8d42f9908d1af1c5540d4bb05fdfca8c1fa47decefc",
    exe: "shellcheck-v0.11.0/shellcheck",
  },
  "darwin-x64": {
    file: "shellcheck-v0.11.0.darwin.x86_64.tar.gz",
    sha256: "c2c15e08df0e8fbc374c335b230a7ee958c313fa5714817a59aa59f1aa594f51",
    exe: "shellcheck-v0.11.0/shellcheck",
  },
  "darwin-arm64": {
    file: "shellcheck-v0.11.0.darwin.aarch64.tar.gz",
    sha256: "339b930feb1ea764467013cc1f72d09cd6b869ebf1013296ba9055ab2ffbd26f",
    exe: "shellcheck-v0.11.0/shellcheck",
  },
  "win32-x64": {
    file: "shellcheck-v0.11.0.zip",
    sha256: "8a4e35ab0b331c85d73567b12f2a444df187f483e5079ceffa6bda1faa2e740e",
    exe: "shellcheck.exe",
  },
};

/** actionlint の Release のファイルの URL。 */
export function actionlintUrl(asset, version = ACTIONLINT_VERSION) {
  return `https://github.com/rhysd/actionlint/releases/download/v${version}/${asset.file}`;
}

/** shellcheck の Release のファイルの URL。 */
export function shellcheckUrl(asset, version = SHELLCHECK_VERSION) {
  return `https://github.com/koalaman/shellcheck/releases/download/v${version}/${asset.file}`;
}

/** actionlint に渡す引数 (固定した shellcheck を使い、pyflakes は使わない)。 */
export function actionlintArgs(shellcheck, extra = []) {
  return ["-shellcheck", shellcheck, "-pyflakes=", ...extra];
}

/** 表から実行する機械のファイルを引く。無ければどこを直すかを書いたエラー。 */
function assetOrThrow(name, version, assets, key) {
  const asset = assets[key];
  if (!asset) {
    throw new Error(
      `${name} ${version} is not set up for ${key} (scripts/actionlint.mjs の ${name.toUpperCase()}_ASSETS)`,
    );
  }
  return asset;
}

async function main(argv) {
  const key = platformKey();
  const actionlintAsset = assetOrThrow("actionlint", ACTIONLINT_VERSION, ACTIONLINT_ASSETS, key);
  const shellcheckAsset = assetOrThrow("shellcheck", SHELLCHECK_VERSION, SHELLCHECK_ASSETS, key);
  const actionlint = await ensureReleaseTool({
    root,
    name: "actionlint",
    version: ACTIONLINT_VERSION,
    asset: actionlintAsset,
    url: actionlintUrl(actionlintAsset),
  });
  const shellcheck = await ensureReleaseTool({
    root,
    name: "shellcheck",
    version: SHELLCHECK_VERSION,
    asset: shellcheckAsset,
    url: shellcheckUrl(shellcheckAsset),
  });
  const r = spawnSync(actionlint, actionlintArgs(shellcheck, argv), {
    cwd: root,
    stdio: "inherit",
  });
  if (r.error) throw r.error;
  process.exit(r.status ?? 1);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main(process.argv.slice(2)).catch((err) => {
    console.error(`error: ${err.message}`);
    process.exit(1);
  });
}
