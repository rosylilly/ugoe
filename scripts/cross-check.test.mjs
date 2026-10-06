// scripts/cross-check.mjs のテスト (`just groups`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { OS_CRATES } from "./ci-groups.mjs";
import {
  CHECKS,
  clippyArgs,
  missingTargets,
  requiredTargets,
  requiredTools,
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

test("allFeatures は --all-features を渡し、無ければ渡さない", () => {
  assert.deepEqual(clippyArgs({ crates: ["x"], targets: ["t"], allFeatures: true }, "t"), [
    "clippy",
    "-p",
    "x",
    "--all-features",
    "--all-targets",
    "--target",
    "t",
    "--",
    "-D",
    "warnings",
  ]);
  assert.deepEqual(clippyArgs(checks.a, "t1").includes("--all-features"), false);
});

test("allFeatures と features を両方書いた定義は誤り", () => {
  const metadata = { packages: [{ name: "x" }] };
  const both = { crates: ["x"], targets: ["t"], allFeatures: true, features: ["x/f"] };
  assert.deepEqual(validate({ both }, metadata), [
    "both: allFeatures と features は両方を書けない",
  ]);
  assert.deepEqual(
    validate({ all: { crates: ["x"], targets: ["t"], allFeatures: true } }, metadata),
    [],
  );
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

test("ugoe-ai と OS_CRATES (CI の rust (windows) の clippy) は MinGW のある windows-gnu で、すべての feature で検査する", () => {
  assert.equal(CHECKS.tools.crates[0], "ugoe-ai");
  for (const name of OS_CRATES) assert.ok(CHECKS.tools.crates.includes(name), name);
  assert.equal(new Set(CHECKS.tools.crates).size, CHECKS.tools.crates.length);
  assert.deepEqual(CHECKS.tools.targets, ["x86_64-pc-windows-gnu"]);
  assert.equal(CHECKS.tools.allFeatures, true);
  assert.equal(CHECKS.tools.features, undefined);
  assert.ok(CHECKS.tools.commands?.includes("x86_64-w64-mingw32-gcc"));
});

test("ツールのコマンドと apt のパッケージは重なりを除いて集める", () => {
  const defs = {
    a: { crates: ["x"], targets: ["t"] },
    b: { crates: ["y"], targets: ["t"], commands: ["gcc", "windres"], apt: ["mingw"] },
    c: { crates: ["z"], targets: ["t"], commands: ["gcc"], apt: ["mingw", "other"] },
  };
  assert.deepEqual(requiredTools(select(defs, [])), {
    commands: ["gcc", "windres"],
    apt: ["mingw", "other"],
  });
  assert.deepEqual(requiredTools(select(defs, ["a"])), { commands: [], apt: [] });
});

test("アプリの src-tauri は MinGW の windows-gnu で、すべての feature で検査する", () => {
  assert.deepEqual(CHECKS.apps.targets, ["x86_64-pc-windows-gnu"]);
  assert.equal(CHECKS.apps.allFeatures, true);
  assert.deepEqual(CHECKS.apps.apt, ["gcc-mingw-w64-x86-64"]);
});
