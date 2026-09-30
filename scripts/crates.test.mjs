// scripts/crates.mjs の単体テスト (`just publish-check` が実行する)。
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  internalDependencyVersions,
  problems,
  publishOrder,
  syncVersions,
  workspaceVersion,
} from "./crates.mjs";

const TOML = [
  "[workspace]",
  'members = ["crates/*"]',
  "",
  "[workspace.package]",
  'version = "0.2.0"',
  "publish = false",
  "",
  "[workspace.dependencies]",
  'a = { path = "crates/a", version = "0.2.0" }',
  'b = { path = "crates/b", version = "0.1.0" }',
  'tool = { path = "crates/tool" }',
  'serde = { version = "1", features = ["derive"] }',
  "",
].join("\n");

const LICENSE = "MIT License\n";

function pkg(name, overrides = {}) {
  return {
    id: name,
    name,
    version: "0.2.0",
    license: "MIT",
    description: "d",
    repository: "https://example.com/r",
    readme: "README.md",
    keywords: ["animation"],
    categories: ["graphics"],
    publish: null,
    manifest_path: `/r/crates/${name}/Cargo.toml`,
    dependencies: [],
    ...overrides,
  };
}

function meta(packages) {
  return { packages, workspace_members: packages.map((p) => p.id) };
}

const dep = (name, kind = null, req = "^0.2.0") => ({ name, kind, req });

describe("crates", () => {
  it("reads the workspace version and the internal dependency versions", () => {
    assert.equal(workspaceVersion(TOML), "0.2.0");
    assert.deepEqual(
      [...internalDependencyVersions(TOML)],
      [
        ["a", "0.2.0"],
        ["b", "0.1.0"],
        ["tool", null],
      ],
    );
  });

  it("syncs only the published crates' versions", () => {
    const next = syncVersions(TOML, "0.2.0", ["a", "b"]);
    assert.match(next, /^b = \{ path = "crates\/b", version = "0\.2\.0" \}$/m);
    assert.match(next, /^tool = \{ path = "crates\/tool" \}$/m);
    assert.match(next, /^serde = \{ version = "1", features = \["derive"\] \}$/m);
    assert.equal(syncVersions(next, "0.2.0", ["a", "b"]), next);
  });

  it("orders crates after their dependencies, including dev-dependencies", () => {
    const order = publishOrder([
      pkg("sdk", { dependencies: [dep("core"), dep("format")] }),
      pkg("format", { dependencies: [dep("core"), dep("runtime", "dev")] }),
      pkg("runtime", { dependencies: [dep("core")] }),
      pkg("core"),
    ]);
    assert.deepEqual(order, ["core", "runtime", "format", "sdk"]);
    assert.throws(() =>
      publishOrder([
        pkg("x", { dependencies: [dep("y")] }),
        pkg("y", { dependencies: [dep("x", "dev")] }),
      ]),
    );
  });

  it("accepts a ready workspace", () => {
    const toml = syncVersions(TOML, "0.2.0", ["a", "b"]);
    const found = problems({
      metadata: meta([
        pkg("a"),
        pkg("b", { dependencies: [dep("a"), dep("tool", "dev", "*")] }),
        pkg("tool", { publish: [] }),
      ]),
      toml,
      license: LICENSE,
      readFile: () => LICENSE,
      tag: "v0.2.0",
      published: ["a", "b"],
    });
    assert.deepEqual(found, []);
  });

  it("reports what blocks publishing", () => {
    const found = problems({
      metadata: meta([
        pkg("a", { keywords: ["a", "b", "c", "d", "e", "f"], readme: null }),
        pkg("b", { dependencies: [dep("tool")] }),
        pkg("tool"),
      ]),
      toml: TOML,
      license: LICENSE,
      readFile: (path) => (path.includes("/a/") ? LICENSE : "other"),
      tag: "v0.3.0",
      published: ["a", "b"],
    });
    const has = (re) =>
      assert.ok(
        found.some((p) => re.test(p)),
        `${re} in ${JSON.stringify(found, null, 2)}`,
      );
    has(/^tool is publishable but not in PUBLISHED/);
    has(/^a: missing readme/);
    has(/^a: at most 5 keywords/);
    has(/^b: .*LICENSE must be a copy/);
    has(/^b depends on tool, which is not published/);
    has(/^\[workspace\.dependencies\] b version 0\.1\.0 differs/);
    has(/^tag v0\.3\.0 does not match/);
  });
});
