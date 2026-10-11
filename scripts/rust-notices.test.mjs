// scripts/lib/rust-notices.mjs の部品のテスト (`just licenses`)
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import {
  ABOUT_CONFIG,
  aboutArgs,
  aboutList,
  CARGO_ABOUT_VERSION,
  compileOnlyCrates,
  crateAuthors,
  crateSource,
  excludedSummary,
  isLicenseFile,
  metadataArgs,
  parseCargoAboutVersion,
  renderRustNotices,
  rustNotices,
  stripPlaceholders,
} from "./lib/rust-notices.mjs";

const REGISTRY = "registry+https://github.com/rust-lang/crates.io-index";

/** cargo-about の JSON の crates の 1 つ。 */
function pkg(name, version, license, extra = {}) {
  return { name, version, license, source: REGISTRY, repository: null, authors: [], ...extra };
}

/** cargo-about の JSON の licenses の 1 つ。 */
function lic(id, text, sourcePath, ...crates) {
  return {
    name: `${id} License`,
    id,
    text,
    source_path: sourcePath,
    used_by: crates.map((crate) => ({ crate, path: null })),
  };
}

test("cargo about --version の版を読む", () => {
  assert.equal(parseCargoAboutVersion("cargo-about 0.9.2\n"), "0.9.2");
  assert.equal(parseCargoAboutVersion("error: no such command: `about`"), undefined);
  assert.equal(parseCargoAboutVersion(undefined), undefined);
});

test("CI と手順書が決めた版の cargo-about を入れる", () => {
  const root = join(import.meta.dirname, "..");
  const justfile = readFileSync(join(root, "justfile"), "utf8");
  assert.match(justfile, /\ncargo-about:\n/);
  const ci = readFileSync(join(root, ".github/workflows/ci.yml"), "utf8");
  assert.ok(ci.includes("just cargo-about"));
  assert.ok(
    ci.includes(`cargo-about-${CARGO_ABOUT_VERSION}`) || ci.includes("cargo-about-version"),
  );
});

test("ライセンスのファイルだけを文の元にする (ソースの見出しは除く)", () => {
  for (const path of [
    "/r/a-1.0.0/LICENSE",
    "/r/a-1.0.0/LICENSE-MIT",
    "/r/a-1.0.0/license-apache.md",
    "/r/a-1.0.0/COPYING",
    "/r/a-1.0.0/UNLICENSE",
    "/r/a-1.0.0/NOTICE.txt",
    "/r/a-1.0.0/COPYRIGHT",
    "/r/a-1.0.0/LICENSES/MIT.txt",
    "C:\\r\\a-1.0.0\\LICENSE.txt",
  ]) {
    assert.ok(isLicenseFile(path), path);
  }
  for (const path of [
    "/r/ring-0.17.14/src/io.rs",
    "/r/libdbus-sys-0.2.7/vendor/dbus/dbus/dbus-server-launchd.h",
    "/r/schemars-1.2.2/src/_private/regex_syntax.rs",
  ]) {
    assert.ok(!isLicenseFile(path), path);
  }
});

test("ソースの URL と authors", () => {
  assert.equal(
    crateSource(pkg("a", "1.0.0", "MIT", { repository: "https://github.com/x/a.git" })),
    "https://github.com/x/a",
  );
  assert.equal(
    crateSource(pkg("a", "1.0.0", "MIT", { homepage: "https://a.example/" })),
    "https://a.example",
  );
  assert.equal(crateSource(pkg("a", "1.0.0", "MIT")), "https://crates.io/crates/a");
  assert.deepEqual(crateAuthors({ authors: ["Jane Doe <jane@example.com>", "The A Authors"] }), [
    "Jane Doe",
    "The A Authors",
  ]);
  assert.deepEqual(crateAuthors({}), []);
});

test("標準の文から著作権者の欄の行を除く", () => {
  assert.equal(
    stripPlaceholders("MIT License\n\nCopyright (c) <year> <copyright holders>\n\nPermission ..."),
    "MIT License\n\nPermission ...",
  );
  assert.equal(
    stripPlaceholders("Copyright (c) <year> <owner>.\n\nRedistribution ..."),
    "Redistribution ...",
  );
});

test("ワークスペースのクレートとソースの見出しを除き、同じ文をまとめ、名前と版で並べる", () => {
  const a = pkg("a", "1.0.0", "MIT OR Apache-2.0", { repository: "https://github.com/x/a" });
  const b = pkg("b", "2.0.0", "MIT", { authors: ["B Author <b@example.com>"] });
  const c = pkg("c", "0.1.0", "Apache-2.0 AND ISC");
  const self = { ...pkg("ugoe-viewer", "0.0.0", "GPL-3.0-only"), source: null };
  const notices = rustNotices({
    crates: [c, a, b, self].map((p) => ({ package: p, license: p.license })),
    licenses: [
      lic("GPL-3.0-only", "GPL", null, self),
      lic("MIT", "Copyright (c) A\r\n\r\nMIT text\n", "/r/a-1.0.0/LICENSE-MIT", a),
      lic("MIT", "MIT License\n\nCopyright (c) <year> <copyright holders>\n\nMIT text", null, b),
      lic("ISC", "ISC text", "/r/c-0.1.0/LICENSE", c),
      lic("ISC", "whole source file", "/r/c-0.1.0/src/io.rs", c),
      lic("Apache-2.0", "Apache text", "/r/c-0.1.0/LICENSE-APACHE", c),
    ],
  });
  assert.deepEqual(
    notices.crates.map((x) => [x.name, x.used, x.noFile]),
    [
      ["a", ["MIT"], false],
      ["b", ["MIT"], true],
      ["c", ["Apache-2.0", "ISC"], false],
    ],
  );
  assert.deepEqual(
    notices.texts.map((t) => [t.id, t.standard, t.crates, t.text]),
    [
      ["Apache-2.0", false, ["c 0.1.0"], "Apache text"],
      ["ISC", false, ["c 0.1.0"], "ISC text"],
      ["MIT", false, ["a 1.0.0"], "Copyright (c) A\n\nMIT text"],
      ["MIT", true, ["b 2.0.0"], "MIT License\n\nMIT text"],
    ],
  );

  const text = renderRustNotices(notices).join("\n");
  assert.ok(
    text.includes(
      "\na 1.0.0\n  License: MIT OR Apache-2.0 (used under MIT)\n  Source: https://github.com/x/a\n",
    ),
  );
  assert.ok(text.includes("\nb 2.0.0\n  License: MIT\n  Source: https://crates.io/crates/b\n"));
  assert.ok(text.includes("\nc 0.1.0\n  License: Apache-2.0 AND ISC\n"));
  assert.ok(text.includes("\nMIT License (MIT)\nUsed by:\n  a 1.0.0\n\nCopyright (c) A\n"));
  assert.ok(
    text.includes(
      "of their own. The copyright belongs to the authors of each crate:\n  b 2.0.0 (B Author)\n",
    ),
  );
  assert.ok(!text.includes("ugoe-viewer"));
  assert.ok(!text.includes("whole source file"));
  assert.ok(!text.includes("example.com"));
});

test("文が見つからないクレートがあれば失敗する", () => {
  const a = pkg("a", "1.0.0", "MIT");
  assert.throws(
    () =>
      rustNotices({
        crates: [{ package: a, license: "MIT" }],
        licenses: [lic("MIT", "source", "/r/a-1.0.0/src/lib.rs", a)],
      }),
    /a 1\.0\.0/,
  );
});

test("ターゲットは 1 つずつ --target に渡す", () => {
  const args = aboutArgs("a/Cargo.toml", ["x86_64-unknown-linux-gnu", "wasm32-unknown-unknown"]);
  assert.deepEqual(args.slice(-4), [
    "--target",
    "x86_64-unknown-linux-gnu",
    "--target",
    "wasm32-unknown-unknown",
  ]);
  assert.ok(!aboutArgs("a/Cargo.toml").includes("--target"));
});

test("ターゲットは 1 つずつ --filter-platform に渡す (cargo metadata)", () => {
  const args = metadataArgs("a/Cargo.toml", ["x86_64-unknown-linux-gnu", "wasm32-unknown-unknown"]);
  assert.deepEqual(args.slice(-4), [
    "--filter-platform",
    "x86_64-unknown-linux-gnu",
    "--filter-platform",
    "wasm32-unknown-unknown",
  ]);
  assert.ok(args.includes("--frozen"));
  assert.ok(!metadataArgs("a/Cargo.toml", []).includes("--filter-platform"));
});

/** cargo metadata の packages の 1 つ (`kind` は lib の target の kind)。 */
function metaPkg(name, version, kind = ["lib"]) {
  return {
    id: `${REGISTRY}#${name}@${version}`,
    name,
    version,
    targets: [{ kind }, { kind: ["test"] }],
  };
}

/** cargo metadata の resolve の deps の 1 つ。 */
function dep(to, ...kinds) {
  return { pkg: to.id, dep_kinds: (kinds.length > 0 ? kinds : [null]).map((kind) => ({ kind })) };
}

test("手続きマクロのクレートと、手続きマクロ・ビルドスクリプトだけが使うクレートを求める", () => {
  const app = { ...metaPkg("app", "0.0.0", ["bin"]), id: "path+file:///r/app#0.0.0" };
  const serde = metaPkg("serde", "1.0.0");
  const serdeDerive = metaPkg("serde_derive", "1.0.0", ["proc-macro"]);
  const syn = metaPkg("syn", "2.0.0");
  const quote = metaPkg("quote", "1.0.0");
  const pm2 = metaPkg("proc-macro2", "1.0.0");
  const cc = metaPkg("cc", "1.0.0");
  const runtimeSyn = metaPkg("parser", "1.0.0");
  const devOnly = metaPkg("png", "0.18.0");
  const node = (p, ...deps) => ({ id: p.id, deps });
  const metadata = {
    packages: [app, serde, serdeDerive, syn, quote, pm2, cc, runtimeSyn, devOnly],
    workspace_members: [app.id],
    resolve: {
      root: app.id,
      nodes: [
        node(app, dep(serde), dep(runtimeSyn), dep(cc, "build"), dep(devOnly, "dev")),
        node(serde, dep(serdeDerive)),
        node(serdeDerive, dep(syn), dep(quote), dep(pm2)),
        node(syn, dep(quote), dep(pm2)),
        node(quote, dep(pm2)),
        node(pm2),
        node(cc),
        // 通常の依存から使われる proc-macro2 は残す
        node(runtimeSyn, dep(pm2)),
        node(devOnly),
      ],
    },
  };
  assert.deepEqual([...compileOnlyCrates(metadata)].sort(), [
    ["cc 1.0.0", "compile-time"],
    ["png 0.18.0", "compile-time"],
    ["quote 1.0.0", "compile-time"],
    ["serde_derive 1.0.0", "proc-macro"],
    ["syn 2.0.0", "compile-time"],
  ]);
  // 根が無ければワークスペースのクレートから
  const noRoot = { ...metadata, resolve: { ...metadata.resolve, root: null } };
  assert.equal(compileOnlyCrates(noRoot).size, 5);
});

test("コンパイルのときだけ使うクレートを表記から除き、数を数える", () => {
  const a = pkg("a", "1.0.0", "MIT");
  const derive = pkg("a_derive", "1.0.0", "MIT");
  const syn = pkg("syn", "2.0.0", "MIT OR Apache-2.0");
  const notices = rustNotices(
    {
      crates: [a, derive, syn].map((p) => ({ package: p, license: p.license })),
      licenses: [
        lic("MIT", "MIT text", "/r/a-1.0.0/LICENSE", a, derive),
        lic("MIT", "syn MIT text", "/r/syn-2.0.0/LICENSE-MIT", syn),
      ],
    },
    {
      compileOnly: new Map([
        ["a_derive 1.0.0", "proc-macro"],
        ["syn 2.0.0", "compile-time"],
      ]),
    },
  );
  assert.deepEqual(
    notices.crates.map((c) => c.name),
    ["a"],
  );
  // 除いたクレートだけが使う文も載らない
  assert.deepEqual(
    notices.texts.map((t) => [t.text, t.crates]),
    [["MIT text", ["a 1.0.0"]]],
  );
  assert.deepEqual(notices.excluded, { "proc-macro": 1, "compile-time": 1 });
  assert.match(excludedSummary(notices), /手続きマクロのクレート 1 個.* 1 個を除いた/);
});

test("選べるライセンスは MIT・MIT-0・Apache-2.0 の順に選ぶ (about.toml の accepted の並び)", () => {
  const toml = readFileSync(join(import.meta.dirname, "..", ABOUT_CONFIG), "utf8");
  const accepted = aboutList(toml, "accepted");
  assert.deepEqual(accepted.slice(0, 3), ["MIT", "MIT-0", "Apache-2.0"]);
  assert.ok(aboutList(toml, "targets").includes("x86_64-unknown-linux-gnu"));
  assert.equal(aboutList(toml, "missing"), undefined);
});
