import assert from "node:assert/strict";
import { test } from "node:test";
import { findCaseCollisions } from "./check-case-collisions.mjs";

test("names that differ only in case collide", () => {
  assert.deepEqual(findCaseCollisions(["notes/README.md", "notes/readme.md", "notes/other.md"]), [
    { dir: "notes", names: ["README.md", "readme.md"] },
  ]);
});

test("a .svelte.ts module collides with a component its import resolves to", () => {
  assert.deepEqual(findCaseCollisions(["lib/FileMenu.svelte", "lib/fileMenu.svelte.ts"]), [
    { dir: "lib", names: ["FileMenu.svelte", "fileMenu.svelte.ts"] },
  ]);
  assert.deepEqual(findCaseCollisions(["lib/Foo.svelte", "lib/foo.ts"]), []);
});

test("directories are compared too", () => {
  assert.deepEqual(findCaseCollisions(["src/Panels/a.ts", "src/panels.ts"]), [
    { dir: "src", names: ["Panels", "panels.ts"] },
  ]);
});

test("same import name with the same case is not a case collision", () => {
  assert.deepEqual(findCaseCollisions(["x/foo.ts", "x/foo.js", "x/foo.d.ts"]), []);
  assert.deepEqual(findCaseCollisions(["a/foo.md", "b/Foo.md"]), []);
});
