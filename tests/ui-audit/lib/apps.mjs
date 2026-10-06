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
 * 折りたためるまとまり (`Section`) の見出しのボタンを、送れる祖先 (右の列か、狭い幅では画面の全体) の頭まで送る。
 * ツールバーの同じ名前のボタン (「出力ウィンドウ」) を拾わないよう、`main` の中で探す。
 */
async function scrollToSection(page, name) {
  await page
    .locator("main")
    .getByRole("button", { name, exact: true })
    .first()
    .evaluate((el) => {
      // scrollIntoView は送らない祖先 (body) まで動かすので、送れる祖先だけを送る
      let box = el.parentElement;
      while (box !== null) {
        const o = getComputedStyle(box).overflowY;
        if (box.scrollHeight > box.clientHeight && (o === "auto" || o === "scroll")) break;
        box = box.parentElement;
      }
      if (box !== null)
        box.scrollTop += el.getBoundingClientRect().top - box.getBoundingClientRect().top;
    });
}

/** 送った所をすべて頭に戻す。 */
async function scrollToTop(page) {
  await page.evaluate(() => {
    for (const el of [document.scrollingElement, ...document.querySelectorAll("*")])
      if (el) el.scrollTop = 0;
  });
}

/**
 * アプリの定義。
 * - `wide`・`narrow`: 広い幅・狭い幅の窓の大きさ (CSS px)。狭い幅は docs/ui.md#そのほか の下限。
 * - `half` (省ける): 画面の半分に置いたときの窓の大きさ。ライトの英語で 1 組だけ撮る (`en-light-half`)。
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
      }
      await page
        .getByRole("tab", {
          name: text("editor", ctx.lang, "app.tabs.inspector.label"),
          exact: true,
        })
        .click();
      // 左の列のデフォーマ・タイムラインの表情
      await page
        .getByRole("tab", { name: text("editor", ctx.lang, "tree.view.deformers"), exact: true })
        .click();
      await page
        .getByRole("tab", { name: pattern(text("editor", ctx.lang, "timeline.tabs.expressions")) })
        .click();
      await page.waitForTimeout(400);
      await ctx.scene("deformers-expressions", { aria: false });
    },
  },
  viewer: {
    package: "@ugoe/viewer",
    wasm: "apps/viewer/src/lib/wasm/generated",
    wide: [1280, 800],
    narrow: [360, 740],
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

      // 出力ウィンドウの様子 (大きさ・枠・背景。T0818)
      await scrollToSection(page, text("studio", ctx.lang, "output.heading"));
      await ctx.scene("output", { aria: false });
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
