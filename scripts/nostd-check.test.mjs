// scripts/nostd-check.mjs のテスト (`just groups`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { clippyArgs, NOSTD_TARGET, select, validate } from "./nostd-check.mjs";

test("クレートを省けばすべてを選び、知らないクレートは失敗する", () => {
  assert.deepEqual(select(["a", "b"], []), ["a", "b"]);
  assert.deepEqual(select(["a", "b"], ["b"]), ["b"]);
  assert.throws(() => select(["a", "b"], ["c"]), /unknown crate: c \(a, b\)/);
});

test("clippy の引数は 1 つのクレートのライブラリを std の無いターゲットで、既定の feature を外して検査する", () => {
  assert.deepEqual(clippyArgs("x"), [
    "clippy",
    "-p",
    "x",
    "--lib",
    "--no-default-features",
    "--target",
    NOSTD_TARGET,
    "--",
    "-D",
    "warnings",
  ]);
});

test("一覧のクレートはワークスペースにあり、feature std を既定で持つ", () => {
  const metadata = {
    packages: [
      { name: "ok", features: { default: ["std"], std: [] } },
      { name: "no-std-feature", features: {} },
      { name: "not-default", features: { default: [], std: [] } },
    ],
  };
  assert.deepEqual(validate(["ok"], metadata), []);
  assert.deepEqual(validate(["ok", "ok"], metadata), ["同じクレートが 2 回ある"]);
  assert.deepEqual(validate(["missing"], metadata), ["missing はワークスペースのクレートではない"]);
  assert.deepEqual(validate(["no-std-feature"], metadata), [
    "no-std-feature: feature std が無い",
    "no-std-feature: feature std が既定で有効になっていない",
  ]);
  assert.deepEqual(validate(["not-default"], metadata), [
    "not-default: feature std が既定で有効になっていない",
  ]);
});
