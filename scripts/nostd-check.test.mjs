// scripts/nostd-check.mjs のテスト (`just groups`)
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  ATOMIC32_TARGET,
  clippyArgs,
  FFI_CRATE,
  HOST_SYMBOLS,
  HOST_TEST_CRATES,
  hostClippyArgs,
  hostStubSource,
  hostTestArgs,
  MATH_CRATES,
  missingTargets,
  NOSTD_CRATES,
  NOSTD_TARGET,
  NOSTD_TARGETS,
  select,
  staticlibArgs,
  undefinedSymbols,
  unexpectedSymbols,
  validate,
  validateClippyConfigs,
  validateHostTests,
} from "./nostd-check.mjs";

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

test("64 ビットのアトミック操作の無いターゲットでも検査し、入っていないターゲットを見つける", () => {
  assert.deepEqual(NOSTD_TARGETS, ["aarch64-unknown-none", "thumbv7em-none-eabihf"]);
  assert.equal(NOSTD_TARGETS[0], NOSTD_TARGET);
  assert.ok(NOSTD_TARGETS.includes(ATOMIC32_TARGET));
  assert.deepEqual(clippyArgs("x", ATOMIC32_TARGET).slice(5, 7), ["--target", ATOMIC32_TARGET]);
  assert.deepEqual(staticlibArgs(ATOMIC32_TARGET).slice(-2), ["--target", ATOMIC32_TARGET]);
  assert.deepEqual(missingTargets(["x86_64-unknown-linux-gnu", ...NOSTD_TARGETS]), []);
  assert.deepEqual(missingTargets(["aarch64-unknown-none"]), [ATOMIC32_TARGET]);
  assert.deepEqual(missingTargets([]), NOSTD_TARGETS);
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

test("数学関数を禁じるクレートの clippy.toml は揃っていて、ルートの設定を含む", () => {
  const root = "# コメント\nallow-unwrap-in-tests = true\n";
  const good = "# 説明\nallow-unwrap-in-tests = true\ndisallowed-methods = []\n";
  const files = { a: good, b: good };
  const read = (name) => files[name];
  assert.deepEqual(validateClippyConfigs(["a", "b"], root, read), []);
  assert.deepEqual(validateClippyConfigs(["a", "c"], root, read), [
    "c: clippy.toml が無い (数学関数の禁止)",
  ]);
  files.b = "allow-unwrap-in-tests = true\n";
  assert.deepEqual(validateClippyConfigs(["a", "b"], root, read), [
    "b: clippy.toml に disallowed-methods が無い",
    "b: clippy.toml が a のものと違う",
  ]);
  files.b = "disallowed-methods = []\n";
  assert.deepEqual(validateClippyConfigs(["b"], root, read), [
    "b: clippy.toml にルートの設定 `allow-unwrap-in-tests = true` が無い",
  ]);
});

test("C ABI のクレートは std 無しの一覧と数学関数を禁じる一覧にある", () => {
  assert.ok(NOSTD_CRATES.includes(FFI_CRATE));
  assert.ok(MATH_CRATES.includes(FFI_CRATE));
});

test("静的ライブラリは巻き戻さないプロファイルで、staticlib だけを std の無いターゲットに作る", () => {
  assert.deepEqual(staticlibArgs(), [
    "rustc",
    "-p",
    FFI_CRATE,
    "--lib",
    "--crate-type",
    "staticlib",
    "--no-default-features",
    "--profile",
    "nostd",
    "--target",
    NOSTD_TARGET,
  ]);
});

test("リンカの出力から未解決のシンボルを集め、ホストの C ライブラリのもの以外を見つける", () => {
  const output = [
    "rust-lld: error: undefined symbol: malloc",
    ">>> referenced by alloc.rs:101",
    "rust-lld: error: undefined symbol: free",
    "rust-lld: error: undefined symbol: malloc",
    "rust-lld: error: undefined symbol: pthread_key_create",
  ].join("\n");
  assert.deepEqual(undefinedSymbols(output), ["free", "malloc", "pthread_key_create"]);
  assert.deepEqual(unexpectedSymbols(undefinedSymbols(output)), ["pthread_key_create"]);
  assert.deepEqual(undefinedSymbols(""), []);
  assert.deepEqual(unexpectedSymbols(HOST_SYMBOLS), []);
});

test("ホストの C ライブラリの代わりは、HOST_SYMBOLS をすべて定義する", () => {
  const source = hostStubSource();
  for (const name of HOST_SYMBOLS) assert.match(source, new RegExp(`\\b${name}\\(`));
  assert.ok(!source.includes("undefined"));
});

test("std の無い形の単体テストは、既定の feature を外してホストで clippy とテストを流す", () => {
  assert.deepEqual(hostClippyArgs("x"), [
    "clippy",
    "-p",
    "x",
    "--lib",
    "--profile",
    "test",
    "--no-default-features",
    "--",
    "-D",
    "warnings",
  ]);
  assert.deepEqual(hostTestArgs("x"), ["test", "-p", "x", "--lib", "--no-default-features"]);
  assert.ok(HOST_TEST_CRATES.includes("ugoe-format"));
  assert.ok(HOST_TEST_CRATES.includes(FFI_CRATE));
  assert.deepEqual(validateHostTests(HOST_TEST_CRATES, NOSTD_CRATES, { packages: [] }), []);
});

test("std の無い形の単体テストのクレートは、std 無しの一覧にあり、開発時の依存に自分を置かない", () => {
  const metadata = {
    packages: [
      { name: "ok", dependencies: [{ name: "other", kind: "dev" }] },
      { name: "normal-self", dependencies: [{ name: "normal-self", kind: null }] },
      { name: "dev-self", dependencies: [{ name: "dev-self", kind: "dev" }] },
    ],
  };
  const crates = ["ok", "normal-self", "dev-self"];
  assert.deepEqual(validateHostTests(["ok", "normal-self"], crates, metadata), []);
  assert.deepEqual(validateHostTests(["dev-self"], crates, metadata), [
    "dev-self: 開発時の依存に自分があり、std の無い形の単体テストにならない",
  ]);
  assert.deepEqual(validateHostTests(["missing"], crates, metadata), [
    "HOST_TEST_CRATES: missing は NOSTD_CRATES に無い",
  ]);
});
