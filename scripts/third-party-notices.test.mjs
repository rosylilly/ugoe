// scripts/third-party-notices.mjs の部品のテスト (`just licenses`)
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import {
  licenseOf,
  mediapipeSections,
  packageDirOf,
  packageName,
  pickLicenseFile,
  renderNotices,
  sourceUrl,
  spdxCopyright,
} from "./third-party-notices.mjs";

test("モジュールの ID から、いちばん内側の node_modules のパッケージのディレクトリを取る", () => {
  assert.equal(
    packageDirOf("/r/node_modules/.pnpm/svelte@5.57.1/node_modules/svelte/src/index.js"),
    "/r/node_modules/.pnpm/svelte@5.57.1/node_modules/svelte",
  );
  assert.equal(
    packageDirOf("/r/node_modules/.pnpm/x/node_modules/@floating-ui/dom/dist/a.mjs?commonjs"),
    "/r/node_modules/.pnpm/x/node_modules/@floating-ui/dom",
  );
  assert.equal(packageDirOf("C:\\r\\node_modules\\clsx\\dist\\clsx.mjs"), "C:/r/node_modules/clsx");
  assert.equal(packageDirOf("/r/apps/viewer/src/App.svelte"), null);
  assert.equal(packageDirOf("/r/packages/ui/src/Dialog.svelte"), null);
  assert.equal(packageDirOf("\0vite/preload-helper.js"), null);
  assert.equal(
    packageName("/r/node_modules/.pnpm/x/node_modules/@tauri-apps/api"),
    "@tauri-apps/api",
  );
  assert.equal(packageName("/r/node_modules/clsx"), "clsx");
});

test("repository をブラウザで開ける URL にする", () => {
  const url = (repository) => sourceUrl({ name: "p", repository });
  assert.equal(url("git+https://github.com/a/b.git"), "https://github.com/a/b");
  assert.equal(url({ type: "git", url: "git://github.com/a/b.git" }), "https://github.com/a/b");
  assert.equal(url("git@github.com:a/b.git"), "https://github.com/a/b");
  assert.equal(url("a/b"), "https://github.com/a/b");
  assert.equal(url("github:a/b"), "https://github.com/a/b");
  assert.equal(
    url({ url: "https://github.com/a/b", directory: "packages/c" }),
    "https://github.com/a/b/tree/HEAD/packages/c",
  );
  assert.equal(url(undefined), "https://www.npmjs.com/package/p");
});

test("license の欄が無いパッケージは、確かめたライセンスを使う", () => {
  assert.equal(licenseOf({ name: "x", license: "MIT" }), "MIT");
  assert.equal(
    licenseOf({ name: "x", licenses: [{ type: "MIT" }, "Apache-2.0"] }),
    "MIT OR Apache-2.0",
  );
  assert.equal(licenseOf({ name: "svelte-toolbelt" }), "MIT");
  assert.equal(licenseOf({ name: "unknown" }), undefined);
});

test("ライセンスの文のファイルを選ぶ (選べるなら MIT のもの、SPDX の文書は除く)", () => {
  assert.equal(pickLicenseFile(["package.json", "LICENSE.md", "README.md"], "MIT"), "LICENSE.md");
  assert.equal(pickLicenseFile(["license"], "MIT"), "license");
  assert.equal(
    pickLicenseFile(["LICENSE-APACHE-2.0", "LICENSE-MIT", "LICENSE.spdx"], "Apache-2.0 OR MIT"),
    "LICENSE-MIT",
  );
  assert.equal(pickLicenseFile(["LICENSE.spdx", "package.json"], "MIT OR Apache-2.0"), undefined);
});

test("SPDX の文書から著作権者を読む", () => {
  const text =
    "PackageName: tauri\nPackageCopyrightText: 2019-2022, The Tauri Programme\nCreated: x\n";
  assert.equal(spdxCopyright(text), "2019-2022, The Tauri Programme");
  assert.equal(spdxCopyright("PackageName: x\n"), undefined);
});

test("face-tracking の表記から、最初の節から終わりまでを取り出す", () => {
  const text = readFileSync(
    join(import.meta.dirname, "../packages/face-tracking/third-party-notices.txt"),
    "utf8",
  );
  const body = mediapipeSections(text);
  assert.ok(body.startsWith("MediaPipe Tasks Vision (@mediapipe/tasks-vision)\n---"));
  assert.ok(body.includes("Apache License"));
  assert.ok(!body.includes("This file is copied next to the MediaPipe files"));
  assert.throws(() => mediapipeSections("Title\n=====\n\nno sections\n"));
});

test("表記の全文に、アプリの名前・MediaPipe の節・各パッケージとクレートの名前と版とライセンスの文が入る", () => {
  const text = renderNotices(
    "ugoe viewer",
    "MediaPipe\n---------\n\nbody",
    [
      { name: "a", version: "1.0.0", license: "MIT", source: "https://x/a", text: "MIT A" },
      { name: "b", version: "2.0.0", license: "ISC", source: "https://x/b", text: "ISC B" },
    ],
    {
      crates: [
        {
          name: "c",
          version: "3.0.0",
          license: "MIT",
          source: "https://x/c",
          authors: [],
          used: ["MIT"],
        },
      ],
      texts: [
        { id: "MIT", name: "MIT License", text: "MIT C", standard: false, crates: ["c 3.0.0"] },
      ],
    },
  );
  assert.ok(text.startsWith("Third-party software and models bundled with ugoe viewer\n====="));
  assert.ok(text.includes("ugoe viewer itself is licensed under GPL-3.0-only"));
  assert.ok(text.includes("\nMediaPipe\n---------\n\nbody\n"));
  assert.ok(text.includes("\na 1.0.0\nLicense: MIT\nSource: https://x/a\n\nMIT A\n"));
  assert.ok(text.indexOf("a 1.0.0") < text.indexOf("b 2.0.0"));
  assert.ok(text.indexOf("ISC B\n") < text.indexOf("Rust crates compiled into the application"));
  assert.ok(text.includes("\nc 3.0.0\n  License: MIT\n  Source: https://x/c\n"));
  assert.ok(text.endsWith("Used by:\n  c 3.0.0\n\nMIT C\n"));
});
