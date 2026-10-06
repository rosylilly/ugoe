// エディタ (apps/editor) のマニュアルの画像 (docs/manual/images) を撮る (`just e2e --manual --only editor`)。
// マニュアルのエディタの章は英語の画面の名前で書くので、英語の画面で撮る。流し方は tests/e2e/README.md#マニュアルの画像。
//
// 見本の着せ替え (「Open Sample…」) と `assets/fixtures/character.psd` から撮る。チュートリアルの途中の姿 (ワープ・ミラー・深度) と
// MCP・AI の注文と作り直しの画像は撮らない (tests/e2e/README.md#マニュアルの画像 の「撮らないもの」)。

import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { before, describe } from "node:test";
import { dialogAnswers, launch, root, step, work } from "../lib/harness.mjs";
import { manualShot, resizeTo } from "../lib/manual.mjs";
import { installPageHelpers } from "../lib/page.mjs";
import { startFakeTracking } from "../lib/tracking.mjs";
import { Keys, waitFor } from "../lib/webdriver.mjs";

/** 次に開くファイルのダイアログが `path` を選んだことにする (specs/editor.test.mjs と同じ)。 */
function answerDialog(kind, path) {
  writeFileSync(join(dialogAnswers(), kind), path);
}

/** ヘッダーの「File」のメニューを開き、項目 `text` を選ぶ。 */
async function fileMenu(session, text) {
  await session.clickText("nav.file-menu button", "File");
  await session.clickText("[role=menuitem]", text);
}

/** 名前 (aria-label か文字) が `pattern` に合う、無効でないボタンを押す。 */
function clickButton(session, pattern, scope = "body") {
  return session.clickWhen(
    () =>
      session.execute(
        (pattern, scope) => {
          const re = new RegExp(pattern);
          const name = (el) =>
            el.getAttribute("aria-label") ?? (el.textContent ?? "").replace(/\s+/g, " ").trim();
          for (const root of document.querySelectorAll(scope)) {
            for (const el of root.querySelectorAll("button, [role=tab], [role=menuitem]")) {
              if (!el.disabled && re.test(name(el))) return el;
            }
          }
          return null;
        },
        pattern,
        scope,
      ),
    { message: `a button named ${pattern}` },
  );
}

/** 右の列のタブ (アイコンだけ。名前は読み上げの名前) を開く。 */
function openTab(session, label) {
  return session.clickWhen(() => session.execute(`return __e2e.byRole("tab", "^${label}")`), {
    message: `the ${label} tab`,
  });
}

/** ツリーの行 (名前が `name`) を押して選ぶ。畳まれていれば親を開く。 */
async function selectRow(session, name) {
  await session.clickWhen(
    () =>
      session.execute((name) => {
        const rows = [...document.querySelectorAll("[role=treeitem]")];
        const row = rows.find((r) => r.querySelector(".name")?.textContent.trim() === name);
        return row?.querySelector(".name") ?? null;
      }, name),
    { message: `the tree row ${name}` },
  );
}

/** 出ている知らせ (トースト) をすべて閉じる。 */
async function dismissToasts(session) {
  await session.execute(() => {
    for (const toast of document.querySelectorAll(".ugoe-toast")) {
      toast.querySelector("button")?.click();
    }
  });
  await waitFor(async () => (await session.findAll(".ugoe-toast")).length === 0, {
    timeout: 5_000,
    message: "the toasts to close",
  }).catch(() => {});
}

/** ビューポートが描けるまで待つ (HUD の「Screenshot」が押せる)。 */
function viewportReady(session) {
  return waitFor(
    () =>
      session.execute(() => {
        const button = document.querySelector(".viewport .hud button");
        return button !== null && !button.disabled;
      }),
    { timeout: 30_000, message: "the viewport to be ready" },
  );
}

/** パラメータを既定の値に戻し (タイムラインのキーを選ぶと再生位置の値になる)、選択を外す。 */
async function resetPose(session) {
  await clickButton(session, "^Reset$", ".pane.parameters");
  await session.execute(() => {
    const row = document.querySelector("[role=treeitem][aria-selected=true]");
    row?.focus();
  });
  await session.keys(Keys.Escape);
  await new Promise((r) => setTimeout(r, 300));
}

/** 右の列の中の `el` を、送れる祖先の頭 (上に残るタブの下) まで送る。 */
function scrollInRight(session, el) {
  return session.execute((el) => {
    let box = el.parentElement;
    while (box !== null) {
      const o = getComputedStyle(box).overflowY;
      if (box.scrollHeight > box.clientHeight && (o === "auto" || o === "scroll")) break;
      box = box.parentElement;
    }
    if (box === null) return;
    const tabs = box.querySelector("[role=tablist]");
    const sticky = tabs && getComputedStyle(tabs.parentElement).position === "sticky";
    const offset = sticky ? tabs.getBoundingClientRect().height + 8 : 0;
    box.scrollTop += el.getBoundingClientRect().top - box.getBoundingClientRect().top - offset;
  }, el);
}

/** 右の列の送れる所をすべて頭に戻す。 */
function scrollRightToTop(session) {
  return session.execute(() => {
    for (const el of document.querySelectorAll("aside.right, aside.right *")) el.scrollTop = 0;
  });
}

/** 保存していない変更を捨てるかを訊かれたら、捨てて続ける (訊かれなければ何もしない)。 */
async function discardIfAsked(session) {
  const button = await waitFor(
    () =>
      session.execute(() =>
        [...document.querySelectorAll("[role=alertdialog] button, [role=dialog] button")].find(
          (b) => b.textContent.includes("Discard and continue"),
        ),
      ),
    { timeout: 3_000, message: "the discard dialog" },
  ).catch(() => null);
  if (button) await button.click();
}

/** プロジェクト (`.ugoeproj`) を「Open…」で開く。 */
async function openProject(session, path) {
  answerDialog("open-directory", path);
  await fileMenu(session, "Open…");
  await discardIfAsked(session);
  await viewportReady(session);
  await dismissToasts(session);
}

/** すべての送れる所を頭に戻し、ポインタの Tooltip が出ないようにフォーカスを外す。 */
async function settle(session, ms = 600) {
  await session.execute(() => document.activeElement?.blur?.());
  await new Promise((r) => setTimeout(r, ms));
}

describe("editor manual images", () => {
  const app = launch({ name: "editor" });
  const sampleDir = join(work(), "ugoe-dress-up-sample");
  const psd = join(root, "assets", "fixtures", "character.psd");
  const litProject = join(work(), "avatar-lit.ugoeproj");

  // 法線マップのある見本のアバター (ライティングと検証の画像。`ugoe sample avatar --lit`)
  before(() => {
    const result = spawnSync(
      "just",
      [
        "cli",
        "sample",
        "avatar",
        "--lit",
        "-o",
        join(work(), "avatar-lit.ugoe"),
        "--project",
        litProject,
      ],
      { cwd: root, stdio: "inherit" },
    );
    if (result.status !== 0) throw new Error("ugoe sample avatar --lit failed");
  });

  step(app, "starts", async () => {
    const s = app.session;
    await s.waitFindText("nav.file-menu button", "File", 20_000);
    await installPageHelpers(s);
    await resizeTo(s, 1280, 800);
  });

  step(app, "screen-empty", async () => {
    const s = app.session;
    await settle(s);
    await manualShot(s, "screen-empty");
  });

  step(app, "tutorial-import", async () => {
    const s = app.session;
    answerDialog("open-file", psd);
    await fileMenu(s, "New from PSD…");
    await waitFor(() => s.execute(() => document.querySelectorAll("[role=treeitem]").length > 3), {
      timeout: 30_000,
      message: "the PSD in the tree",
    });
    await viewportReady(s);
    await dismissToasts(s);
    await settle(s, 1000);
    await manualShot(s, "tutorial-import");
  });

  step(app, "dress-up-errors", async () => {
    const s = app.session;
    // スロットの無いプロジェクト (取り込んだだけの character.psd) に、見本のモジュールを読み込む (付けられない)
    await clickButton(s, "^Preview$", "header");
    await s.waitFindText(".preview-panel h2", "Preview");
    answerDialog("open-file", join(root, "assets", "samples", "dress-up", "cat-ears.ugoemod"));
    await clickButton(s, "^Load \\.ugoemod", ".dress-up");
    const section = await s.waitFind(".dress-up");
    await waitFor(
      () => s.execute(() => document.querySelector(".dress-up .ugoe-notice") !== null),
      { timeout: 20_000, message: "the error in the dress-up section" },
    );
    await scrollInRight(s, section);
    await settle(s, 300);
    await manualShot(s, "dress-up-errors", { clip: section, pad: 8 });
    await clickButton(s, "^Exit preview$", "header");
    await waitFor(async () => (await s.findAll(".preview-panel")).length === 0, {
      message: "the preview to close",
    });
  });

  step(app, "screen", async () => {
    const s = app.session;
    answerDialog("open-directory", work());
    await fileMenu(s, "Open Sample");
    await s.clickText("[role=alertdialog] button, [role=dialog] button", "Discard and continue");
    await s.waitFindText(".project-info .project .name", "dress-up", 20_000);
    await viewportReady(s);
    await dismissToasts(s);
    await selectRow(s, "Face");
    await settle(s, 1000);
    await manualShot(s, "screen");
  });

  step(app, "timeline", async () => {
    const s = app.session;
    // カーブのある行とキーを選んでグラフを出す (tests/ui-audit の timeline-graph と同じ)
    await s.clickWhen(() => s.find(".motion-editor .label.has-curve button.track"), {
      message: "a track with a curve",
    });
    await s.clickWhen(
      () =>
        s.execute(
          () =>
            document.querySelectorAll(".motion-editor .lane.selected button.diamond")[1] ?? null,
        ),
      { message: "a key" },
    );
    await settle(s);
    await manualShot(s, "timeline");
    await clickButton(s, "^Settings…$", ".motion-editor");
    await s.waitFind(".ugoe-popover");
    await settle(s, 400);
    await manualShot(s, "timeline-settings");
    await s.keys(Keys.Escape);
  });

  step(app, "expressions", async () => {
    const s = app.session;
    await clickButton(s, "^Expressions \\(");
    await settle(s);
    await manualShot(s, "expressions");
    await clickButton(s, "^Motions \\(");
    // ここからはビューポートを広く使う: タイムラインを畳み、パラメータを既定の値に戻す
    await clickButton(s, "^Collapse the timeline$");
    await resetPose(s);
  });

  step(app, "tools", async () => {
    const s = app.session;
    // ワープのギズモ (見本の HeadWarp)、回転のギズモとインスペクタの角度の欄 (キーの間の HeadRotation)、メッシュの編集 (Face)
    await selectRow(s, "HeadWarp");
    await settle(s);
    await manualShot(s, "tools-warp-gizmo");
    await selectRow(s, "HeadRotation");
    await settle(s);
    await manualShot(s, "tools-rotation");
    await selectRow(s, "Face");
    await clickButton(s, "^Edit mesh$");
    await settle(s);
    await manualShot(s, "tools-mesh-edit");
    await clickButton(s, "^Done$");
    await resetPose(s);
  });

  step(app, "physics", async () => {
    const s = app.session;
    await openTab(s, "Physics");
    await s.clickWhen(() => s.execute(() => document.querySelector("aside.right li.rig button")), {
      message: "the first rig",
    });
    await scrollRightToTop(s);
    await settle(s);
    await manualShot(s, "physics-rig");
    // 「Templates」の節
    const templates = await waitFor(
      () =>
        s.execute(() =>
          [...document.querySelectorAll("aside.right section")].find((el) =>
            /^Templates/.test(el.querySelector("h3")?.textContent.trim() ?? ""),
          ),
        ),
      { message: "the Templates section" },
    );
    await scrollInRight(s, templates);
    await settle(s, 300);
    await manualShot(s, "physics-templates", { clip: templates, pad: 8 });
    await scrollRightToTop(s);
    // プレビュー: 頭を傾けた直後
    await clickButton(s, "^Preview$", "aside.right");
    const angleZ = await waitFor(
      () =>
        s.execute(() =>
          [...document.querySelectorAll("input.value")].find((el) =>
            /^AngleZ value$/i.test(el.getAttribute("aria-label") ?? ""),
          ),
        ),
      { message: "the AngleZ field" },
    );
    await angleZ.replace("30");
    await new Promise((r) => setTimeout(r, 350));
    await manualShot(s, "physics-preview");
    await clickButton(s, "^Stop preview$", "aside.right");
    await resetPose(s);
  });

  step(app, "slots", async () => {
    const s = app.session;
    await resizeTo(s, 1280, 1000);
    await openTab(s, "Slots");
    await scrollRightToTop(s);
    await settle(s);
    const pane = await s.find("aside.right .pane.inspector");
    await manualShot(s, "slots", { clip: pane });
    const order = await waitFor(
      () =>
        s.execute(() =>
          [...document.querySelectorAll("aside.right section")].find(
            (el) => el.querySelector("h3")?.textContent.trim() === "Draw order",
          ),
        ),
      { message: "the Draw order section" },
    );
    await scrollInRight(s, order);
    await settle(s, 300);
    await manualShot(s, "slots-draw-order", { clip: pane });
    await scrollRightToTop(s);
    await openTab(s, "Inspector");
    await resizeTo(s, 1280, 800);
  });

  step(app, "preview", async () => {
    const s = app.session;
    await clickButton(s, "^Preview$", "header");
    await s.waitFindText(".preview-panel h2", "Preview");
    await viewportReady(s);
    await clickButton(s, "^Play shake$", ".preview-panel");
    await clickButton(s, "^Expression$", ".preview-panel");
    await s.clickText("[role=option]", "smile");
    await new Promise((r) => setTimeout(r, 700));
    await settle(s, 200);
    await manualShot(s, "preview");
  });

  step(app, "preview-tracking", async () => {
    const s = app.session;
    await startFakeTracking(s, { seconds: 30 });
    await s.execute(() =>
      document.querySelector(".preview-panel .tracking")?.scrollIntoView({ block: "start" }),
    );
    await new Promise((r) => setTimeout(r, 1500));
    await manualShot(s, "preview-tracking");
    await s.execute(() => window.__UGOE_TRACKER__?.stop());
  });

  step(app, "dress-up-preview", async () => {
    const s = app.session;
    for (const file of ["long-hair.ugoemod", "sailor.ugoemod"]) {
      answerDialog("open-file", join(root, "assets", "samples", "dress-up", file));
      await clickButton(s, "^Load \\.ugoemod", ".dress-up");
      await s
        .waitFindText(".dress-up .modules > li", file.replace(".ugoemod", ""), 20_000)
        .catch(() => {});
    }
    await waitFor(
      () => s.execute(() => document.querySelectorAll(".dress-up .modules > li").length >= 2),
      { timeout: 20_000, message: "two modules in the preview" },
    );
    await s.execute(() => document.querySelector(".dress-up")?.scrollIntoView({ block: "start" }));
    await new Promise((r) => setTimeout(r, 1000));
    await settle(s, 200);
    await manualShot(s, "dress-up-preview");
  });

  step(app, "preview-module-pose", async () => {
    const s = app.session;
    const pose = await waitFor(
      () => s.execute(() => document.querySelector(".preview-panel section[aria-label=Pose]")),
      { message: "the Pose section" },
    );
    await s.execute((el) => el.scrollIntoView({ block: "start" }), pose);
    // Sailor のグループの 2 つ目の差分を選ぶ
    await s.clickWhen(
      () =>
        s.execute((pose) => {
          const groups = [...pose.querySelectorAll(".ugoe-segmented")];
          const sailor = groups.at(-1);
          return sailor?.querySelectorAll("button, [role=radio]")[1] ?? null;
        }, pose),
      { message: "a Sailor pose entry" },
    );
    await new Promise((r) => setTimeout(r, 800));
    await settle(s, 200);
    await manualShot(s, "preview-module-pose");
  });

  step(app, "close-preview", async () => {
    const s = app.session;
    await clickButton(s, "^Exit preview$", "header");
    await waitFor(async () => (await s.findAll(".preview-panel")).length === 0, {
      message: "the preview to close",
    });
  });

  step(app, "settings", async () => {
    const s = app.session;
    await clickButton(s, "^Settings$", "header");
    const dialog = await s.waitFind("[role=dialog]");
    await clickButton(s, "^General$", "[role=dialog]");
    await settle(s, 400);
    await manualShot(s, "settings-general", { clip: dialog });
    await clickButton(s, "^AI provider$", "[role=dialog]");
    await s.clickWhen(() => s.find('[role=dialog] input[type=radio][value="openai_compatible"]'), {
      message: "the OpenAI-compatible choice",
    });
    await s.clickWhen(
      () =>
        s.execute(() => {
          const label = [...document.querySelectorAll("[role=dialog] span[id]")].find(
            (l) => l.textContent.trim() === "Service",
          );
          return label ? document.querySelector(`[aria-labelledby="${label.id}"]`) : null;
        }),
      { message: "the Service field" },
    );
    await s.clickText("[role=option]", "OpenRouter");
    await settle(s, 400);
    // 送り先の欄 (「Apply」の行まで)。API キーの欄は OS の鍵の保管庫が要り、e2e のセッションには無いので撮らない
    const rect = await s.execute((dialog) => {
      const apply = [...dialog.querySelectorAll("button")].find(
        (b) => b.textContent.trim() === "Apply",
      );
      const d = dialog.getBoundingClientRect();
      const bottom = (apply ?? dialog).getBoundingClientRect().bottom + 12;
      return {
        x: Math.floor(d.left),
        y: Math.floor(d.top),
        width: Math.ceil(d.width),
        height: Math.ceil(bottom - d.top),
      };
    }, dialog);
    await manualShot(s, "ai-settings", { clip: rect });
    await s.keys(Keys.Escape);
  });

  step(app, "lighting", async () => {
    const s = app.session;
    await openProject(s, litProject);
    await selectRow(s, "face");
    await openTab(s, "Lighting");
    await scrollRightToTop(s);
    await settle(s, 1000);
    await manualShot(s, "lighting");
    await openTab(s, "Inspector");
    await resetPose(s);
  });

  step(app, "verify", async () => {
    const s = app.session;
    const angleZ = await waitFor(
      () =>
        s.execute(() =>
          [...document.querySelectorAll("input.value")].find((el) =>
            /^AngleZ value$/i.test(el.getAttribute("aria-label") ?? ""),
          ),
        ),
      { message: "the AngleZ field" },
    );
    await angleZ.replace("30");
    await openTab(s, "Verify");
    await clickButton(s, "^Verify all cells$", "aside.right");
    await new Promise((r) => setTimeout(r, 2000));
    await scrollRightToTop(s);
    await settle(s, 600);
    await manualShot(s, "verify");
    await openTab(s, "Inspector");
    await resetPose(s);
  });

  step(app, "module-workspace", async () => {
    const s = app.session;
    await openProject(s, join(sampleDir, "cat-ears.ugoeproj"));
    await s.waitFind(".project-info button.module", 20_000);
    await settle(s, 1000);
    await manualShot(s, "module-workspace");
  });
});
