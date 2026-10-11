// scripts/github-release.mjs の、タグの Release を選ぶ規則のテスト (`just app-build-check`)
import assert from "node:assert/strict";
import { test } from "node:test";
import { draftNotes, parseArgs, pickRelease } from "./github-release.mjs";

test("タグの Release が無ければ何も選ばない (下書きを作る)", () => {
  const picked = pickRelease([{ id: 1, tag_name: "v0.1.0", draft: false }], "v0.2.0");
  assert.equal(picked.release, null);
  assert.equal(picked.published, false);
});

test("下書きが重なったら、いちばん古いものに寄せ、ほかを余分とする", () => {
  const picked = pickRelease(
    [
      { id: 30, tag_name: "v1.0.0", draft: true },
      { id: 10, tag_name: "v1.0.0", draft: true },
      { id: 20, tag_name: "v1.0.0", draft: true },
      { id: 5, tag_name: "v0.9.0", draft: true },
    ],
    "v1.0.0",
  );
  assert.equal(picked.release.id, 10);
  assert.deepEqual(
    picked.extra.map((r) => r.id),
    [20, 30],
  );
});

test("公開済みの Release があれば、それを選んで公開済みと知らせる (添付しない)", () => {
  const picked = pickRelease(
    [
      { id: 1, tag_name: "v1.0.0", draft: true },
      { id: 2, tag_name: "v1.0.0", draft: false },
    ],
    "v1.0.0",
  );
  assert.equal(picked.release.id, 2);
  assert.equal(picked.published, true);
  assert.deepEqual(picked.extra, []);
});

test("下書きの本文は手順の節を指す", () => {
  assert.match(draftNotes("v1.0.0"), /v1\.0\.0/);
  assert.match(draftNotes("v1.0.0"), /docs\/release\.md#github-release-の下書き/);
});

test("引数: upload はタグとファイルの列、notes はタグと 1 つのファイル", () => {
  assert.deepEqual(parseArgs(["upload", "v1.0.0", "a.msi", "b.dmg"]), {
    command: "upload",
    tag: "v1.0.0",
    files: ["a.msi", "b.dmg"],
  });
  assert.deepEqual(parseArgs(["notes", "v1.0.0", "release-notes.md"]), {
    command: "notes",
    tag: "v1.0.0",
    file: "release-notes.md",
  });
  assert.equal(parseArgs(["notes", "v1.0.0"]), undefined);
  assert.equal(parseArgs(["notes", "v1.0.0", "a.md", "b.md"]), undefined);
  assert.equal(parseArgs(["upload"]), undefined);
  assert.equal(parseArgs(["publish", "v1.0.0"]), undefined);
});
