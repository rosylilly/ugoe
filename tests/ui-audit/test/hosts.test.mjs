// UI の監査の偽のホスト・静的サーバ・アプリの定義のテスト (`just ui-audit-check`)
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import { pattern, text } from "../lib/apps.mjs";
import { createEditorHost, ipcBinding } from "../lib/editor-host.mjs";
import { contentType, resolvePath, serve } from "../lib/serve.mjs";
import { hostMethods, studioFixture } from "../lib/studio-host.mjs";

const root = join(import.meta.dirname, "..", "..", "..");
const work = mkdtempSync(join(tmpdir(), "ugoe-ui-audit-test-"));
after(() => rmSync(work, { recursive: true, force: true }));

/** `ugoe sample dress-up --project` の形の小さなプロジェクト。 */
function fakeProject() {
  const dir = join(work, "sample.ugoeproj");
  mkdirSync(join(dir, "motions"), { recursive: true });
  mkdirSync(join(dir, "expressions"), { recursive: true });
  writeFileSync(join(dir, "project.json"), JSON.stringify({ format_version: 0, name: "sample" }));
  writeFileSync(
    join(dir, "model.json"),
    JSON.stringify({
      schema_version: 0,
      meta: { name: "sample", canvas: { width: 8, height: 8, origin: [4, 4] } },
      parameters: [{ id: "AngleX" }],
      parts: [{ id: "Body" }],
      effects: { breath: { parameters: ["AngleX"] } },
    }),
  );
  writeFileSync(join(dir, "motions", "nod.motion.json"), JSON.stringify({ name: "nod" }));
  writeFileSync(
    join(dir, "expressions", "smile.expression.json"),
    JSON.stringify({ name: "smile" }),
  );
  const bundle = join(work, "sample.ugoe");
  writeFileSync(bundle, Buffer.from([1, 2, 3]));
  return { project: dir, bundle };
}

test("エディタの偽のホスト: 開く前は空のモデル、開いた後は見本を返す", () => {
  const host = createEditorHost(fakeProject());
  assert.equal(host.handle("project_status").value.path, null);
  const empty = JSON.parse(host.handle("get_model_json").value);
  // 一覧は空、省ける欄 (effects) は除き、形は保つ
  assert.deepEqual(empty.parameters, []);
  assert.deepEqual(empty.parts, []);
  assert.equal(empty.effects, undefined);
  assert.equal(empty.meta.name, "Untitled");
  assert.equal(host.handle("get_model_bundle").error.code, "ui_audit.read_only");

  const status = host.handle("open_project", { path: "/somewhere/x.ugoeproj" }).value;
  assert.deepEqual(status, { name: "sample", path: "/somewhere/x.ugoeproj", modified: false });
  assert.equal(JSON.parse(host.handle("get_model_json").value).parts.length, 1);
  assert.deepEqual(host.handle("get_animations").value, {
    motions: [{ name: "nod" }],
    expressions: [{ name: "smile" }],
  });
  assert.deepEqual([...host.handle("get_model_bundle").bytes], [1, 2, 3]);
});

test("エディタの偽のホスト: 編集は読むだけのエラー、知らないコマンドは null で名前を残す", () => {
  const host = createEditorHost(fakeProject());
  const r = host.handle("apply_command", { command: { kind: "x" } });
  assert.equal(r.error.code, "ui_audit.read_only");
  assert.equal(r.error.args.command, "apply_command");
  assert.deepEqual(host.handle("some_new_command"), { value: null });
  assert.deepEqual([...host.unknown], ["some_new_command"]);
});

test("エディタの偽のホスト: ページへはバイト列を base64 で渡す", () => {
  const host = createEditorHost(fakeProject());
  const call = ipcBinding(host);
  host.handle("open_project", { path: "/p" });
  assert.deepEqual(JSON.parse(call("get_model_bundle", "{}")), { bytes: "AQID" });
  assert.deepEqual(JSON.parse(call("app_info", "{}")), { value: { version: "0.0.0-ui-audit" } });
});

test("スタジオの偽のホスト: StudioHost のメソッドをすべて置き、見本のプロファイルに既定値を足す", async () => {
  const methods = hostMethods(readFileSync(join(root, "apps/studio/src/lib/host.ts"), "utf8"));
  for (const name of ["restoreProfile", "subscribeStatus", "systemLocale", "reportLoadFailure"]) {
    assert.ok(methods.includes(name), name);
  }
  assert.throws(() => hostMethods("export interface Other {}"), /StudioHost/);
  const fixture = await studioFixture("en");
  assert.deepEqual(fixture.methods, methods);
  assert.equal(fixture.profile.format, "ugoe-studio-profile");
  assert.equal(typeof fixture.profile.input.smoothing, "number");
  assert.equal(typeof fixture.profile.motion.autoIdle, "boolean");
  assert.ok(fixture.profile.output.virtualCamera);
  assert.equal(fixture.profile.scenes[0].name, "Chat");
});

test("アプリの定義が使う文言のキーが、英語と日本語のカタログにある", () => {
  const source = readFileSync(join(root, "tests/ui-audit/lib/apps.mjs"), "utf8");
  const calls = [...source.matchAll(/text\("(\w+)", ctx\.lang, (`[^`]+`|"[^"]+")\)/g)];
  assert.ok(calls.length > 5);
  for (const [, app, raw] of calls) {
    const keys = raw.startsWith("`")
      ? ["physics", "glue", "lighting", "slots", "poses", "verify", "ai"].map((id) =>
          raw.slice(1, -1).replace(/\$\{id\}/, id),
        )
      : [raw.slice(1, -1)];
    for (const key of keys) {
      for (const lang of ["en", "ja"])
        assert.equal(typeof text(app, lang, key), "string", `${app} ${lang} ${key}`);
    }
  }
  assert.throws(() => text("editor", "en", "no.such.key"), /no text/);
});

test("引数を含む文言を正規表現に", () => {
  const re = pattern("Expressions ({{count, number}})");
  assert.ok(re.test("Expressions (3)"));
  assert.ok(re.test("Expressions (1,024)"));
  assert.ok(!re.test("Expressions"));
});

test("静的サーバ: dist の中だけを配り、wasm の型を付ける", async () => {
  const dist = join(work, "dist");
  mkdirSync(join(dist, "assets"), { recursive: true });
  writeFileSync(join(dist, "index.html"), "<p>hi</p>");
  writeFileSync(join(dist, "assets", "a.wasm"), Buffer.from([0, 97, 115, 109]));
  assert.equal(resolvePath(dist, "/../secret"), null);
  assert.equal(resolvePath(dist, "/%2e%2e/secret"), null);
  assert.equal(resolvePath(dist, "/assets/a.wasm?x=1"), join(dist, "assets", "a.wasm"));
  assert.equal(contentType("x.wasm"), "application/wasm");
  assert.equal(contentType("x.unknown"), "application/octet-stream");
  const server = await serve(dist);
  try {
    const page = await fetch(`${server.url}/`);
    assert.equal(await page.text(), "<p>hi</p>");
    const wasm = await fetch(`${server.url}/assets/a.wasm`);
    assert.equal(wasm.headers.get("content-type"), "application/wasm");
    assert.equal((await fetch(`${server.url}/missing.js`)).status, 404);
  } finally {
    await server.close();
  }
});
