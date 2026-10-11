// scripts/secrets.mjs の単体テスト (`just script-check` が実行する)。gitleaks そのものは取得しない。
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import {
  assetFor,
  assetUrl,
  CONFIG,
  configWithIgnored,
  GITLEAKS_ASSETS,
  GITLEAKS_VERSION,
  ignoredPathPatterns,
  isCommitId,
  isNullRev,
  parseArgs,
} from "./secrets.mjs";

describe("GITLEAKS_ASSETS", () => {
  it("どのファイルも固定した版のもので、sha256 は 64 桁の 16 進", () => {
    for (const [key, asset] of Object.entries(GITLEAKS_ASSETS)) {
      assert.ok(asset.file.startsWith(`gitleaks_${GITLEAKS_VERSION}_`), key);
      assert.match(asset.sha256, /^[0-9a-f]{64}$/, key);
    }
  });

  it("CI の Linux と、手元の主な機械に対応する", () => {
    assert.equal(assetFor("linux", "x64")?.file, `gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz`);
    assert.ok(assetFor("darwin", "arm64"));
    assert.equal(assetFor("win32", "x64")?.file, `gitleaks_${GITLEAKS_VERSION}_windows_x64.zip`);
    assert.equal(assetFor("freebsd", "x64"), null);
  });

  it("URL は版のタグの Release のファイル", () => {
    assert.equal(
      assetUrl(GITLEAKS_ASSETS["linux-x64"]),
      `https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}/gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz`,
    );
  });
});

describe("ignoredPathPatterns", () => {
  const matches = (entries, path) =>
    ignoredPathPatterns(entries).some((p) => new RegExp(p).test(path));

  it("ディレクトリはその下のファイルを外す (区切りは / と \\ のどちらも)", () => {
    assert.ok(matches(["node_modules/"], "node_modules/a/b.js"));
    assert.ok(matches(["apps/editor/dist/"], "apps/editor/dist/index.js"));
    assert.ok(matches(["apps/editor/dist/"], "apps\\editor\\dist\\index.js"));
    assert.ok(!matches(["apps/editor/dist/"], "apps/editor/distx/index.js"));
    assert.ok(!matches(["dist/"], "apps/editor/dist/index.js"));
  });

  it("ファイルはそれだけを外す", () => {
    assert.ok(matches([".mcp.json"], ".mcp.json"));
    assert.ok(!matches([".mcp.json"], ".mcp.json.bak"));
    // `.` は任意の 1 文字にしない
    assert.ok(!matches([".mcp.json"], "xmcp.json"));
  });

  it("空の行は飛ばす (git ls-files -z の最後)", () => {
    assert.deepEqual(ignoredPathPatterns(["", "a/", ""]), ["^a[\\\\/]"]);
  });
});

describe("configWithIgnored", () => {
  it("元の設定の後に、無視するパスと .git の allowlist を足す", () => {
    const base = readFileSync(join(import.meta.dirname, "..", CONFIG), "utf8");
    const text = configWithIgnored(base, ["target/", ""]);
    assert.ok(text.startsWith(base.trimEnd()));
    assert.match(
      text,
      /\[\[allowlists\]\]\ndescription = "ignored by git"\npaths = \[\n {2}'''\^target\[\\\\\/\]''',\n/,
    );
    assert.match(text, /'''\^\\\.git\(\[\\\\\/\]\|\$\)''',\n\]\n$/);
  });

  it("TOML のリテラル文字列に書けない名前は拒む", () => {
    assert.throws(() => configWithIgnored("", ["a'''b"]));
  });
});

describe("parseArgs", () => {
  it("既定は履歴のすべて", () => {
    assert.deepEqual(parseArgs([]), { since: null });
  });

  it("--since はその範囲だけ", () => {
    assert.deepEqual(parseArgs(["--since", "HEAD^1"]), { since: "HEAD^1" });
    assert.deepEqual(parseArgs(["--since=abc"]), { since: "abc" });
  });

  it("空・すべて 0 (新しいブランチの push) は履歴のすべて", () => {
    assert.deepEqual(parseArgs(["--since", ""]), { since: null });
    assert.deepEqual(parseArgs(["--since", "0".repeat(40)]), { since: null });
  });

  it("知らない引数と値の無い --since は拒む", () => {
    assert.throws(() => parseArgs(["--all"]));
    assert.throws(() => parseArgs(["--since"]));
  });
});

describe("isNullRev と isCommitId", () => {
  it("すべて 0 の ID", () => {
    assert.ok(isNullRev("0".repeat(40)));
    assert.ok(!isNullRev("HEAD"));
  });

  it("省略の無い ID だけをコミットの ID とみなす", () => {
    assert.ok(isCommitId("a".repeat(40)));
    assert.ok(isCommitId("0123456789abcdef".repeat(4)));
    assert.ok(!isCommitId("HEAD^1"));
    assert.ok(!isCommitId("abc1234"));
  });
});
