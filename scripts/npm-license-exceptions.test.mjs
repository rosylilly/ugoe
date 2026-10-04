// scripts/lib/npm-license-exceptions.mjs の一覧のテスト (`just script-check` が実行する)。
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { NPM_LICENSE_EXCEPTIONS } from "./lib/npm-license-exceptions.mjs";
import { licenseOf } from "./third-party-notices.mjs";

const entries = Object.entries(NPM_LICENSE_EXCEPTIONS);

describe("NPM_LICENSE_EXCEPTIONS", () => {
  it("どの例外も、確かめた版・SPDX の式のライセンス・確かめ方を持つ", () => {
    assert.ok(entries.length > 0);
    for (const [name, exception] of entries) {
      assert.deepEqual(Object.keys(exception).sort(), ["license", "reason", "version"], name);
      // 版は範囲 (`^1.0.0` など) ではなく、確かめた 1 つの版
      assert.match(exception.version, /^\d+\.\d+\.\d+(?:-[\w.]+)?$/, name);
      // SPDX の識別子を AND・OR・WITH と括弧でつないだ式
      assert.match(exception.license, /^[\w.+()-]+(?:\s+(?:AND|OR|WITH)\s+[\w.+()-]+)*$/, name);
      assert.ok(exception.reason.trim().length > 0, name);
    }
  });

  it("書き換えられない (2 つのスクリプトが同じものを読む)", () => {
    assert.ok(Object.isFrozen(NPM_LICENSE_EXCEPTIONS));
    for (const [, exception] of entries) assert.ok(Object.isFrozen(exception));
  });

  it("license の欄の無いパッケージの表記のライセンスに、一覧のライセンスを使う", () => {
    for (const [name, exception] of entries) {
      assert.equal(licenseOf({ name, version: exception.version }), exception.license);
    }
    assert.equal(licenseOf({ name: "not-in-the-list", version: "1.0.0" }), undefined);
    // package.json に license があれば、そちらを使う
    const [name] = entries[0];
    assert.equal(licenseOf({ name, license: "ISC" }), "ISC");
  });
});
