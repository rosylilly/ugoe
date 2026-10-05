// エディタ (apps/editor) の主な流れ: 見本のプロジェクトを開く・パラメータを動かす・保存する・PSD を取り込む・`.ugoe` を書き出す・
// プレビューでモジュールを付け外しし、画像からトラッキングする・言語を切り替える。
// 流し方は tests/e2e/README.md。

import assert from "node:assert/strict";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe } from "node:test";
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
import { installPageHelpers } from "../lib/page.mjs";
import { checkImageTracking } from "../lib/tracking.mjs";
import { Keys, waitFor } from "../lib/webdriver.mjs";

/**
 * 次に開くダイアログ (`open-file`・`open-directory`・`save-file`) が `path` を選んだことにする (1 回だけ。決めていなければ取り消し)。
 * OS のファイルダイアログの代わりに、デバッグビルドのバックエンドが答えのファイルを読む (`UGOE_EDITOR_DIALOG_ANSWERS`、
 * apps/editor/src-tauri/src/dialog.rs)。バックエンドが選んだパスを覚えるので、コマンドはそのパスを読み書きできる (T1616)。
 */
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

describe("editor", () => {
  const app = launch({ name: "editor" });
  const project = join(work(), "ugoe-dress-up-sample", "dress-up.ugoeproj");

  step(app, "starts", async () => {
    const s = app.session;
    await s.waitFindText("nav.file-menu button", "Open Sample", 20_000);
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
    const button = await s.waitFindText("nav.file-menu button", "Open Sample");
    await button.click();
    await waitFor(() => existsSync(project), { message: `${project} to be written` });
    await s.waitFindText("nav.file-menu .project .name", "dress-up", 20_000);
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
    await s.waitFind("nav.file-menu .project .dirty");
    const save = await s.waitFindText("nav.file-menu button", "Save");
    await save.click();
    await waitFor(
      () => s.execute(`return __e2e.byText(".ugoe-toast__message", "Saved to ") !== null`),
      {
        message: "the saved notice",
      },
    );
    await waitFor(async () => (await s.findAll("nav.file-menu .project .dirty")).length === 0, {
      message: "the modified mark to go away",
    });
    const files = readdirSync(project, { recursive: true }).filter((f) => f.endsWith(".json"));
    const written = files.some((f) => readFileSync(join(project, f), "utf8").includes("9.8.7"));
    assert.ok(written, `9.8.7 is not in the saved project (${files.join(", ")})`);
  });

  step(app, "imports a PSD and its layers appear in the tree", async () => {
    const s = app.session;
    answerDialog("open-file", join(root, "assets", "fixtures", "character.psd"));
    const button = await s.waitFindText("nav.file-menu button", "Import PSD");
    await button.click();
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
    await s.waitFind("nav.file-menu .project .dirty");
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

  step(app, "switches the language to Japanese and keeps it", async () => {
    const s = app.session;
    const settings = await s.waitFindText("button", "Settings");
    await settings.click();
    const general = await s.waitFindText("[role=dialog] [role=tab]", "General");
    await general.click();
    const japanese = await s.waitFindText("[role=dialog] .ugoe-segmented__item", "日本語");
    await japanese.click();
    await waitFor(
      () => s.execute(`return __e2e.byText("nav.file-menu button", "^開く…$") !== null`),
      {
        message: "the file menu in Japanese",
      },
    );
    await s.keys(Keys.Escape);
    assert.equal(await s.execute(`return localStorage.getItem("ugoe-editor.locale")`), "ja");
    // 読み込み直しても日本語のまま (WebView のデータはテスト用の XDG_DATA_HOME の下)
    await s.call("POST", "/refresh", {});
    await s.waitFindText("nav.file-menu button", "保存", 20_000);
    await shot(s, "editor-japanese");
  });

  checkNoCspViolations(app);
});
