// scripts/check-hardcoded-strings.mjs の検査のテスト (`just i18n`)
import assert from "node:assert/strict";
import { test } from "node:test";
import {
  compare,
  isErrorText,
  isHardcoded,
  isTemplateSentence,
  scan,
  shrinkAllowlist,
} from "./check-hardcoded-strings.mjs";

const texts = (source, kind = "svelte") => scan(source, kind).map((f) => `${f.kind}: ${f.text}`);

test("テキストノードと表示の属性の文言を見つける", () => {
  const found = texts(
    `<button title="Save the file" aria-label="Save {name}" class="primary big">Save</button>`,
  );
  assert.deepEqual(found, ["attr title: Save the file", "attr aria-label: Save", "text: Save"]);
});

test("カタログを通した文言・記号・数値・小文字の 1 語は見つけない", () => {
  const found = texts(
    `<p title={t("a.title")}>{t("a.label")} — 42 % <span>px</span> {value}°</p>
<dt>yaw</dt><img alt="" src="x.png" />`,
  );
  assert.deepEqual(found, []);
});

test("表示する式の、値になる位置の文字列を見つける", () => {
  const found = texts(
    `<p aria-label={busy ? "Saving" : t("a.save")}>{count > 0 ? "Some" : "None"} {name ?? "Untitled"}</p>`,
  );
  assert.deepEqual(found, [
    "attr aria-label: Saving",
    "expression: Some",
    "expression: None",
    "expression: Untitled",
  ]);
});

test("比較・case・console・import・型の文字列は見つけない", () => {
  const source = `
import x from "Some module path";
type Mode = "Edit mode" | "View mode";
if (e.key === "Escape key") console.warn("Could not load the file");
switch (mode) { case "Edit mode": break; }
`;
  assert.deepEqual(texts(source, "ts"), []);
});

test("テンプレート文字列の文を見つけ、識別子・パス・URL・CSS は見つけない", () => {
  const source = `
this.busy = \`Asking \${name} to revise proposal #\${id}…\`;
const path = \`\${dir}/\${name}.png\`;
const key = \`editor.\${key}.label\`;
const url = \`http://127.0.0.1:\${port}/mcp\`;
const style = \`translate(\${x}px, \${y}px) rotate(\${r}deg)\`;
const css = \`width: \${w}px; height: \${h}px\`;
const cls = \`tool \${active ? "on" : ""}\`;
`;
  assert.deepEqual(texts(source, "ts"), ["script: Asking 0 to revise proposal #0…"]);
  assert.equal(isTemplateSentence("Failed to open 0: 0"), true);
  assert.equal(isTemplateSentence("Bearer 0"), false);
  assert.equal(isTemplateSentence("HTTP 0 error"), false);
});

test("例外の文を見つける (小文字で始まる文も)。t() を通したもの・識別子だけのものは見つけない", () => {
  const source = `
throw new Error("the proposal could not be applied");
const err = new TypeError(\`Bad value: \${value}\`);
const reject = Promise.reject(new Error("no files"));
throw "plain string thrown";
throw new Error(t("errors.adoptFailed"));
throw new Error(\`\${path}: \${response.status}\`);
throw new Error(describeError(e), { cause: e });
throw Object.assign(new Error("not_found"), { code: "not_found" });
// i18n-ignore: 開発者向け
throw new Error("#app not found");
`;
  assert.deepEqual(texts(source, "ts"), [
    "script: the proposal could not be applied",
    "script: Bad value: 0",
    "script: no files",
    "script: plain string thrown",
  ]);
  assert.equal(isErrorText("読めません"), true);
  assert.equal(isErrorText("unsupported"), false);
});

test("ファイルのダイアログのフィルタの name を見つける", () => {
  const source = `
const filters = [{ name: "PNG image (.png)", extensions: ["png"] }];
const translated = { name: t("fileMenu.filters.png"), extensions: ["png"] };
const getter = { get name() { return t("fileMenu.filters.psd"); }, extensions: ["psd"] };
const notFilter = { name: "Hair", id: "hair" };
`;
  assert.deepEqual(texts(source, "ts"), ["script: PNG image (.png)"]);
});

test("スクリプトの表示の場所・日本語・英語の文を見つける", () => {
  const source = `
const a = { label: "Save", id: "save-button", kind: "warp" };
el.textContent = "Done";
alert("Hello");
const b = "読み込みました";
const c = \`Loaded \${n} files\`;
const d = "image/png";
`;
  assert.deepEqual(texts(source, "ts"), [
    "script: Save",
    "script: Done",
    "script: Hello",
    "script: 読み込みました",
    "script: Loaded 0 files",
  ]);
});

test("i18n-ignore を書いた行と次の行は見つけない", () => {
  const source = `<!-- i18n-ignore: 製品名 -->
<h1>ugoe Editor</h1>
<p>Other text</p>
<script lang="ts">
  const name = "Product Name"; // i18n-ignore
</script>`;
  assert.deepEqual(texts(source), ["text: Other text"]);
});

test("行番号は元のファイルの行", () => {
  const found = scan(`const a = 1;\n\nconst b = "日本語";\n`, "ts");
  assert.equal(found[0].line, 3);
  const svelte = scan(`<script>\n  let x = 1;\n</script>\n\n<p>Hi there</p>\n`, "svelte");
  assert.equal(svelte[0].line, 5);
});

test("表示の場所では ID のような小文字の識別子を除く", () => {
  assert.equal(isHardcoded("dialog-title", true), false);
  assert.equal(isHardcoded("Title", true), true);
  assert.equal(isHardcoded("Title", false), false);
  assert.equal(isHardcoded("Open the file", false), true);
  // ASCII でない記号 (矢印・三点リーダ) だけでは日本語などとみなさない
  assert.equal(isHardcoded("from → to…", false), false);
  assert.equal(isHardcoded("café", false), true);
});

const item = (text) => ({ line: 1, kind: "text", text });

test("許す一覧より増えたら失敗し、減ったら縮めるよう促す", () => {
  const allowlist = {
    "apps/a": { "apps/a/A.svelte": 1, "apps/a/B.svelte": 2, "apps/a/C.svelte": 1 },
  };
  const found = new Map([
    ["apps/a/A.svelte", [item("x"), item("y")]],
    ["apps/a/B.svelte", [item("z")]],
    ["apps/a/D.svelte", [item("w")]],
  ]);
  const { errors, shrunk } = compare(found, allowlist);
  assert.match(errors[0], /A\.svelte: 直書きの文言が 2 件 \(許す一覧は 1 件\)/);
  assert.ok(errors.some((e) => /D\.svelte: 直書きの文言が 1 件 \(許す一覧は 0 件\)/.test(e)));
  assert.deepEqual(shrunk, [
    { file: "apps/a/B.svelte", from: 2, to: 1 },
    { file: "apps/a/C.svelte", from: 1, to: 0 },
  ]);
  // 縮めるときは増えた分を書かない
  assert.deepEqual(shrinkAllowlist(found, allowlist), {
    "apps/a": { "apps/a/A.svelte": 1, "apps/a/B.svelte": 1 },
  });
});
