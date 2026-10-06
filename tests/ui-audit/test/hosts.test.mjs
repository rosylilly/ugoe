// UI の監査の偽のホスト・静的サーバ・アプリの定義のテスト (`just ui-audit-check`)
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import { pattern, text } from "../lib/apps.mjs";
import {
  classificationFixture,
  glueModel,
  proposalsFixture,
  renameModule,
} from "../lib/editor-fixtures.mjs";
import { createEditorHost, ipcBinding } from "../lib/editor-host.mjs";
import { contentType, resolvePath, serve } from "../lib/serve.mjs";
import {
  attentionMessages,
  firstRunMessages,
  hostMethods,
  PENDING_CLIENT,
  profileExtension,
  sampleAvatar,
  studioFixture,
} from "../lib/studio-host.mjs";

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

test("エディタの偽のホスト: ページへはバイト列を base64 で渡す", async () => {
  const host = createEditorHost(fakeProject());
  const call = ipcBinding(host);
  host.handle("open_project", { path: "/p" });
  assert.deepEqual(JSON.parse(await call("get_model_bundle", "{}")), { bytes: "AQID" });
  assert.deepEqual(JSON.parse(await call("app_info", "{}")), {
    value: { version: "0.0.0-ui-audit" },
  });
});

test("エディタの偽のホスト: AI の状態では確認を求め、同意の後の送信は取り消すまで答えない", async () => {
  const host = createEditorHost(fakeProject());
  assert.equal(host.handle("get_proposals").value.proposals.length, 0);
  host.set({ ai: true, restoring: true });
  const state = host.handle("get_proposals").value;
  assert.ok(state.proposals.length > 0);
  assert.equal(state.restoring, true);
  assert.equal(host.handle("ai_settings").value.key, "stored");
  const ask = host.handle("classify_psd_layers", { useAi: true, consent: null });
  assert.equal(ask.value.status, "needs_consent");
  const sent = host.handle("classify_psd_layers", { useAi: true, consent: { remember: false } });
  assert.ok(sent instanceof Promise);
  assert.equal(host.handle("cancel_ai_requests").value, 1);
  assert.equal((await sent).error.code, "ai.cancelled");
  assert.equal(host.handle("propose_rig_from_psd", {}).error.code, "ai.time_limit");
});

/** 小さなコンテナ (MODU と TEXR の 2 つのチャンク)。 */
function container(name) {
  const modu = Buffer.concat([
    Buffer.from([0x82, 0xae]),
    Buffer.from("schema_version"),
    Buffer.from([0x00, 0xa4]),
    Buffer.from("name"),
    Buffer.from([0xa0 + name.length]),
    Buffer.from(name),
  ]);
  const texr = Buffer.from([1, 2, 3, 4, 5]);
  const header = Buffer.alloc(16 + 2 * 24);
  header.write("UGOE", 0, "latin1");
  header.writeUInt32LE(2, 12);
  const at = [64, 64 + Math.ceil(modu.length / 16) * 16];
  [
    ["MODU", modu],
    ["TEXR", texr],
  ].forEach(([tag, data], i) => {
    header.write(tag, 16 + i * 24, "latin1");
    header.writeBigUInt64LE(BigInt(at[i]), 16 + i * 24 + 8);
    header.writeBigUInt64LE(BigInt(data.length), 16 + i * 24 + 16);
  });
  const out = Buffer.alloc(at[1] + texr.length);
  header.copy(out, 0);
  modu.copy(out, at[0]);
  texr.copy(out, at[1]);
  return out;
}

test("エディタの偽のホスト: モジュールの名前を書き換え、チャンクを 16 バイト境界に並べ直す", () => {
  const renamed = renameModule(container("Sailor"), "Sailor (editing)");
  assert.deepEqual(renamed, container("Sailor (editing)"));
  assert.throws(() => renameModule(Buffer.from("nope"), "x"), /not a ugoe container/);
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

test("スタジオの偽のホスト: 初めて起動した画面と注意の要る状態の通知", async () => {
  const fixture = await studioFixture("ja");
  const first = firstRunMessages(fixture);
  const profile = first.find((m) => m.kind === "profile");
  // profile.rs の Profile::new と同じ: 名前もモデルも無いシーンが 1 つ、保存先なし、配信していない
  assert.equal(profile.path, null);
  assert.equal(profile.live, null);
  assert.deepEqual(
    profile.profile.scenes.map((s) => [s.id, s.name, s.model]),
    [["scene-1", "", null]],
  );
  assert.deepEqual(profile.profile.hotkeys, []);
  assert.equal(first.find((m) => m.kind === "scene").scene.modelRevision, 0);
  assert.deepEqual(first.find((m) => m.kind === "api").clients, []);
  // 見本のプロファイルを書き換えない
  assert.equal(fixture.profile.scenes.length, 4);

  const { messages, status } = attentionMessages(fixture);
  const hotkeys = messages.find((m) => m.kind === "profile").profile.hotkeys;
  const problems = messages.find((m) => m.kind === "hotkeys").problems;
  // ぶつかりは、本当に同じキーの 2 つ目に出す
  const [index, problem] = problems.find(([, p]) => p.code === "duplicate");
  assert.equal(hotkeys[index].keys, hotkeys[problem.other].keys);
  assert.notEqual(fixture.profile.hotkeys[1].keys, fixture.profile.hotkeys[0].keys);
  for (const [i] of problems) assert.ok(i < hotkeys.length);
  assert.deepEqual(messages.find((m) => m.kind === "api").pending, [PENDING_CLIENT]);
  assert.ok(status.measuredRate < 60);
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

test("エディタの偽のホスト: パラメータの多いモデル (T1630)", () => {
  const host = createEditorHost(fakeProject());
  host.handle("open_project", { path: "/somewhere/x.ugoeproj" });
  host.set({ manyParameters: true });
  const model = JSON.parse(host.handle("get_model_json").value);
  assert.equal(model.parameters.length, 100);
  assert.equal(model.parameter_groups.length, 10);
  // ID は重ならず、グループの無いパラメータもある
  assert.equal(new Set(model.parameters.map((p) => p.id)).size, 100);
  assert.ok(model.parameters.some((p) => p.group === null));
  host.set({ manyParameters: false });
  assert.equal(JSON.parse(host.handle("get_model_json").value).parameters.length, 1);
});

test("エディタの偽のホスト: 見本にグルーを 1 つ足す (T0044)", () => {
  const uvs = Array.from({ length: 25 }, () => [0, 0]);
  const model = glueModel({
    meshes: [
      { id: "Face", uvs },
      { id: "HandWave", uvs },
      { id: "SleeveWave", uvs },
    ],
    parameters: [{ id: "ArmWave", min: -1, max: 1, default: 0 }],
    glues: [],
  });
  assert.equal(model.glues.length, 1);
  const [glue] = model.glues;
  assert.deepEqual([glue.mesh_a, glue.mesh_b], ["SleeveWave", "HandWave"]);
  assert.deepEqual(glue.keyforms.bindings, [{ parameter: "ArmWave", keys: [-1, 0, 1] }]);
  // キーフォームの数は軸のキーの数、頂点の番号はメッシュの中
  assert.equal(glue.keyforms.keyforms.length, 3);
  assert.ok(glue.pairs.length > 0);
  assert.ok(glue.pairs.every((p) => p.a < 25 && p.b < 25));
  // メッシュが 2 つ無ければそのまま
  const one = { meshes: [{ id: "Face", uvs }], parameters: [] };
  assert.equal(glueModel(one), one);
});

test("エディタの偽のホスト: 外での書き換えと本体の変化は、切り替えたときだけ答える (T0044)", () => {
  const host = createEditorHost(fakeProject());
  host.handle("open_project", { path: "/somewhere/x.ugoeproj" });
  assert.equal(host.handle("check_external_change").value, null);
  host.set({ externalChange: true });
  const change = host.handle("check_external_change").value;
  assert.ok(change.files.length > 3);
  assert.equal(change.missing, false);
  // 本体の変化はモジュールのプロジェクトのときだけ
  host.set({ baseChange: true });
  assert.equal(host.handle("check_base_change").value, null);
  host.set({ module: true });
  assert.ok(host.handle("check_base_change").value.files.length > 0);
  host.set({ externalChange: false, baseChange: false, module: false });
  assert.equal(host.handle("check_external_change").value, null);
  assert.equal(host.handle("check_base_change").value, null);
});

/** `ugoe_editor_core::Command` の種類 (`#[serde(tag = "op", rename_all = "snake_case")]`。apps/editor の proposals.test.ts と同じ読み方)。 */
function commandKinds() {
  const source = readFileSync(join(root, "crates/ugoe-editor-core/src/command.rs"), "utf8");
  const body = source.slice(source.indexOf("pub enum Command {"));
  return new Set(
    [...body.slice(0, body.indexOf("\n}\n")).matchAll(/^ {4}([A-Z][A-Za-z]+)\b/gm)].map((m) =>
      m[1].replace(/[A-Z]/g, (c, i) => (i === 0 ? "" : "_") + c.toLowerCase()),
    ),
  );
}

/** 分類のラベル (crates/ugoe-ai/src/classify.rs の `Label::as_str`)。 */
function classificationLabels() {
  const source = readFileSync(join(root, "crates/ugoe-ai/src/classify.rs"), "utf8");
  const body = source.slice(source.indexOf("pub const fn as_str"));
  return new Set(
    [...body.slice(0, body.indexOf("\n    }\n")).matchAll(/Label::\w+ => "(\w+)",/g)].map(
      (m) => m[1],
    ),
  );
}

test("エディタの偽のホスト: 提案の見本は実在するコマンドの種類だけを使う (T0047)", () => {
  const kinds = commandKinds();
  assert.ok(kinds.has("add_parameter") && kinds.has("set_layer_correction"));
  assert.ok(!kinds.has("set_keyform_bindings"));
  for (const proposal of proposalsFixture()) {
    for (const [i, c] of proposal.commands.entries()) {
      assert.ok(kinds.has(c.op), `proposal ${proposal.id}: ${c.op} is not an editor command`);
      // 頼るのは前のコマンドだけ
      assert.ok(
        c.requires.every((r) => r < i),
        `proposal ${proposal.id}: command ${i} requires a later one`,
      );
    }
  }
});

test("エディタの偽のホスト: 分類の見本は実在するラベルだけを使う (T0047)", () => {
  const labels = classificationLabels();
  assert.equal(labels.size, 40);
  const c = classificationFixture();
  // 選択欄は本物と同じくすべてのラベル
  assert.deepEqual(new Set(c.label_choices), labels);
  for (const layer of c.layers) assert.ok(labels.has(layer.label), layer.label);
  for (const proposal of proposalsFixture()) {
    for (const command of proposal.commands) {
      if (command.correction) assert.ok(labels.has(command.correction.label));
    }
  }
});

test("スタジオの偽のホスト: 雑談のシーンのモデルとプロファイルの拡張子は本物の見本と同じ (T0047)", async () => {
  // 保存のダイアログが付ける拡張子
  assert.equal(profileExtension(), "json");
  const fixture = await studioFixture("en");
  assert.ok(fixture.path.endsWith(".json"));
  // 配信中のシーンは見本のアバター
  const active = fixture.profile.scenes.find((s) => s.id === fixture.profile.activeScene);
  assert.ok(active.model.endsWith("avatar.ugoe"));
  const avatar = sampleAvatar();
  // assets/samples/README.md: パラメータ 27、ポーズは耳飾りの有無、モーション 4・表情 4
  assert.equal(avatar.parameters.length, 27);
  assert.deepEqual(avatar.parameters[0], {
    id: "AngleX",
    name: "AngleX",
    min: -30,
    max: 30,
    default: 0,
    internal: false,
  });
  assert.equal(avatar.parameters.find((p) => p.id === "EyeLOpen").default, 1);
  assert.deepEqual(avatar.poseGroups, [{ parts: ["earwear", "no_earwear"], module: null }]);
  assert.deepEqual(
    avatar.motions.map((m) => [m.name, m.looping, m.parameters]),
    [
      ["idle", true, ["AngleX", "BodyAngleZ"]],
      ["nod", false, ["AngleY"]],
      ["shake", false, ["AngleX"]],
      ["tilt", false, ["AngleZ", "EyeBallX"]],
    ],
  );
  assert.deepEqual(avatar.expressions, ["angry", "sad", "smile", "surprised"]);
  assert.deepEqual(fixture.poseGroups, avatar.poseGroups);
  // ホットキーが指すモーションと表情は見本にある (着せ替えのシーンだけのものは除く)
  for (const { action } of fixture.profile.hotkeys) {
    if (action.kind === "motion") assert.ok(avatar.motions.some((m) => m.name === action.motion));
    if (action.kind === "expression") assert.ok(avatar.expressions.includes(action.expression));
  }
});
