// エディタ (apps/editor) の主な流れ: 見本のプロジェクトを開く・パラメータを動かす・保存する・PSD を取り込む・`.ugoe` を書き出す・
// プレビューでモジュールを付け外しし、画像からトラッキングする・モジュールのプロジェクトを開いて `.ugoemod` を書き出す・
// 偽の AI のプロバイダで、見本の着せ替えの本体に取り込んだ PSD と「New from PSD…」の新しいプロジェクトから提案を作って
// プレビューする・言語を切り替える。
// 流し方は tests/e2e/README.md。

import assert from "node:assert/strict";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { after, before, describe } from "node:test";
import { FAKE_REASON, startFakeAi } from "../lib/fake-ai.mjs";
import {
  checkNoCspViolations,
  dialogAnswers,
  invoke,
  launch,
  root,
  shot,
  step,
  work,
} from "../lib/harness.mjs";
import { countInvokes, installPageHelpers, invokeStats } from "../lib/page.mjs";
import { checkImageTracking } from "../lib/tracking.mjs";
import { Keys, waitFor } from "../lib/webdriver.mjs";

/**
 * 次に開くダイアログ (`open-file`・`open-directory`・`save-file`) が `path` を選んだことにする (1 回だけ。決めていなければ取り消し)。
 * OS のファイルダイアログの代わりに、デバッグビルドのバックエンドが答えのファイルを読む (`UGOE_EDITOR_DIALOG_ANSWERS`、
 * apps/editor/src-tauri/src/dialog.rs)。バックエンドが選んだパスを覚えるので、コマンドはそのパスを読み書きできる (T1616)。
 */
/** ヘッダーの「File」のメニューを開き、項目 `text` を選ぶ (T0673)。 */
async function fileMenu(session, text) {
  await session.clickText("nav.file-menu button", "File");
  await session.clickText("[role=menuitem]", text);
}

function answerDialog(kind, path) {
  writeFileSync(join(dialogAnswers(), kind), path);
}

/** バックエンドのコマンドを直に呼び、失敗したらそのコード (成功したら `null`) を返す。 */
function commandError(session, command, args) {
  return session.run(
    (command, args) =>
      window.__TAURI_INTERNALS__.invoke(command, args).then(
        () => null,
        (e) => e.code,
      ),
    command,
    args,
  );
}

/** ツリーの行 (`[role=treeitem]`) のうち、名前が `name` のもの。 */
function treeRow(session, name) {
  return session.execute(
    `return [...document.querySelectorAll("[role=treeitem]")]
       .find((row) => row.querySelector(":scope > .name, .name")?.textContent.trim() === arguments[0]) ?? null`,
    name,
  );
}

/** プレビューの着せ替えの、`.ugoemod` のファイル名を含む行の状態 (付いているか)。行が無ければ `null`。 */
function moduleRow(session, file) {
  return session.execute(
    `
    const li = [...document.querySelectorAll(".dress-up .modules > li")]
      .find((li) => li.textContent.includes(arguments[0]));
    if (!li) return null;
    const box = li.querySelector("[role=checkbox]");
    return { attached: box?.getAttribute("aria-checked") === "true", disabled: box?.disabled ?? true };
  `,
    file,
  );
}

/** プレビューの着せ替えの、スロットの選択欄に出ている値。 */
function slotValue(session, slot) {
  return session.execute(
    `const trigger = document.querySelector(\`.dress-up [aria-label="Module in slot \${arguments[0]}"]\`);
     return trigger?.querySelector(".ugoe-select__value")?.textContent.trim() ?? null;`,
    slot,
  );
}

/** 画面の通知 (トースト) に `text` を含むものが出ているか。 */
function hasToast(session, text) {
  return session.execute(
    `return [...document.querySelectorAll(".ugoe-toast__message")].some((el) => el.textContent.includes(arguments[0]))`,
    text,
  );
}

/** 物理の設定が着せ替えの本体の衝突体 `Shoulders` を持つか。 */
function hasShoulders(physics) {
  return physics?.colliders?.some((c) => c.id === "Shoulders") ?? false;
}

/**
 * AI タブで、取り込んだ PSD (character.psd) を偽のプロバイダ (`ai`) で分類してリグを提案し、提案を「Preview」して
 * 「End preview」で戻す。分類とリグの要求が画像付きで届き、提案の理由に偽の答えの文が入り、プレビューの間も後もドキュメントが
 * 変わらないことを確かめ、作った提案 (`get_proposals` の最後) を返す。
 */
async function proposeAndPreview(s, ai) {
  const sent = ai.requests.length;
  await s.clickWhen(
    () => s.execute(`return __e2e.byText(".right [role=tab]", "^AI( \\(\\d+\\))?$")`),
    { message: "the AI tab" },
  );
  await s.waitFindText(".psd", "character.psd");
  // 「Use …」を入れる (前の提案で入れたままなら押さない)
  const useAi = await waitFor(
    () =>
      s.execute(
        `const label = __e2e.byText(".ugoe-checkbox__label", "sends images to a server on this computer");
         const box = label && document.getElementById(label.htmlFor);
         return box && !box.disabled ? box : null;`,
      ),
    { message: "the Use AI checkbox" },
  );
  if ((await useAi.attribute("aria-checked")) !== "true") await useAi.click();
  await waitFor(async () => (await useAi.attribute("aria-checked")) === "true", {
    message: "the Use AI checkbox to be checked",
  });
  await s.clickText("button", "Propose rig");
  // 提案が一覧に出るか、パネルにエラーが出るまで待つ
  const made = await waitFor(
    () =>
      s.execute(`
        if (document.querySelector("article.proposal")) return { made: true };
        const error = [
          ...document.querySelectorAll(".right .error, .right .ugoe-notice[data-tone=danger]"),
        ].find((el) => el.closest("section"));
        return error ? { error: error.textContent.trim() } : null;
      `),
    { timeout: 60_000, message: "the proposal" },
  );
  assert.equal(made.error, undefined, `the proposal failed: ${made.error}`);
  const requests = ai.requests.slice(sent);
  const kinds = requests.map((r) => r.kind);
  assert.ok(kinds.includes("classify"), `no classification request: ${JSON.stringify(requests)}`);
  assert.ok(kinds.includes("values"), `no rig request: ${JSON.stringify(requests)}`);
  // 分類はレイヤーのサムネイル、リグは原画の全体を送る
  assert.ok(
    requests.every((r) => r.images > 0),
    JSON.stringify(requests),
  );
  const state = await invoke(s, "get_proposals");
  assert.equal(state.classification?.by_ai, true, "the layers were not classified by the AI");
  const proposal = state.proposals.at(-1);
  assert.ok(proposal.commands.length > 0, JSON.stringify(proposal));
  assert.ok(proposal.rationale.includes(FAKE_REASON), proposal.rationale);

  // プレビュー: 提案を複製に適用した姿 (`get_proposal_preview_bundle`) をビューポートに出す。ドキュメントは変えない
  const before = await invoke(s, "get_model_json");
  // 数は前の提案のプレビューからの通算なので、押す前との差を見る
  await countInvokes(s, ["preview_proposal", "get_proposal_preview_bundle"]);
  const counted = await invokeStats(s);
  await s.clickText("article.proposal button", "Preview");
  await s.waitFind(".viewport .proposal-banner", 30_000);
  await waitFor(
    () =>
      s.execute(`return document.querySelector("article.proposal.previewing .sliders") !== null`),
    { message: "the proposal to be previewed" },
  );
  const stats = await invokeStats(s);
  const bundles =
    (stats.done.get_proposal_preview_bundle ?? 0) - (counted.done.get_proposal_preview_bundle ?? 0);
  assert.equal(bundles, 1, JSON.stringify(stats));
  assert.deepEqual(stats.errors, {}, JSON.stringify(stats));
  const hud = await s.execute(
    `return document.querySelector(".viewport .hud .error")?.textContent.trim() ?? null`,
  );
  assert.equal(hud, null, `the viewport failed: ${hud}`);
  await shot(s, "editor-proposal-preview");

  // プレビューをやめる。ビューポートは元のモデルに戻り、ドキュメントは変わっていない
  await s.clickText("article.proposal.previewing button", "End preview");
  await waitFor(async () => (await s.findAll(".viewport .proposal-banner")).length === 0, {
    message: "the preview to end",
  });
  assert.equal(await invoke(s, "get_model_json"), before);
  return proposal;
}

describe("editor", () => {
  const app = launch({ name: "editor" });
  const project = join(work(), "ugoe-dress-up-sample", "dress-up.ugoeproj");
  /** 偽の AI のプロバイダ (lib/fake-ai.mjs)。 */
  let ai;
  before(async () => {
    ai = await startFakeAi();
  });
  after(() => ai?.close());

  step(app, "starts", async () => {
    const s = app.session;
    await s.waitFindText("nav.file-menu button", "File", 20_000);
    await installPageHelpers(s);
  });

  step(app, "refuses paths that were not chosen in a dialog", async () => {
    const s = app.session;
    // webview が渡しただけのパスは読み書きできない (src-tauri/src/access.rs、T1616)
    const psd = join(root, "assets", "fixtures", "character.psd");
    assert.equal(await commandError(s, "import_psd", { path: psd }), "file.not_chosen");
    const out = join(work(), "not-chosen.ugoe");
    assert.equal(await commandError(s, "export_bundle", { path: out }), "file.not_chosen_for_save");
    assert.ok(!existsSync(out), `${out} was written`);
  });

  step(app, "opens the sample project", async () => {
    const s = app.session;
    // 見本のフォルダーは、フォルダーを選ぶダイアログで選んだ場所の中に作る
    answerDialog("open-directory", work());
    await fileMenu(s, "Open Sample");
    await waitFor(() => existsSync(project), { message: `${project} to be written` });
    await s.waitFindText(".project-info .project .name", "dress-up", 20_000);
    await waitFor(
      () =>
        s.execute(
          `return __e2e.byText(".ugoe-toast__message", "Opened the dress-up sample") !== null`,
        ),
      {
        message: "the notice that the sample opened",
      },
    );
    // ビューポートが wasm を読んで描ける。wasm は起動の直後 (集める仕掛けを入れる前) に読み、CSP で止まっても
    // WebKitGTK はイベントを出さず HUD に出るだけなので、最後の「CSP の違反が無い」の手順では気づけない
    const hud = await waitFor(
      () =>
        s.execute(`
          const hud = document.querySelector(".viewport .hud");
          const error = hud?.querySelector(".error");
          if (error) return { error: error.textContent.trim() };
          const screenshot = hud?.querySelector("button");
          return screenshot && !screenshot.disabled ? { ready: true } : null;
        `),
      { timeout: 20_000, message: "the viewport to be ready" },
    );
    assert.equal(hud.error, undefined, `the viewport failed: ${hud.error}`);
    await shot(s, "editor-sample");
  });

  step(app, "moves a parameter", async () => {
    const s = app.session;
    const input = await waitFor(
      () =>
        s.execute(`
          for (const el of document.querySelectorAll("input.value")) {
            if (/^Angle ?X value$/i.test(el.getAttribute("aria-label") ?? "")) return el;
          }
          return null;
        `),
      { timeout: 20_000, message: "the AngleX value field" },
    );
    await input.replace("15");
    await waitFor(
      () =>
        s.execute(`
          const range = [...document.querySelectorAll("input[type=range]")]
            .find((el) => /^Angle ?X$/i.test(el.getAttribute("aria-label") ?? ""));
          return range && Math.abs(Number(range.value) - 15) < 0.01;
        `),
      { message: "the AngleX slider at 15" },
    );
    await shot(s, "editor-parameter");
  });

  step(app, "edits the model and saves the project", async () => {
    const s = app.session;
    // インスペクタのアバタープロファイルの Version (選択が無いときに出る) を変える。Command を通る本物の編集
    const version = await waitFor(
      () =>
        s.execute(`
          const label = [...document.querySelectorAll("label")].find((l) => l.textContent.trim() === "Version");
          return label?.control ?? null;
        `),
      { message: "the Version field" },
    );
    await version.replace("9.8.7");
    // 編集はバックエンドへ非同期に届く。モデルに入ってから保存する (先に保存すると前の中身が書かれる)
    await waitFor(async () => (await invoke(s, "get_model_json")).includes("9.8.7"), {
      message: "the edit in the model",
    });
    await s.waitFind(".project-info .project .dirty");
    const save = await s.waitFindText("nav.file-menu button", "Save");
    await save.click();
    await waitFor(
      () => s.execute(`return __e2e.byText(".ugoe-toast__message", "Saved to ") !== null`),
      {
        message: "the saved notice",
      },
    );
    await waitFor(async () => (await s.findAll(".project-info .project .dirty")).length === 0, {
      message: "the modified mark to go away",
    });
    const files = readdirSync(project, { recursive: true }).filter((f) => f.endsWith(".json"));
    const written = files.some((f) => readFileSync(join(project, f), "utf8").includes("9.8.7"));
    assert.ok(written, `9.8.7 is not in the saved project (${files.join(", ")})`);
  });

  step(app, "imports a PSD and its layers appear in the tree", async () => {
    const s = app.session;
    answerDialog("open-file", join(root, "assets", "fixtures", "character.psd"));
    await fileMenu(s, "Import PSD");
    // 取り込みは別のスレッドで作り、適用してから画面に届く (T0685)。PSD のファイル名の最上位のパーツができる
    const top = await waitFor(() => treeRow(s, "character"), {
      timeout: 30_000,
      message: "the imported PSD in the tree",
    });
    if ((await top.attribute("aria-expanded")) === "false") {
      const expand = await top.find("button[aria-label=Expand]");
      await expand.click();
    }
    // PSD のグループ `頭` (character の直下) と、その中のレイヤー
    const head = await waitFor(() => treeRow(s, "頭"), { message: "the group 頭 in the tree" });
    if ((await head.attribute("aria-expanded")) === "false") {
      const expand = await head.find("button[aria-label=Expand]");
      await expand.click();
    }
    await waitFor(() => treeRow(s, "前髪"), { message: "the layer 前髪 in the tree" });
    const model = JSON.parse(await invoke(s, "get_model_json"));
    for (const layer of ["背景", "後ろ髪", "首", "服", "顔", "右目", "左目", "口", "前髪"]) {
      assert.ok(
        model.meshes.some((m) => m.name === layer),
        `the layer ${layer} is not a mesh in the model`,
      );
    }
    await s.waitFind(".project-info .project .dirty");
    await shot(s, "editor-psd");
  });

  step(app, "exports a .ugoe file", async () => {
    const s = app.session;
    const dir = join(work(), "export");
    mkdirSync(dir, { recursive: true });
    const path = join(dir, "dress-up-with-psd.ugoe");
    answerDialog("save-file", path);
    const open = await s.waitFindText("nav.file-menu button", "Export…");
    await open.click();
    const button = await s.waitFindText(".export-panel button", "Export .ugoe");
    await button.click();
    await waitFor(
      () => s.execute(`return __e2e.byText(".ugoe-toast__message", "Exported to ") !== null`),
      { timeout: 30_000, message: "the exported notice" },
    );
    assert.ok(existsSync(path), `${path} was not written`);
    // 取り込んだ PSD のテクスチャも入るので、小さすぎない
    assert.ok(statSync(path).size > 10_000, `${path} is only ${statSync(path).size} bytes`);
    // 一時ファイルに書いてから名前を変える (T0687)。一時ファイルが残らない
    assert.deepEqual(readdirSync(dir), ["dress-up-with-psd.ugoe"]);
    await s.keys(Keys.Escape);
  });

  step(app, "puts a module on and takes it off in the preview", async () => {
    const s = app.session;
    const preview = await waitFor(() => s.execute(`return __e2e.byText("button", "^Preview$")`), {
      message: "the Preview button",
    });
    await preview.click();
    await s.waitFindText(".preview-panel h2", "Preview");
    // 見本の着せ替えのモジュール (ビューアの見本と同じもの)。読み込むとスロット Headwear に付く
    const file = "cat-ears.ugoemod";
    answerDialog("open-file", join(root, "assets", "samples", "dress-up", file));
    const load = await s.waitFindText(".dress-up button", "Load .ugoemod");
    await load.click();
    await waitFor(async () => (await moduleRow(s, file))?.attached, {
      timeout: 20_000,
      message: "the module to be put on",
    });
    await waitFor(async () => (await slotValue(s, "Headwear")) === "CatEars", {
      message: "the Headwear slot to show CatEars",
    });
    await shot(s, "editor-dress-up-on");

    // チェックを外すと外れる
    const box = await waitFor(
      () =>
        s.execute(
          `return [...document.querySelectorAll(".dress-up .modules > li")]
             .find((li) => li.textContent.includes(arguments[0]))?.querySelector("[role=checkbox]") ?? null`,
          file,
        ),
      { message: "the module checkbox" },
    );
    await waitFor(async () => (await moduleRow(s, file))?.disabled === false, {
      message: "the module checkbox to be enabled",
    });
    await box.click();
    await waitFor(async () => (await moduleRow(s, file))?.attached === false, {
      message: "the module to be taken off",
    });
    await waitFor(async () => (await slotValue(s, "Headwear")) === "(none)", {
      message: "the Headwear slot to show (none)",
    });

    // スロットの選択欄から付け直す
    const trigger = await s.waitFind('.dress-up [aria-label="Module in slot Headwear"]');
    await trigger.click();
    const option = await s.waitFindText("[role=option]", "CatEars");
    await option.click();
    await waitFor(async () => (await moduleRow(s, file))?.attached, {
      message: "the module to be put on again from the slot",
    });
    await shot(s, "editor-dress-up-again");
  });

  step(app, "tracks a face in an image with MediaPipe in the preview", async () => {
    const s = app.session;
    const result = await checkImageTracking(s, "editor-tracking-image");
    assert.ok(result.after > result.before, JSON.stringify(result));
    const exit = await s.waitFindText("button", "Exit preview");
    await exit.click();
    await waitFor(async () => (await s.findAll(".preview-panel")).length === 0, {
      message: "the preview to close",
    });
  });

  step(app, "opens a module project and exports a .ugoemod", async () => {
    const s = app.session;
    // 見本のモジュールのプロジェクト (見本を開いたときに本体の隣に書き出したもの)。取り込んだ PSD は保存していないので、
    // 開く前に変更を捨てるかを訊かれる
    const moduleProject = join(work(), "ugoe-dress-up-sample", "cat-ears.ugoeproj");
    answerDialog("open-directory", moduleProject);
    await fileMenu(s, "Open…");
    await s.clickText("[role=alertdialog] button, [role=dialog] button", "Discard and continue");
    await s.waitFind(".project-info button.module", 20_000);
    await waitFor(async () => (await s.findAll(".project-info .project .dirty")).length === 0, {
      message: "the module project to open without changes",
    });

    const dir = join(work(), "export-module");
    mkdirSync(dir, { recursive: true });
    const path = join(dir, "cat-ears.ugoemod");
    answerDialog("save-file", path);
    await s.clickText("nav.file-menu button", "Export Module…");
    await s.clickText(".export-panel button", "Export .ugoemod");
    await waitFor(() => hasToast(s, `Exported to ${path}`), {
      timeout: 30_000,
      message: "the exported notice",
    });
    // 一時ファイルに書いてから名前を変える (T0687)。一時ファイルが残らない
    assert.deepEqual(readdirSync(dir), ["cat-ears.ugoemod"]);
    // 見本の本体の上で、見本のモジュールのプロジェクトをそのまま書き出すので、見本の `.ugoemod` と同じバイト列になる
    // (apps/editor/src-tauri/src/sample.rs のテストと同じ)
    const sample = readFileSync(join(root, "assets", "samples", "dress-up", "cat-ears.ugoemod"));
    assert.ok(
      readFileSync(path).equals(sample),
      `${path} differs from the sample cat-ears.ugoemod`,
    );
    await shot(s, "editor-module-export");
    await s.keys(Keys.Escape);
  });

  step(app, "points the AI provider at a fake one", async () => {
    const s = app.session;
    // 設定の「AI provider」で、OpenAI 互換の API の送り先を偽のプロバイダ (127.0.0.1) にする (モデルは既定のまま)。
    // ループバックなので送る前の確認も API キーも要らない
    await s.clickText("button", "Settings");
    await s.clickText("[role=dialog] [role=tab]", "AI provider");
    await s.clickWhen(() => s.find('[role=dialog] input[type=radio][value="openai_compatible"]'), {
      message: "the OpenAI-compatible choice",
    });
    const field = (label) =>
      waitFor(
        () =>
          s.execute(
            `const label = [...document.querySelectorAll("[role=dialog] form.provider label")]
               .find((l) => l.textContent.trim() === arguments[0]);
             return label?.control ?? null;`,
            label,
          ),
        { message: `the ${label} field` },
      );
    // 打った後の Enter で設定の欄を送る (「Apply」と同じ)
    await (await field("Base URL")).replace(ai.baseUrl);
    await waitFor(
      async () => {
        const settings = await invoke(s, "ai_settings");
        return settings.local && settings.provider.openai.base_url === ai.baseUrl;
      },
      { message: "the fake provider to be applied" },
    );
    await s.keys(Keys.Escape);
    await waitFor(async () => (await s.findAll("[role=dialog]")).length === 0, {
      message: "the settings to close",
    });
  });

  step(
    app,
    "proposes a rig on the dress-up base with an imported PSD and previews it",
    async () => {
      const s = app.session;
      // 見本の着せ替えの本体を開き直す (今はモジュールのプロジェクト。変更は無いので確かめずに開く)。
      // 本体は物理の衝突体 `Shoulders` を持ち、スロット `Hair` がそれを公開する
      answerDialog("open-directory", project);
      await fileMenu(s, "Open…");
      await s.waitFindText(".project-info .project .name", "dress-up", 20_000);
      await waitFor(async () => (await s.findAll(".project-info button.module")).length === 0, {
        message: "the base project to open",
      });
      const physics = await invoke(s, "get_physics");
      assert.ok(
        hasShoulders(physics),
        `the base has no Shoulders collider: ${JSON.stringify(physics)}`,
      );
      const hair = JSON.parse(await invoke(s, "get_model_json")).slots.find(
        (slot) => slot.id === "Hair",
      );
      assert.deepEqual(hair?.colliders, ["Shoulders"], JSON.stringify(hair));

      // 本体に character.psd を取り込む。取り込んだ PSD が AI タブの提案の元になる
      answerDialog("open-file", join(root, "assets", "fixtures", "character.psd"));
      await fileMenu(s, "Import PSD");
      await waitFor(() => treeRow(s, "character"), {
        timeout: 30_000,
        message: "the imported PSD in the tree",
      });

      // 提案は本体の物理 (衝突体 `Shoulders`) に髪揺れのリグを足す。前は本体の物理を外して試し、スロット `Hair` の
      // 衝突体の参照が何も指さなくなって提案が作れなかった (T1627)
      const proposal = await proposeAndPreview(s, ai);
      assert.ok(
        proposal.commands.some((c) => c.op === "add_physics_rig"),
        `no hair sway in the proposal: ${proposal.rationale}`,
      );

      // 採用すると、本体の物理に髪揺れのリグが足され、衝突体 `Shoulders` とスロットの参照が残る
      await s.clickText("article.proposal button", "Adopt");
      const adopted = await waitFor(
        async () => {
          const after = await invoke(s, "get_physics");
          return after && after.rigs.length > physics.rigs.length ? after : null;
        },
        { timeout: 30_000, message: "the proposal to be adopted" },
      );
      assert.ok(
        hasShoulders(adopted),
        `the Shoulders collider is gone: ${JSON.stringify(adopted)}`,
      );
      for (const rig of physics.rigs) {
        assert.ok(
          adopted.rigs.some((r) => r.id === rig.id),
          `the base rig ${rig.id} is gone: ${adopted.rigs.map((r) => r.id).join(", ")}`,
        );
      }
      const model = JSON.parse(await invoke(s, "get_model_json"));
      assert.deepEqual(model.slots.find((slot) => slot.id === "Hair")?.colliders, ["Shoulders"]);
      await shot(s, "editor-proposal-dress-up");
    },
  );

  step(app, "proposes a rig in a new project from a PSD and previews it", async () => {
    const s = app.session;
    // 「New from PSD…」で character.psd から新しいプロジェクトを作る (本体の要素の無いプロジェクト)。
    // 前の手順で採用した提案は保存していないので、作る前に変更を捨てるかを訊かれる
    answerDialog("open-file", join(root, "assets", "fixtures", "character.psd"));
    await fileMenu(s, "New from PSD…");
    await s.clickText("[role=alertdialog] button, [role=dialog] button", "Discard and continue");
    await waitFor(
      async () => (await treeRow(s, "character")) && (await invoke(s, "get_physics")) === null,
      { timeout: 30_000, message: "the new project from the PSD in the tree" },
    );
    await proposeAndPreview(s, ai);
  });

  step(app, "switches the language to Japanese and keeps it", async () => {
    const s = app.session;
    const settings = await s.waitFindText("button", "Settings");
    await settings.click();
    const general = await s.waitFindText("[role=dialog] [role=tab]", "General");
    await general.click();
    const japanese = await s.waitFindText("[role=dialog] .ugoe-segmented__item", "日本語");
    await japanese.click();
    await waitFor(
      () => s.execute(`return __e2e.byText("nav.file-menu button", "^ファイル$") !== null`),
      {
        message: "the file menu in Japanese",
      },
    );
    await s.keys(Keys.Escape);
    assert.equal(await s.execute(`return localStorage.getItem("ugoe.editor.locale")`), "ja");
    // 読み込み直しても日本語のまま (WebView のデータはテスト用の XDG_DATA_HOME の下)
    await s.call("POST", "/refresh", {});
    await s.waitFindText("nav.file-menu button", "保存", 20_000);
    await shot(s, "editor-japanese");
  });

  checkNoCspViolations(app);
});
