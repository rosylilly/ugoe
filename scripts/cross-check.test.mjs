// scripts/cross-check.mjs のテスト (`just groups`)
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  CHECKS,
  clippyArgs,
  missingTargets,
  requiredTargets,
  select,
  validate,
} from "./cross-check.mjs";

const checks = {
  a: { crates: ["x"], targets: ["t1", "t2"] },
  b: { crates: ["y", "z"], targets: ["t2", "t3"], features: ["y/f"] },
};

test("名前を省けばすべての検査を選び、知らない名前は失敗する", () => {
  assert.deepEqual(
    select(checks, []).map(([name]) => name),
    ["a", "b"],
  );
  assert.deepEqual(
    select(checks, ["b"]).map(([name]) => name),
    ["b"],
  );
  assert.throws(() => select(checks, ["c"]), /unknown check: c \(a, b\)/);
});

test("要るターゲットは重なりを除き、入っていないものだけを出す", () => {
  const required = requiredTargets(select(checks, []));
  assert.deepEqual(required, ["t1", "t2", "t3"]);
  assert.deepEqual(missingTargets(required, ["t2", "other"]), ["t1", "t3"]);
  assert.deepEqual(missingTargets(required, ["t1", "t2", "t3"]), []);
});

test("clippy の引数はクレート・feature・ターゲットを並べ、警告をエラーにする", () => {
  assert.deepEqual(clippyArgs(checks.b, "t3"), [
    "clippy",
    "-p",
    "y",
    "-p",
    "z",
    "--features",
    "y/f",
    "--all-targets",
    "--target",
    "t3",
    "--",
    "-D",
    "warnings",
  ]);
});

test("ワークスペースにないクレートと、空のクレート・ターゲットを見つける", () => {
  const metadata = { packages: [{ name: "x" }] };
  assert.deepEqual(validate({ a: checks.a }, metadata), []);
  const errors = validate({ b: checks.b, c: { crates: [], targets: [] } }, metadata);
  assert.deepEqual(errors, [
    "b: y はワークスペースのクレートではない",
    "b: z はワークスペースのクレートではない",
    "c: クレートが 1 つもない",
    "c: ターゲットが 1 つもない",
  ]);
});

test("仮想カメラは Windows と macOS のターゲットで検査する", () => {
  assert.deepEqual(CHECKS.vcam.crates, ["ugoe-studio-vcam"]);
  assert.deepEqual(CHECKS.vcam.targets, ["x86_64-pc-windows-msvc", "aarch64-apple-darwin"]);
});
