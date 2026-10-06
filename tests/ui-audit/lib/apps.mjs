// 3 つのアプリの開き方と場面 (tests/ui-audit/README.md#場面)。
// ボタンの名前はアプリのカタログ (apps/<アプリ>/locales/<言語>.json) から引く (文言が変わっても追える)。
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { createEditorHost, editorPageInit, ipcBinding } from "./editor-host.mjs";
import {
  attentionMessages,
  firstRunMessages,
  studioFixture,
  studioPageInit,
} from "./studio-host.mjs";

const root = join(import.meta.dirname, "..", "..", "..");

const catalogs = new Map();
/** アプリのカタログから文言を引く (`a.b.c` の形のキー)。 */
export function text(app, lang, key) {
  const name = `${app}/${lang}`;
  if (!catalogs.has(name)) {
    catalogs.set(
      name,
      JSON.parse(readFileSync(join(root, "apps", app, "locales", `${lang}.json`), "utf8")),
    );
  }
  const value = key.split(".").reduce((o, k) => o?.[k], catalogs.get(name));
  if (typeof value !== "string")
    throw new Error(`no text for ${key} in apps/${app}/locales/${lang}.json`);
  return value;
}

/** 引数 (`{{count, number}}` など) を含む文言を、数に合う正規表現に。 */
export function pattern(value) {
  const parts = value.split(/\{\{[^}]*\}\}/).map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  return new RegExp(`^${parts.join("[\\d,.]+")}$`);
}

/** 設定のダイアログを開いて撮り、Esc で閉じる。 */
async function settingsScene(page, ctx, name) {
  await page.getByRole("button", { name, exact: true }).first().click();
  await page.getByRole("dialog").first().waitFor({ timeout: 5000 });
  await ctx.scene("settings");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);
}

/**
 * 要素を送れる祖先 (右の列か、狭い幅では画面の全体) の頭まで送る (ページの中で動く)。scrollIntoView は送らない祖先 (body)
 * まで動かすので、送れる祖先だけを送る。送った後に枠の頭に固定された要素 (sticky。右の列のタブ・ツリーの頭) があれば、その下に
 * 隠れないよう、その下端と 8px の分だけ戻す (T0048)。
 */
function scrollInBox(el) {
  const scrolls = (node) => {
    const o = getComputedStyle(node).overflowY;
    return o === "auto" || o === "scroll";
  };
  let box = el.parentElement;
  while (box !== null) {
    if (box.scrollHeight > box.clientHeight && scrolls(box)) break;
    box = box.parentElement;
  }
  if (box === null) return;
  box.scrollTop += el.getBoundingClientRect().top - box.getBoundingClientRect().top;
  const top = box.getBoundingClientRect().top;
  // 送った後の位置は画素に丸められるので、頭ではなく下端と比べる
  const bottom = el.getBoundingClientRect().bottom;
  let cover = 0;
  for (const s of box.querySelectorAll("*")) {
    if (s.contains(el)) continue;
    const style = getComputedStyle(s);
    if (style.position !== "sticky" || style.top === "auto") continue;
    // 間に別の送る枠があれば、その枠に固定される
    let p = s.parentElement;
    while (p !== null && p !== box && !scrolls(p)) p = p.parentElement;
    if (p !== box) continue;
    const r = s.getBoundingClientRect();
    if (r.height > 0 && r.top < bottom) cover = Math.max(cover, r.bottom - top);
  }
  if (cover > 0) box.scrollTop -= cover + 8;
}

/**
 * 折りたためるまとまり (`Section`) の見出しのボタンを、送れる祖先の頭まで送る。
 * ツールバーの同じ名前のボタン (「出力ウィンドウ」) を拾わないよう、`main` の中で探す。
 */
async function scrollToSection(page, name) {
  await page
    .locator("main")
    .getByRole("button", { name, exact: true })
    .first()
    .evaluate(scrollInBox);
}

/** `locator` の要素を、送れる祖先の頭まで送る (`scrollInBox`)。 */
async function scrollToElement(locator) {
  await locator.first().evaluate(scrollInBox);
}

/** エディタの右の列の AI のタブを開き直す (パネルを作り直して、偽のホストの状態を読み直させる)。 */
async function reopenAiTab(page, ctx) {
  // ヘッダーのボタンの Tooltip (ダイアログを閉じてフォーカスが戻ると出る) がタブに重ならないよう、フォーカスとポインタを外す
  await page.evaluate(() => document.activeElement?.blur());
  await page.mouse.move(1, 300);
  await page.waitForTimeout(300);
  await page
    .getByRole("tab", { name: text("editor", ctx.lang, "app.tabs.inspector.label"), exact: true })
    .click();
  await page.waitForTimeout(200);
  await page
    .getByRole("tab", { name: new RegExp(`^${text("editor", ctx.lang, "app.tabs.ai.label")}`) })
    .first()
    .click();
  await page.waitForTimeout(600);
}

/** エディタで見本のプロジェクトを開き直す (偽のホストの状態を変えた後、モジュールの一覧などを読み直させる)。 */
async function reopenEditorProject(page, ctx) {
  await page.evaluate((p) => {
    window.__uiAuditNextPath = p;
  }, ctx.sample.project);
  await page
    .getByRole("button", { name: text("editor", ctx.lang, "fileMenu.menu.label"), exact: true })
    .first()
    .click();
  await page
    .getByRole("menuitem", {
      name: new RegExp(`^${text("editor", ctx.lang, "fileMenu.open.label")}`),
    })
    .first()
    .click();
  await page.waitForTimeout(1500);
}

/** エディタのプレビューに入る・出る (ヘッダーのボタン)。 */
async function togglePreview(page, ctx, enter) {
  const key = enter ? "app.preview.label" : "app.preview.exit";
  await page
    .getByRole("button", { name: text("editor", ctx.lang, key), exact: true })
    .first()
    .click();
  await page.waitForTimeout(enter ? 2500 : 800);
}

/** 送った所をすべて頭に戻す。 */
async function scrollToTop(page) {
  await page.evaluate(() => {
    for (const el of [document.scrollingElement, ...document.querySelectorAll("*")])
      if (el) el.scrollTop = 0;
  });
}

/**
 * エディタの AI の場面 (T0676): 設定の AI のタブ、提案と分類のある AI のタブ (頭と提案の一覧)、分類のキャッシュを戻している最中、
 * 送る前の確認、送っている最中、取り消した後、失敗。
 */
async function aiScenes(page, ctx) {
  const host = ctx.editorHost;
  host.set({ ai: true });
  // 設定の「AI provider」(キーを保存した状態)
  await page
    .getByRole("button", { name: text("editor", ctx.lang, "app.settings.label"), exact: true })
    .first()
    .click();
  await page.getByRole("dialog").first().waitFor({ timeout: 5000 });
  await page
    .getByRole("tab", { name: text("editor", ctx.lang, "settings.ai.tab"), exact: true })
    .click();
  await page.waitForTimeout(400);
  await ctx.scene("settings-ai");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);

  // タブを開いていない間にエージェントが提案を置いた: AI のタブに数と見てほしい変化の印 (T1632)
  await page.evaluate(() => document.activeElement?.blur());
  await page.mouse.move(1, 300);
  await page.evaluate(() => window.__uiAuditEmit("mcp-bridge", { kind: "proposals" }));
  await page.waitForTimeout(600);
  await ctx.scene("ai-fresh", { aria: false });

  await reopenAiTab(page, ctx);
  await ctx.scene("ai");
  await scrollToElement(
    page.getByRole("heading", {
      name: text("editor", ctx.lang, "proposals.list.title"),
      exact: true,
    }),
  );
  await ctx.scene("ai-list", { aria: false });
  await scrollToTop(page);

  host.set({ restoring: true });
  await reopenAiTab(page, ctx);
  await ctx.scene("ai-restoring", { aria: false });
  host.set({ restoring: false });
  await reopenAiTab(page, ctx);

  // 送る前の確認 → 送っている最中 → 取り消した → 失敗
  await page
    .getByRole("checkbox", { name: /Claude/ })
    .first()
    .check();
  await page
    .getByRole("button", { name: text("editor", ctx.lang, "proposals.classify"), exact: true })
    .click();
  await page.getByRole("alertdialog").first().waitFor({ timeout: 5000 });
  await ctx.scene("consent");
  await page
    .getByRole("button", { name: text("editor", ctx.lang, "consent.send.label"), exact: true })
    .click();
  await page.waitForTimeout(1500);
  await ctx.scene("ai-sending", { aria: false });
  await page
    .getByRole("button", { name: text("editor", ctx.lang, "proposals.cancel.label"), exact: true })
    .first()
    .click();
  await page.waitForTimeout(500);
  await ctx.scene("ai-cancelled", { aria: false });
  await page
    .getByRole("button", { name: text("editor", ctx.lang, "proposals.propose"), exact: true })
    .click();
  await page.waitForTimeout(600);
  await ctx.scene("ai-error", { aria: false, errors: true });
  await page
    .getByRole("checkbox", { name: /Claude/ })
    .first()
    .uncheck();
  host.set({ ai: false });
}

/**
 * エディタの着せ替えの場面 (T0676): 本体のプレビューの着せ替え (見本のモジュールと読めないファイル)、モジュールのプロジェクトの
 * AI のタブとプレビュー (今の編集とファイルの違いの一覧)。
 */
async function dressUpScenes(page, ctx) {
  const host = ctx.editorHost;
  const dressUpTitle = page.getByRole("heading", {
    name: text("editor", ctx.lang, "preview.dressUp.title"),
    exact: true,
  });
  host.set({ dressUp: true });
  await reopenEditorProject(page, ctx);
  await togglePreview(page, ctx, true);
  await scrollToElement(dressUpTitle);
  await ctx.scene("preview-dress-up", { errors: true });
  await scrollToTop(page);
  await togglePreview(page, ctx, false);

  host.set({ module: true });
  await reopenEditorProject(page, ctx);
  // 本体の変化: ウィンドウに戻ったときの確かめで見つかったダイアログ (T0953)
  host.set({ baseChange: true });
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await page.getByRole("dialog").first().waitFor({ timeout: 5000 });
  await page.waitForTimeout(300);
  await ctx.scene("base-change");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);
  host.set({ baseChange: false });
  // 本体の要素 (見本の Head) を選んだ行に乗せた (T0851)
  await treeSelectedHoverScene(page, ctx, "part:Head", "tree-base-hover");
  await reopenAiTab(page, ctx);
  await ctx.scene("module-ai", { aria: false });
  await togglePreview(page, ctx, true);
  // 違いを比べ終えるのを待って開く
  const differences = page.locator(".differences summary, .differences-toggle").first();
  await differences.waitFor({ timeout: 10000 }).catch(() => {});
  if ((await differences.count()) > 0) await differences.click();
  await page.waitForTimeout(400);
  await scrollToElement(dressUpTitle);
  await ctx.scene("module-preview", { errors: true });
  await scrollToTop(page);
  await togglePreview(page, ctx, false);
  host.set({ module: false, dressUp: false });
}

/**
 * パラメータの多いモデル (100 個・グループ 10 個。T1630) のパラメータのパネル: 一覧の頭と、一覧の下の「Groups」を開いた所。
 */
async function manyParametersScenes(page, ctx) {
  const host = ctx.editorHost;
  host.set({ manyParameters: true });
  await reopenEditorProject(page, ctx);
  await ctx.scene("many-parameters", { aria: false });
  const groups = page.locator(".pane.parameters").getByRole("button", {
    name: pattern(text("editor", ctx.lang, "parameters.groups.title_other")),
  });
  await groups.first().click();
  await page.waitForTimeout(300);
  await scrollToElement(groups);
  await ctx.scene("many-parameters-groups", { aria: false });
  await scrollToTop(page);
  // 一覧の上の絞り込みの欄 (T1646): 語で絞った一覧 (閉じたグループも開いて出る) と、一致なしの文
  const filter = page.getByRole("searchbox", {
    name: text("editor", ctx.lang, "parameters.filter.aria"),
    exact: true,
  });
  await filter.first().fill("long");
  await page.waitForTimeout(400);
  await ctx.scene("parameter-filter", { aria: false });
  await filter.first().fill("no-such-parameter");
  await page.getByRole("status").filter({ hasText: "no-such-parameter" }).first().waitFor({
    timeout: 5000,
  });
  await ctx.scene("parameter-filter-none");
  await filter.first().fill("");
  await page.waitForTimeout(300);
  host.set({ manyParameters: false });
}

/** グルーのパネルで見本のグルーを選び、編集の欄 (強さ・軸のキー・軸を足す欄) を撮る (T0949)。選んだのを外して戻す。 */
async function glueScene(page, ctx) {
  const item = page.getByRole("button", { name: /^SleeveHandGlue\b/ }).first();
  await item.click();
  await page.waitForTimeout(400);
  const axes = page.getByRole("group", {
    name: text("editor", ctx.lang, "glues.axes.aria"),
    exact: true,
  });
  await scrollToElement(axes);
  await ctx.scene("glue-edit", { aria: false });
  await scrollToTop(page);
  await item.click();
  await page.waitForTimeout(300);
}

/**
 * ツリーのホバー (T1638): パーツの行に乗せた (ビューポートにパーツの中のメッシュの破線の輪郭) 場面と、パーツの行を畳んで
 * ビューポートのメッシュに乗せた (畳んだ祖先の行に地の重ねと左端の縦線) 場面。
 */
async function treeHoverScenes(page, ctx) {
  const head = page.locator('[role=treeitem][data-key="part:Head"]').first();
  await head.hover();
  await page.waitForTimeout(500);
  await ctx.scene("tree-hover", { aria: false });
  const twisty = head.locator("button.twisty");
  await twisty.click();
  await page.waitForTimeout(300);
  // 畳んだパーツの中のメッシュに乗るまで、ビューポートの上で場所を変えて乗せる
  const box = await page.locator(".viewport .surface").first().boundingBox();
  let found = false;
  if (box) {
    for (const y of [0.3, 0.4, 0.2, 0.5, 0.35, 0.25, 0.45]) {
      for (const x of [0.5, 0.45, 0.55, 0.4, 0.6]) {
        await page.mouse.move(box.x + box.width * x, box.y + box.height * y);
        await page.waitForTimeout(150);
        if ((await page.locator("[role=treeitem].hovered-within").count()) > 0) {
          found = true;
          break;
        }
      }
      if (found) break;
    }
  }
  if (!found) throw new Error("no mesh of the collapsed part under the pointer");
  await page.waitForTimeout(300);
  await ctx.scene("tree-hover-collapsed", { aria: false });
  await page.mouse.move(1, 300);
  await twisty.click();
  await page.waitForTimeout(300);
}

/**
 * ツリーの選んだ行に乗せた場面 (T0851): 控えめな文字の名前 (非表示・本体の要素) が、選んだ地と hover の重ねの上に乗る。
 * `key` の行 (`data-key`) を選んで乗せて撮り、ポインタを外す。選んだのはそのまま (後の場面が選び直す)。
 */
async function treeSelectedHoverScene(page, ctx, key, name) {
  // 左の列をパーツの木に (前の場面がデフォーマの木にしている)
  await page
    .getByRole("tab", { name: text("editor", ctx.lang, "tree.view.parts"), exact: true })
    .click();
  await page.waitForTimeout(200);
  const row = page.locator(`[role=treeitem][data-key="${key}"]`).first();
  await scrollToElement(row);
  await row.click();
  await page.waitForTimeout(400);
  await row.hover();
  await page.waitForTimeout(400);
  await ctx.scene(name, { aria: false });
  await page.mouse.move(1, 300);
  await page.waitForTimeout(200);
}

/**
 * アプリの定義。
 * - `wide`・`narrow`: 広い幅・狭い幅の窓の大きさ (CSS px)。狭い幅は docs/ui.md#そのほか の下限。
 * - `half` (省ける): 画面の半分に置いたときの窓の大きさ。ライトの英語で 1 組だけ撮る (`en-light-half`)。
 * - `xwide` (省ける): 1600px ほどの広い窓。ライトの英語とダークの日本語の 2 組を撮る (`en-light-xwide`・`ja-dark-xwide`)。
 * - `wasm`: `just wasm` の出力先。`package`: pnpm のパッケージ。
 * - `setup(page, ctx)`: 読み込む前の用意 (偽のホスト)。`load(page, ctx)`: 開いて、空の状態を撮り、モデルを読む。
 * - `extras(page, ctx)`: モデルを読んだ後の場面 (ダイアログなど)。
 */
export const APPS = {
  editor: {
    package: "@ugoe/editor",
    wasm: "apps/editor/src/lib/viewport/generated",
    wide: [1440, 900],
    narrow: [1024, 720],
    async setup(page, ctx) {
      const host = createEditorHost(ctx.sample);
      ctx.editorHost = host;
      await page.exposeFunction("__ugoeAuditIpc", ipcBinding(host));
      await page.addInitScript(editorPageInit, { osLocale: ctx.osLocale, home: ctx.home });
    },
    async load(page, ctx) {
      await page.goto(`${ctx.url}/`);
      const open = page.getByRole("button", {
        name: text("editor", ctx.lang, "fileMenu.open.label"),
        exact: true,
      });
      await open.first().waitFor({ timeout: 20000 });
      await page.waitForTimeout(800);
      await ctx.scene("empty");
      await page.evaluate((p) => {
        window.__uiAuditNextPath = p;
      }, ctx.sample.project);
      await open.first().click();
      await page.waitForFunction(
        () => document.querySelectorAll("[role=treeitem]").length > 0,
        null,
        { timeout: 20000 },
      );
      await page.waitForTimeout(2000);
    },
    async extras(page, ctx) {
      await settingsScene(page, ctx, text("editor", ctx.lang, "app.settings.label"));
      // ヘッダーの「File」のメニュー
      await page
        .getByRole("button", { name: text("editor", ctx.lang, "fileMenu.menu.label"), exact: true })
        .first()
        .click();
      await page.getByRole("menu").first().waitFor({ timeout: 5000 });
      await ctx.scene("file-menu", { aria: false });
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
      await page
        .getByRole("button", {
          name: text("editor", ctx.lang, "fileMenu.export.label"),
          exact: true,
        })
        .first()
        .click();
      await page.waitForTimeout(600);
      await ctx.scene("export");
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
      // 外での書き換え: 保存の前の確かめで見つかった (上書き・別名で保存を選べる) ダイアログ (T0953)
      ctx.editorHost.set({ externalChange: true });
      await page
        .getByRole("button", { name: text("editor", ctx.lang, "fileMenu.save.label"), exact: true })
        .first()
        .click();
      await page.getByRole("dialog").first().waitFor({ timeout: 5000 });
      await page.waitForTimeout(300);
      await ctx.scene("external-change");
      await page.keyboard.press("Escape");
      await page.getByRole("dialog").first().waitFor({ state: "hidden", timeout: 5000 });
      await page.waitForTimeout(300);
      ctx.editorHost.set({ externalChange: false });
      await treeHoverScenes(page, ctx);
      // 非表示の要素 (見本の Wave は不透明度 0) を選んだ行に乗せた
      await treeSelectedHoverScene(page, ctx, "part:Wave", "tree-hidden-hover");
      // パーツを選んだ状態 (インスペクタに中身が出る)
      await page.locator("[role=treeitem]").nth(1).click();
      await page.waitForTimeout(600);
      await ctx.scene("selected");
      // 右の列のタブ (物理・グルー・ライティング・スロット・ポーズ・検証・AI)
      for (const id of ["physics", "glue", "lighting", "slots", "poses", "verify", "ai"]) {
        await page
          .getByRole("tab", { name: text("editor", ctx.lang, `app.tabs.${id}.label`), exact: true })
          .click();
        await page.waitForTimeout(400);
        await ctx.scene(`tab-${id}`, { aria: false });
        if (id === "glue") await glueScene(page, ctx);
      }
      await page
        .getByRole("tab", {
          name: text("editor", ctx.lang, "app.tabs.inspector.label"),
          exact: true,
        })
        .click();
      // タイムラインのグラフ (カーブのある行とキーを選ぶ) と、モーションの「Settings…」の Popover (T0675)。
      // 幅 900px 未満の窓 (200% の拡大) ではタイムラインが畳まれているので、先に開く (T1637)
      const expandTimeline = page.getByRole("button", {
        name: text("editor", ctx.lang, "timeline.expand"),
        exact: true,
      });
      if ((await expandTimeline.count()) > 0) {
        await expandTimeline.first().click();
        await page.waitForTimeout(400);
      }
      await page.locator(".motion-editor .label.has-curve button.track").first().click();
      await page.locator(".motion-editor .lane.selected button.diamond").nth(1).click();
      await page.waitForTimeout(400);
      await ctx.scene("timeline-graph", { aria: false });
      await page
        .getByRole("button", {
          name: text("editor", ctx.lang, "timeline.motion.settings.label"),
          exact: true,
        })
        .click();
      await page.locator(".ugoe-popover").first().waitFor({ timeout: 5000 });
      await ctx.scene("motion-settings", { aria: false });
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
      // ビューポートの道具: ワープのギズモと、メッシュの編集 (見本の HeadWarp・Face。T0675)
      await page
        .getByRole("treeitem", { name: /^HeadWarp\b/ })
        .first()
        .click();
      await page.waitForTimeout(600);
      await ctx.scene("gizmo", { aria: false });
      // 回転のギズモとインスペクタの角度・拡大率の欄: キーの上 (軸の無い HairSway) と、キーの間 (HeadRotation。T1636)
      await page
        .getByRole("treeitem", { name: /^HairSway\b/ })
        .first()
        .click();
      await page.waitForTimeout(600);
      await ctx.scene("rotation", { aria: false });
      await page
        .getByRole("treeitem", { name: /^HeadRotation\b/ })
        .first()
        .click();
      await page.waitForTimeout(600);
      await ctx.scene("rotation-between", { aria: false });
      await page
        .getByRole("treeitem", { name: /^Face\b/ })
        .first()
        .click();
      await page
        .getByRole("button", {
          name: text("editor", ctx.lang, "viewport.editMesh.label"),
          exact: true,
        })
        .click();
      await page.waitForTimeout(600);
      await ctx.scene("mesh-edit", { aria: false });
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
      // 左の列のデフォーマ・タイムラインの表情
      await page
        .getByRole("tab", { name: text("editor", ctx.lang, "tree.view.deformers"), exact: true })
        .click();
      await page
        .getByRole("tab", { name: pattern(text("editor", ctx.lang, "timeline.tabs.expressions")) })
        .click();
      await page.waitForTimeout(400);
      await ctx.scene("deformers-expressions", { aria: false });
      await aiScenes(page, ctx);
      await dressUpScenes(page, ctx);
      await manyParametersScenes(page, ctx);
    },
  },
  viewer: {
    package: "@ugoe/viewer",
    wasm: "apps/viewer/src/lib/wasm/generated",
    wide: [1280, 800],
    narrow: [360, 740],
    // 右の列を 2 列に並べる幅 (1200px 以上。apps/viewer/README.md) で、ビューポートがさらに広がったとき
    xwide: [1600, 900],
    async setup() {},
    async load(page, ctx) {
      await page.goto(`${ctx.url}/`);
      await page.waitForFunction(
        () => ["empty", "unavailable"].includes(window.__UGOE_VIEWER__?.status),
        null,
        {
          timeout: 20000,
        },
      );
      await page.waitForTimeout(500);
      await ctx.scene("empty");
      // 空の状態にも「サンプルを開く」があるので、ツールバーのものを名前の全体で選ぶ
      await page
        .getByRole("button", {
          name: text("viewer", ctx.lang, "toolbar.samples.label"),
          exact: true,
        })
        .click();
      await page.getByRole("menu").first().waitFor({ timeout: 5000 });
      await ctx.scene("samples-menu", { aria: false });
      await page
        .getByRole("menuitem", {
          name: text("viewer", ctx.lang, "samples.avatarLit.label"),
          exact: true,
        })
        .click();
      await page.waitForFunction(() => window.__UGOE_VIEWER__?.status === "ready", null, {
        timeout: 30000,
      });
      // 動きを止めて、撮るたびに同じ絵にする
      await page.evaluate(() => {
        const p = window.__UGOE_VIEWER__;
        p.setAutoEyeBlink(false);
        p.setAutoIdle(false);
        p.stopMotions();
      });
      await page.waitForTimeout(1000);
    },
    async extras(page, ctx) {
      // 背景の色と「透明」の Popover (T0737)。キーボードで開く
      await page
        .getByRole("button", {
          name: text("viewer", ctx.lang, "toolbar.background.label"),
          exact: true,
        })
        .focus();
      await page.keyboard.press("Enter");
      await page.locator(".ugoe-popover").waitFor({ timeout: 5000 });
      await ctx.scene("background", { aria: false });
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);
      await settingsScene(page, ctx, text("viewer", ctx.lang, "toolbar.settings.aria"));
    },
  },
  studio: {
    package: "@ugoe/studio",
    wasm: "apps/studio/src/lib/wasm/generated",
    wide: [1280, 800],
    narrow: [360, 740],
    // 1920 の画面の半分に置いた操作ウィンドウ (列の並びが 3 列から 2 列 + 下に設定に変わる幅。docs/studio.md#操作ウィンドウ)
    half: [960, 800],
    // 中央の列 (1fr) だけが伸びる広い窓
    xwide: [1600, 900],
    async setup(page, ctx) {
      const fixture = await studioFixture(ctx.lang);
      ctx.fixture = fixture;
      await page.addInitScript(studioPageInit, { osLocale: ctx.osLocale, fixture });
    },
    async load(page, ctx) {
      await page.goto(`${ctx.url}/index.html`);
      await page.waitForSelector("header", { timeout: 20000 });
      await page.waitForTimeout(1500);
    },
    async extras(page, ctx) {
      await settingsScene(page, ctx, text("studio", ctx.lang, "toolbar.settings"));
      await page
        .getByRole("button", { name: text("studio", ctx.lang, "toolbar.profile"), exact: true })
        .click();
      await page.getByRole("menu").first().waitFor({ timeout: 5000 });
      await ctx.scene("profile-menu", { aria: false });
      await page.keyboard.press("Escape");
      await page.waitForTimeout(300);

      // 「入力」のパネル (カメラのフェイストラッキングとマイク。T0951)
      await scrollToSection(page, text("studio", ctx.lang, "inputs.heading"));
      await ctx.scene("inputs", { aria: false });
      await scrollToTop(page);

      // 出力ウィンドウの様子 (大きさ・枠・背景。T0818)
      await scrollToSection(page, text("studio", ctx.lang, "output.heading"));
      await ctx.scene("output", { aria: false });
      // 透過が画面で効かない (コンポジタが無い X11 など): 「背景」の欄の注意と「背景をクロマキーにする」のボタン (T0846・T0849)
      await page.evaluate(() => window.__uiAuditStudio.setOutput({ transparency: "unsupported" }));
      const chroma = page.getByRole("button", {
        name: text("studio", ctx.lang, "output.opaqueChroma"),
        exact: true,
      });
      await chroma.waitFor({ timeout: 5000 });
      await chroma.scrollIntoViewIfNeeded();
      await ctx.scene("opaque");
      await page.evaluate(() => window.__uiAuditStudio.setOutput({ transparency: "supported" }));
      await page.waitForTimeout(300);
      await scrollToTop(page);

      // 「いまの状態」のホットキーの印からホットキーのパネルへ送り、1 つ目を削除した後の「元に戻す」の知らせ (T0845)
      await page
        .getByRole("list", { name: text("studio", ctx.lang, "status.label"), exact: true })
        .locator('[data-status="hotkeys"]')
        .click();
      await page.waitForTimeout(800);
      await page
        .locator('[data-panel="hotkeys"]')
        .getByRole("button", { name: text("studio", ctx.lang, "hotkeys.remove"), exact: true })
        .first()
        .click();
      await page.waitForTimeout(300);
      await ctx.scene("hotkey-removed");
      await page
        .locator('[data-panel="hotkeys"]')
        .getByRole("button", { name: text("studio", ctx.lang, "hotkeys.undo"), exact: true })
        .click();
      await page.waitForTimeout(300);
      await scrollToTop(page);

      // 注意の要る状態 (承認を待つアプリ・登録できないホットキー・負荷の注意。T0818)
      await page.evaluate(({ messages, status }) => {
        window.__uiAuditStudio.setStatus(status);
        for (const m of messages) window.__uiAuditStudio.emit(m);
      }, attentionMessages(ctx.fixture));
      await page.waitForTimeout(600);
      await ctx.scene("attention");
      await scrollToSection(page, text("studio", ctx.lang, "hotkeys.heading"));
      await ctx.scene("attention-hotkeys", { aria: false });
      await scrollToTop(page);
      // 承認した後の「取り消す」の知らせ
      await page
        .getByRole("button", { name: text("studio", ctx.lang, "api.approve"), exact: true })
        .click();
      await page.waitForTimeout(300);
      await ctx.scene("approved", { aria: false });

      // 初めて起動した画面 (新しいプロファイル。モデルの無いシーンが 1 つ)
      await page.evaluate((messages) => {
        window.__uiAuditStudio.setStatus({});
        for (const m of messages) window.__uiAuditStudio.emit(m);
      }, firstRunMessages(ctx.fixture));
      await page.waitForTimeout(600);
      await ctx.scene("first-run");
    },
  },
};
