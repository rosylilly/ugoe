import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { crateVersion, distTag, versionProblems } from "./release.mjs";

describe("release", () => {
  it("keeps package.json at the crates' version", () => {
    // 版はすべての配布物で揃える (docs/portability.md)。クレートの版を上げたら `node scripts/release.mjs sync`
    const pkg = JSON.parse(readFileSync(join(import.meta.dirname, "..", "package.json"), "utf8"));
    expect(pkg.version).toBe(crateVersion());
  });

  it("reads the workspace version only from [workspace.package]", () => {
    const toml = [
      "[workspace]",
      'members = ["crates/*"]',
      "",
      "[workspace.package]",
      'edition = "2024"',
      'version = "0.3.0-alpha.1"',
      "",
      "[workspace.dependencies]",
      'foo = { version = "9.9.9" }',
    ].join("\n");
    expect(crateVersion(toml)).toBe("0.3.0-alpha.1");
    expect(() => crateVersion('[package]\nversion = "1.0.0"\n')).toThrow();
  });

  it("uses the next dist-tag for prereleases", () => {
    expect(distTag("1.0.0")).toBe("latest");
    expect(distTag("1.0.0-rc.1")).toBe("next");
  });

  it("reports version and tag mismatches", () => {
    expect(versionProblems({ packageVersion: "0.1.0", crate: "0.1.0", tag: "v0.1.0" })).toEqual([]);
    expect(versionProblems({ packageVersion: "0.1.0", crate: "0.1.0" })).toEqual([]);
    expect(
      versionProblems({ packageVersion: "0.0.0", crate: "0.1.0", tag: "v0.2.0" }),
    ).toHaveLength(2);
  });
});
