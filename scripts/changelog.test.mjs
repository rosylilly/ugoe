// scripts/changelog.mjs と scripts/cliff.toml の単体テスト (`just script-check` が実行する)。
// git-cliff そのものは呼ばない (`just ci` を流す機械に入っているとは限らないため)。
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, it } from "node:test";
import conventional from "@commitlint/config-conventional";
import { CONFIG, notesArgs, parseCliffVersion, VERSION_TAG } from "./changelog.mjs";

const cliffToml = readFileSync(join(import.meta.dirname, "..", CONFIG), "utf8");

/** cliff.toml の commit_parsers (1 行に 1 つ) の message の正規表現と、group か skip。 */
function commitParsers() {
  return [
    ...cliffToml.matchAll(/^\s*\{ message = "([^"]+)", (?:group = "([^"]+)"|skip = true) \}/gm),
  ].map((m) => ({ message: new RegExp(m[1].replaceAll("\\\\", "\\")), group: m[2] }));
}

describe("parseCliffVersion", () => {
  it("reads the version from `git-cliff --version`", () => {
    assert.equal(parseCliffVersion("git-cliff 2.14.2\n"), "2.14.2");
  });
  it("returns undefined for other output", () => {
    assert.equal(parseCliffVersion("command not found"), undefined);
  });
});

describe("VERSION_TAG", () => {
  it("accepts version tags", () => {
    for (const tag of ["v0.1.0", "v1.0.0", "v1.2.3-rc.1"]) assert.ok(VERSION_TAG.test(tag), tag);
  });
  it("rejects other names", () => {
    for (const tag of ["1.0.0", "v1.0", "release-1", "v1.0.0 extra"]) {
      assert.ok(!VERSION_TAG.test(tag), tag);
    }
  });
  it("agrees with tag_pattern in cliff.toml", () => {
    const pattern = cliffToml.match(/^tag_pattern = "([^"]+)"/m)?.[1];
    assert.ok(pattern, "tag_pattern is missing");
    const re = new RegExp(pattern.replaceAll("\\\\", "\\"));
    for (const tag of ["v0.1.0", "v1.2.3-rc.1"]) assert.ok(re.test(tag), tag);
    for (const tag of ["1.0.0", "release-1"]) assert.ok(!re.test(tag), tag);
  });
});

describe("notesArgs", () => {
  it("previews a tag that does not exist yet as the unreleased commits", () => {
    assert.deepEqual(notesArgs({ tag: "v1.0.0", exists: false }), [
      "--unreleased",
      "--tag",
      "v1.0.0",
      "--strip",
      "all",
    ]);
  });
  it("uses the range from the previous version tag", () => {
    assert.deepEqual(notesArgs({ tag: "v1.1.0", exists: true, previous: "v1.0.0", root: "abc" }), [
      "--strip",
      "all",
      "v1.0.0..v1.1.0",
    ]);
  });
  it("starts the first version at the root commit", () => {
    assert.deepEqual(notesArgs({ tag: "v1.0.0", exists: true, previous: undefined, root: "abc" }), [
      "--strip",
      "all",
      "abc..v1.0.0",
    ]);
  });
});

describe("commit_parsers in cliff.toml", () => {
  const types = conventional.rules["type-enum"][2];
  const parsers = commitParsers();
  /** type の最初に当たる parser の group (skip なら null、どれにも当たらなければ undefined)。 */
  const groupOf = (type) => {
    const p = parsers.find((p) => p.message.test(`${type}(core): do something (T0123)`));
    return p === undefined ? undefined : (p.group ?? null);
  };

  it("handles every commitlint type", () => {
    for (const type of types) assert.notEqual(groupOf(type), undefined, type);
  });
  it("lists only user-facing types", () => {
    const listed = types.filter((type) => groupOf(type) !== null).sort();
    assert.deepEqual(listed, ["feat", "fix", "perf", "revert"]);
  });
});
