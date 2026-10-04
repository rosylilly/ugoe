// エディタ (apps/editor) の主な流れ: 見本のプロジェクトを開く・パラメータを動かす・保存する・言語を切り替える。
// 流し方は tests/e2e/README.md。

import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe } from "node:test";
import { checkNoCspViolations, invoke, launch, shot, step, work } from "../lib/harness.mjs";
import { installPageHelpers } from "../lib/page.mjs";
import { Keys, waitFor } from "../lib/webdriver.mjs";

/**
 * OS のファイルダイアログの代わり (`window.__UGOE_FILE_DIALOGS__`、apps/editor/src/lib/fileDialogs.ts)。
 * フォルダーを選ぶダイアログは `dir` を返す。ファイルを選ぶダイアログは使わない (取り消し)。
 */
function replaceDialogs(session, dir) {
  return session.execute(
    `
    const dir = arguments[0];
    window.__UGOE_FILE_DIALOGS__ = {
      openFile: async () => null,
      openDirectory: async () => dir,
      saveFile: async () => null,
      defaultDirectory: async () => dir,
    };
  `,
    dir,
  );
}

describe("editor", () => {
  const app = launch({ name: "editor" });
  const project = join(work(), "ugoe-dress-up-sample", "dress-up.ugoeproj");

  step(app, "starts", async () => {
    const s = app.session;
    await s.waitFindText("nav.file-menu button", "Open Sample", 20_000);
    await installPageHelpers(s);
    await replaceDialogs(s, work());
  });

  step(app, "opens the sample project", async () => {
    const s = app.session;
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
