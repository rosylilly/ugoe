// scripts/actionlint.mjs の単体テスト (`just script-check` が実行する)。道具は取得しない。
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  ACTIONLINT_ASSETS,
  ACTIONLINT_VERSION,
  actionlintArgs,
  actionlintUrl,
  SHELLCHECK_ASSETS,
  SHELLCHECK_VERSION,
  shellcheckUrl,
} from "./actionlint.mjs";
import { GITLEAKS_ASSETS } from "./secrets.mjs";

describe("ACTIONLINT_ASSETS / SHELLCHECK_ASSETS", () => {
  it("ファイルの名前が固定した版を含み、sha256 は 64 桁の 16 進", () => {
    for (const [assets, prefix] of [
      [ACTIONLINT_ASSETS, `actionlint_${ACTIONLINT_VERSION}_`],
      [SHELLCHECK_ASSETS, `shellcheck-v${SHELLCHECK_VERSION}.`],
    ]) {
      for (const [key, asset] of Object.entries(assets)) {
        assert.ok(asset.file.startsWith(prefix), `${key}: ${asset.file}`);
        assert.match(asset.sha256, /^[0-9a-f]{64}$/, key);
        assert.ok(asset.exe, key);
      }
    }
  });

  it("gitleaks と同じ機械に対応する", () => {
    const keys = Object.keys(GITLEAKS_ASSETS).sort();
    assert.deepEqual(Object.keys(ACTIONLINT_ASSETS).sort(), keys);
    assert.deepEqual(Object.keys(SHELLCHECK_ASSETS).sort(), keys);
  });

  it("展開した中の実行ファイルは Windows だけ .exe", () => {
    for (const assets of [ACTIONLINT_ASSETS, SHELLCHECK_ASSETS]) {
      for (const [key, asset] of Object.entries(assets)) {
        assert.equal(asset.exe.endsWith(".exe"), key.startsWith("win32-"), key);
      }
    }
  });
});

describe("URL と引数", () => {
  it("Release のファイルの URL", () => {
    assert.equal(
      actionlintUrl(ACTIONLINT_ASSETS["linux-x64"]),
      `https://github.com/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz`,
    );
    assert.equal(
      shellcheckUrl(SHELLCHECK_ASSETS["linux-x64"]),
      `https://github.com/koalaman/shellcheck/releases/download/v${SHELLCHECK_VERSION}/shellcheck-v${SHELLCHECK_VERSION}.linux.x86_64.tar.gz`,
    );
  });

  it("固定した shellcheck を渡し、pyflakes を外し、残りの引数をそのまま渡す", () => {
    assert.deepEqual(actionlintArgs("/x/shellcheck", ["a.yml"]), [
      "-shellcheck",
      "/x/shellcheck",
      "-pyflakes=",
      "a.yml",
    ]);
  });
});
