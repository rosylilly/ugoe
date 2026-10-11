// scripts/dist-notices.mjs の部品のテスト (`just licenses`)
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { DISTS, mplSourceLines, renderDistNotices } from "./dist-notices.mjs";

const root = join(import.meta.dirname, "..");

/** rustNotices の結果の形の 1 クレート。 */
function crate(name, version, license, used) {
  return {
    name,
    version,
    license,
    source: `https://github.com/example/${name}`,
    authors: [],
    used,
    noFile: false,
  };
}

const rust = {
  crates: [
    crate("godot", "0.5.5", "MPL-2.0", ["MPL-2.0"]),
    crate("serde", "1.0.0", "MIT OR Apache-2.0", ["MIT"]),
  ],
  texts: [
    {
      id: "MIT",
      name: "MIT License",
      text: "MIT text",
      standard: false,
      crates: ["serde 1.0.0"],
    },
    {
      id: "MPL-2.0",
      name: "Mozilla Public License 2.0",
      text: "MPL text",
      standard: false,
      crates: ["godot 0.5.5"],
    },
  ],
};

test("MPL-2.0 で使うクレートだけに、crates.io の版のソースの在り処を書く", () => {
  const lines = mplSourceLines(rust.crates);
  const text = lines.join("\n");
  assert.match(text, /section 3\.2 of the MPL-2\.0/);
  assert.match(text, /^godot 0\.5\.5$/m);
  assert.match(text, /https:\/\/crates\.io\/crates\/godot\/0\.5\.5/);
  assert.match(text, /https:\/\/static\.crates\.io\/crates\/godot\/godot-0\.5\.5\.crate/);
  assert.match(text, /https:\/\/github\.com\/example\/godot/);
  assert.doesNotMatch(text, /serde/);
  assert.deepEqual(mplSourceLines([rust.crates[1]]), []);
});

test("配布物の表記は、ugoe のライセンス・クレートの一覧・文・MPL のソースを並べる", () => {
  const dist = DISTS.find((d) => d.name === "godot");
  const text = renderDistNotices(dist, rust);
  assert.ok(text.startsWith(`Third-party software in ${dist.title}\n===`));
  assert.match(text, /licensed under the MIT License/);
  assert.match(text, /^serde 1\.0\.0\n {2}License: MIT OR Apache-2\.0 \(used under MIT\)$/m);
  assert.match(text, /^MPL text$/m);
  assert.ok(text.indexOf("MPL text") < text.indexOf("section 3.2"));
  assert.ok(text.endsWith("\n"));
});

test("Markdown の表記 (Unity) は、一覧と文をコードブロックに入れる", () => {
  const dist = DISTS.find((d) => d.markdown);
  const text = renderDistNotices(dist, { crates: [rust.crates[1]], texts: [rust.texts[0]] });
  assert.ok(text.startsWith(`# Third-party software in ${dist.title}\n`));
  assert.match(text, /\n```text\n[\s\S]*\nMIT text\n```\n$/);
  assert.doesNotMatch(text, /section 3\.2/);
});

test("表記の置き場と名前は配布物ごとに違う", () => {
  assert.equal(new Set(DISTS.map((d) => d.name)).size, DISTS.length);
  assert.equal(new Set(DISTS.map((d) => d.out)).size, DISTS.length);
});

test("npm のパッケージの files に表記が入る", () => {
  const web = DISTS.find((d) => d.name === "web");
  const pkg = JSON.parse(readFileSync(join(root, "bindings/web/package.json"), "utf8"));
  assert.ok(web.out.startsWith("bindings/web/"));
  assert.ok(pkg.files.includes(web.out.slice("bindings/web/".length)));
});

test("Unity の表記のターゲットは、unity.yml でビルドするターゲットを含む", () => {
  const unity = DISTS.find((d) => d.name === "unity");
  const yml = readFileSync(join(root, ".github/workflows/unity.yml"), "utf8");
  const targets = [...yml.matchAll(/rust_target: (\S+)/g)].map((m) => m[1]);
  assert.ok(targets.length > 0);
  for (const target of targets) assert.ok(unity.targets.includes(target), target);
});
