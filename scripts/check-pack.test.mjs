// scripts/check-pack.mjs のパスの変換と見本の写し方のテスト (`just app-build-check`)
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { PROFILE_FILES } from "../apps/studio/scripts/sample-profile.mjs";
import { packSamples, windowsPath } from "./check-pack.mjs";

test("/mnt/<ドライブ>/ の下を Windows のパスにし、それ以外は null", () => {
  assert.equal(windowsPath("/mnt/c/Users/a/ugoe-check"), "C:\\Users\\a\\ugoe-check");
  assert.equal(windowsPath("/mnt/d/x/"), "D:\\x");
  assert.equal(windowsPath("/mnt/c"), "C:\\");
  assert.equal(windowsPath("/home/a/ugoe-check"), null);
  assert.equal(windowsPath("/mnt/wsl/x"), null);
});

test("見本のモデルとプロファイルを samples/ にそのまま写し、前のプロファイルも置き換える", () => {
  const samples = join(import.meta.dirname, "..", "assets", "samples");
  const to = join(mkdtempSync(join(tmpdir(), "check-pack-")), "samples");
  try {
    packSamples(to);
    for (const name of Object.values(PROFILE_FILES)) {
      // モデルを相対パスで持つので、書き直さずに写す (T0865)
      assert.equal(readFileSync(join(to, name), "utf8"), readFileSync(join(samples, name), "utf8"));
    }
    assert.ok(existsSync(join(to, "avatar.ugoe")));
    assert.ok(existsSync(join(to, "dress-up", "long-hair.ugoemod")));

    const profile = join(to, PROFILE_FILES.ja);
    // スタジオは見本の印のあるプロファイルを書き換えない (T0866) ので、前のもの (印の無い前の版など) は残さない
    writeFileSync(profile, "edited");
    packSamples(to);
    assert.equal(
      readFileSync(profile, "utf8"),
      readFileSync(join(samples, PROFILE_FILES.ja), "utf8"),
    );
  } finally {
    rmSync(join(to, ".."), { recursive: true, force: true });
  }
});
