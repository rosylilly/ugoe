// scripts/lib/rust-notices.mjs の部品のテスト (`just licenses`)
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import {
  CARGO_ABOUT_VERSION,
  crateAuthors,
  crateSource,
  isLicenseFile,
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
