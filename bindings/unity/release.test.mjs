// release.mjs の版の照合と tarball の中身の検査 (node --test。build.mjs --check が実行する)。
import assert from "node:assert/strict";
import { test } from "node:test";
import { PLUGINS } from "./build.mjs";
import { crateVersion, missingEntries, versionProblems } from "./release.mjs";

test("reads the workspace version", () => {
  const toml =
    '[workspace]\nmembers = []\n\n[workspace.package]\nedition = "2024"\nversion = "1.2.3"\n\n[x]\nversion = "9"\n';
  assert.equal(crateVersion(toml), "1.2.3");
  assert.throws(() => crateVersion('[package]\nversion = "1.0.0"\n'));
});

test("versions must match the crates and the tag", () => {
  assert.deepEqual(versionProblems({ packageVersion: "0.1.0", crate: "0.1.0", tag: "v0.1.0" }), []);
  assert.deepEqual(versionProblems({ packageVersion: "0.1.0", crate: "0.1.0" }), []);
  assert.equal(
    versionProblems({ packageVersion: "0.0.0", crate: "0.1.0", tag: "v0.1.0" }).length,
    1,
  );
  assert.equal(
    versionProblems({ packageVersion: "0.1.0", crate: "0.1.0", tag: "v0.1.1" }).length,
    1,
  );
});

test("the tarball needs every native library unless allowed", () => {
  const base = [
    "package.json",
    "LICENSE.md",
    "README.md",
    "Runtime/Ugoe.Runtime.asmdef",
    "Samples~/Basic/Models/avatar.ugoe",
  ];
  const plugins = PLUGINS.flatMap((p) => [
    `Runtime/Plugins/${p.path}`,
    `Runtime/Plugins/${p.path}.meta`,
  ]);
  const all = [...base, ...plugins].map((e) => `package/${e}`);
  assert.deepEqual(missingEntries(all, { allowMissing: false }), []);
  const linuxOnly = [...base, "Runtime/Plugins/Linux/x86_64/libugoe_ffi.so"].map(
    (e) => `package/${e}`,
  );
  assert.ok(
    missingEntries(linuxOnly, { allowMissing: false }).includes(
      "Runtime/Plugins/iOS/libugoe_ffi.a",
    ),
  );
  assert.deepEqual(missingEntries(linuxOnly, { allowMissing: true }), []);
  assert.equal(
    missingEntries(
      base.map((e) => `package/${e}`),
      { allowMissing: true },
    ).length,
    1,
  );
  assert.ok(
    missingEntries(
      all.filter((e) => !e.includes("Samples~")),
      { allowMissing: false },
    )[0].startsWith("Samples~"),
  );
});
