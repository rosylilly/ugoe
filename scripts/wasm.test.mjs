// scripts/wasm.mjs の生成物の古さの判定の単体テスト (`just script-check` が実行する)。
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after, beforeEach, describe, it } from "node:test";
import {
  APP_OUTS,
  FINGERPRINT_FILE,
  fingerprint,
  inputFiles,
  nonDevDependencies,
  OUTS,
  staleOutputs,
  WASM_FILE,
  WEB_RUNTIME_OUT,
  wasmCrateDirs,
} from "./wasm.mjs";

const tmp = mkdtempSync(join(tmpdir(), "ugoe-wasm-test-"));
after(() => rmSync(tmp, { recursive: true, force: true }));

let root;
let n = 0;

function write(path, text) {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), text);
}

// ugoe-wasm -> ugoe-a -> ugoe-core。ugoe-b は ugoe-a の dev の依存だけ、ugoe-c はどこからも使わない
beforeEach(() => {
  root = join(tmp, String(n++));
  write(
    "Cargo.toml",
    `[workspace]
members = ["crates/*"]

[workspace.dependencies]
serde = "1"
ugoe-a = { path = "crates/ugoe-a" }
ugoe-b = { path = "crates/ugoe-b" }
ugoe-c = { path = "crates/ugoe-c" }
ugoe-core = { path = "crates/ugoe-core", version = "0.0.0" }
ugoe-wasm = { path = "crates/ugoe-wasm" }
`,
  );
  write("Cargo.lock", '[[package]]\nname = "wasm-bindgen"\nversion = "0.2.1"\n');
  write("rust-toolchain.toml", '[toolchain]\nchannel = "1.0.0"\n');
  write(
    "crates/ugoe-wasm/Cargo.toml",
    "[dependencies]\nserde.workspace = true\n\n[target.'cfg(target_arch = \"wasm32\")'.dependencies]\nugoe-a = { workspace = true }\n",
  );
  write("crates/ugoe-wasm/src/lib.rs", "// wasm\n");
  write("crates/ugoe-wasm/README.md", "# wasm\n");
  write(
    "crates/ugoe-a/Cargo.toml",
    "[dependencies]\nugoe-core.workspace = true\n\n[dev-dependencies]\nugoe-b.workspace = true\n",
  );
  write("crates/ugoe-a/src/lib.rs", "// a\n");
  write("crates/ugoe-a/src/shader/a.wgsl", "// shader\n");
  write("crates/ugoe-a/tests/a.rs", "// test\n");
  write("crates/ugoe-core/Cargo.toml", "[dependencies]\n");
  write("crates/ugoe-core/build.rs", "fn main() {}\n");
  write("crates/ugoe-core/src/lib.rs", "// core\n");
  write("crates/ugoe-b/Cargo.toml", "[dependencies]\n");
  write("crates/ugoe-b/src/lib.rs", "// b\n");
  write("crates/ugoe-c/Cargo.toml", "[dependencies]\n");
  write("crates/ugoe-c/src/lib.rs", "// c\n");
});

/** `build` と同じく、生成物と今の指紋を `out` に置く。 */
function generate(out, print = fingerprint(root)) {
  write(`${out}/ugoe_wasm.d.ts`, "export {};\n");
  write(`${out}/${WASM_FILE}`, "");
  write(`${out}/${FINGERPRINT_FILE}`, `${print}\n`);
}

describe("nonDevDependencies", () => {
  it("dependencies・build-dependencies・target の dependencies を読み、dev-dependencies を飛ばす", () => {
    const toml = `[package]
name = "x"

[dependencies]
a.workspace = true
b = { workspace = true }

[dev-dependencies]
c.workspace = true

[build-dependencies]
d = "1"

[target.'cfg(unix)'.dependencies]
e.workspace = true

[target.'cfg(unix)'.dev-dependencies]
f.workspace = true
`;
    assert.deepEqual([...nonDevDependencies(toml)].sort(), ["a", "b", "d", "e"]);
  });
});

describe("wasmCrateDirs", () => {
  it("ugoe-wasm から dev 以外の依存をたどる", () => {
    assert.deepEqual(wasmCrateDirs(root), [
      "crates/ugoe-a",
      "crates/ugoe-core",
      "crates/ugoe-wasm",
    ]);
  });
});

describe("inputFiles", () => {
  it("ルートのファイルと、クレートの Cargo.toml・build.rs・src/ だけを入れる", () => {
    assert.deepEqual(inputFiles(root), [
      "Cargo.lock",
      "Cargo.toml",
      "crates/ugoe-a/Cargo.toml",
      "crates/ugoe-a/src/lib.rs",
      "crates/ugoe-a/src/shader/a.wgsl",
      "crates/ugoe-core/Cargo.toml",
      "crates/ugoe-core/build.rs",
      "crates/ugoe-core/src/lib.rs",
      "crates/ugoe-wasm/Cargo.toml",
      "crates/ugoe-wasm/src/lib.rs",
      "rust-toolchain.toml",
    ]);
  });
});

describe("staleOutputs", () => {
  it("生成物が無ければ (作ったことが無ければ) 飛ばす", () => {
    mkdirSync(join(root, APP_OUTS[1]), { recursive: true }); // 空のディレクトリも無いのと同じ
    assert.deepEqual(staleOutputs(root), []);
  });

  it("今の指紋と同じなら古くない", () => {
    for (const out of OUTS) generate(out);
    assert.deepEqual(staleOutputs(root), []);
  });

  it("指紋が違えば (書き換えれば) 古い", () => {
    for (const out of APP_OUTS) generate(out);
    write(`${APP_OUTS[2]}/${FINGERPRINT_FILE}`, "0".repeat(64));
    assert.deepEqual(
      staleOutputs(root).map((s) => s.out),
      [APP_OUTS[2]],
    );
  });

  it("型だけで wasm 本体の無いもの (bindings/web/scripts/check.mjs が置くスタブの写しなど) は飛ばす", () => {
    write(`${APP_OUTS[0]}/ugoe_wasm.d.ts`, "export {};\n");
    write(`${WEB_RUNTIME_OUT}/ugoe_wasm.d.ts`, "export {};\n");
    assert.deepEqual(staleOutputs(root), []);
  });

  it("指紋の無い生成物は古い", () => {
    write(`${APP_OUTS[0]}/ugoe_wasm.d.ts`, "export {};\n");
    write(`${APP_OUTS[0]}/${WASM_FILE}`, "");
    const stale = staleOutputs(root);
    assert.deepEqual(
      stale.map((s) => s.out),
      [APP_OUTS[0]],
    );
    assert.match(stale[0].reason, /no ugoe_wasm\.fingerprint/);
  });

  for (const [what, path] of [
    ["ugoe-wasm のソース", "crates/ugoe-wasm/src/lib.rs"],
    ["依存するクレートのソース", "crates/ugoe-core/src/lib.rs"],
    ["依存するクレートの src/ の下のシェーダ", "crates/ugoe-a/src/shader/a.wgsl"],
    ["依存するクレートの Cargo.toml", "crates/ugoe-a/Cargo.toml"],
    ["Cargo.lock (wasm-bindgen の版)", "Cargo.lock"],
    ["rust-toolchain.toml", "rust-toolchain.toml"],
  ]) {
    it(`${what}が変われば古い`, () => {
      generate(APP_OUTS[0]);
      write(path, "// changed\n");
      assert.deepEqual(
        staleOutputs(root).map((s) => s.out),
        [APP_OUTS[0]],
      );
    });
  }

  it("ソースのファイルが増えれば古い", () => {
    generate(APP_OUTS[0]);
    write("crates/ugoe-a/src/new.rs", "");
    assert.equal(staleOutputs(root).length, 1);
  });

  for (const [what, path] of [
    ["テスト", "crates/ugoe-a/tests/a.rs"],
    ["README", "crates/ugoe-wasm/README.md"],
    ["dev の依存だけのクレート", "crates/ugoe-b/src/lib.rs"],
    ["使わないクレート", "crates/ugoe-c/src/lib.rs"],
  ]) {
    it(`${what}が変わっても古くない`, () => {
      generate(APP_OUTS[0]);
      write(path, "// changed\n");
      assert.deepEqual(staleOutputs(root), []);
    });
  }

  it("@ugoe/runtime (bindings/web) の生成物も確かめる", () => {
    assert.ok(OUTS.includes(WEB_RUNTIME_OUT));
    generate(WEB_RUNTIME_OUT);
    assert.deepEqual(staleOutputs(root), []);
    write("crates/ugoe-wasm/src/lib.rs", "// changed\n");
    assert.deepEqual(
      staleOutputs(root).map((s) => s.out),
      [WEB_RUNTIME_OUT],
    );
  });

  it("指紋の無い @ugoe/runtime の生成物 (指紋を書く前のビルドの残り) は古い", () => {
    write(`${WEB_RUNTIME_OUT}/ugoe_wasm.d.ts`, "export {};\n");
    write(`${WEB_RUNTIME_OUT}/${WASM_FILE}`, "");
    assert.deepEqual(
      staleOutputs(root).map((s) => s.out),
      [WEB_RUNTIME_OUT],
    );
  });
});
