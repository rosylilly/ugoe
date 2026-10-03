// scripts/check-pack.mjs のパスの変換のテスト (`just app-build-check`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { windowsPath, windowsSampleProfile } from "./check-pack.mjs";

test("/mnt/<ドライブ>/ の下を Windows のパスにし、それ以外は null", () => {
  assert.equal(windowsPath("/mnt/c/Users/a/ugoe-check"), "C:\\Users\\a\\ugoe-check");
  assert.equal(windowsPath("/mnt/d/x/"), "D:\\x");
  assert.equal(windowsPath("/mnt/c"), "C:\\");
  assert.equal(windowsPath("/home/a/ugoe-check"), null);
  assert.equal(windowsPath("/mnt/wsl/x"), null);
});

test("見本のプロファイルのファイルはすべて Windows の区切りで samples の下を指す", () => {
  const samples = "C:\\Users\\a\\ugoe-check\\samples";
  const profile = windowsSampleProfile(samples);
  const files = [
    ...profile.scenes.flatMap((s) => [s.model, ...s.modules]),
    ...profile.hotkeys.flatMap((h) => (h.action.kind === "dressUp" ? [h.action.module] : [])),
  ];
  assert.ok(files.length > 0);
  for (const file of files) {
    assert.ok(file.startsWith(`${samples}\\`), file);
    assert.ok(!file.includes("/"), file);
  }
  assert.ok(files.includes(`${samples}\\dress-up\\long-hair.ugoemod`));
});
