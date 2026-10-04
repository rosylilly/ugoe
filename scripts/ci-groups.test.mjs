// scripts/ci-groups.mjs の検査のテスト (`just groups`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { cargoArgs, OS_CRATES, osCargoArgs, validate, validateOs } from "./ci-groups.mjs";

const pkg = (name, { features = {}, targets = [] } = {}) => ({
  name,
  features,
  targets: [{ name, kind: ["lib"] }, ...targets],
});

test("すべてのクレートがちょうど 1 つのグループに入っていれば問題なし", () => {
  const metadata = { packages: [pkg("a"), pkg("b")] };
  assert.deepEqual(validate({ x: { crates: ["a"] }, y: { crates: ["b"] } }, metadata), []);
});

test("どのグループにも入っていないクレートと、2 つのグループに入っているクレートを見つける", () => {
  const metadata = { packages: [pkg("a"), pkg("b"), pkg("c")] };
  const errors = validate({ x: { crates: ["a", "b"] }, y: { crates: ["b"] } }, metadata);
  assert.equal(errors.length, 2);
  assert.match(errors[0], /^b: x と y/);
  assert.match(errors[1], /^c: どのグループにも入っていない/);
});

test("ワークスペースにないクレートと空のグループを見つける", () => {
  const metadata = { packages: [pkg("a")] };
  const errors = validate({ x: { crates: ["a", "gone"] }, y: { crates: [] } }, metadata);
  assert.equal(errors.length, 2);
  assert.match(errors[0], /gone はワークスペースのクレートではない/);
  assert.match(errors[1], /^y: クレートが 1 つもない/);
});

test("required-features はグループの features に書く (gpu-tests は除く)", () => {
  const targets = [
    { name: "spec", kind: ["test"], "required-features": ["schema"] },
    { name: "gpu", kind: ["test"], "required-features": ["gpu-tests"] },
  ];
  const metadata = {
    packages: [pkg("a", { features: { schema: [], "gpu-tests": [] }, targets })],
  };
  const missing = validate({ x: { crates: ["a"] } }, metadata);
  assert.equal(missing.length, 1);
  assert.match(missing[0], /test spec は feature schema が要る/);
  assert.deepEqual(validate({ x: { crates: ["a"], features: ["a/schema"] } }, metadata), []);
});

test("features はグループのクレートの、実在する feature に限る", () => {
  const metadata = { packages: [pkg("a", { features: { schema: [] } }), pkg("b")] };
  const errors = validate(
    { x: { crates: ["a"], features: ["b/schema", "a/nope", "schema"] }, y: { crates: ["b"] } },
    metadata,
  );
  assert.equal(errors.length, 3);
});

test("cargo の引数は -p の並びと --features", () => {
  assert.deepEqual(cargoArgs({ crates: ["a", "b"] }), ["-p", "a", "-p", "b"]);
  assert.deepEqual(cargoArgs({ crates: ["a"], features: ["a/x", "a/y"] }), [
    "-p",
    "a",
    "--features",
    "a/x,a/y",
  ]);
});

test("--all-features はグループの features の代わりに付ける", () => {
  assert.deepEqual(cargoArgs({ crates: ["a"], features: ["a/x"] }, { allFeatures: true }), [
    "-p",
    "a",
    "--all-features",
  ]);
});

test("OS_CRATES はワークスペースのクレートに限る", () => {
  const metadata = { packages: [pkg("a")] };
  assert.deepEqual(validateOs(["a"], metadata), []);
  assert.deepEqual(validateOs(["a", "gone"], metadata), [
    "OS_CRATES: gone はワークスペースのクレートではない",
  ]);
  assert.deepEqual(validateOs([], metadata), ["OS_CRATES: クレートが 1 つもない"]);
  assert.ok(OS_CRATES.length > 0);
});

test("OS のジョブの clippy はすべての feature で警告をエラーにし、test は引数をそのまま渡す", () => {
  assert.deepEqual(osCargoArgs("clippy", ["a", "b"]), [
    "clippy",
    "-p",
    "a",
    "-p",
    "b",
    "--all-features",
    "--all-targets",
    "--",
    "-D",
    "warnings",
  ]);
  assert.deepEqual(osCargoArgs("test", ["a"], ["--", "--nocapture"]), [
    "test",
    "-p",
    "a",
    "--",
    "--nocapture",
  ]);
  assert.throws(() => osCargoArgs("build", ["a"]), /unknown command: build/);
});
