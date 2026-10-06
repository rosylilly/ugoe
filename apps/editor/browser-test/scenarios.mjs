// エディタのブラウザのテストのシナリオ (run.mjs が 1 つずつ新しいページで流す。README.md)。
//
// 画面は WebDriver の代わりに Playwright で操作し、ボタンの名前はエディタのカタログ (locales/en.json) から引く。
// 本物の WebPuppet は、ページが wasm-bindgen のグルー (エディタと同じチャンクの URL) を読んで `WebPuppet` を包んで見る
// (`hookWasm`): 描いている WebPuppet (`render` を最後に呼んだもの)・生きている WebPuppet・作成の遅れ (`createDelayMs`)。
import { join } from "node:path";
import { text } from "../../../tests/ui-audit/lib/apps.mjs";

/** 文言 (英語)。 */
const en = (key) => text("editor", "en", key);

/** 待つ上限 (ミリ秒)。SwiftShader の WebPuppet の作り直しは数秒かかることがある。 */
const WAIT_MS = 60_000;
/**
 * 作り直しの WebPuppet の作成を遅らせる時間 (ミリ秒)。CI の SwiftShader のように作成が遅いときを再現する
 * (新しい canvas はあるが WebPuppet はまだ無い間に確かめてしまわないことを確かめる。T0960 と同じ)。
 */
const CREATE_DELAY_MS = 500;
/**
 * 作り直しの途中に読み込むシナリオで、WebPuppet の作成を遅らせる時間 (ミリ秒)。その間 (古い WebPuppet を捨て、新しいものが
 * まだ無い) にファイルを選んで読み終えるのに足りる長さ。
 */
const RECREATION_DELAY_MS = 5_000;

/**
 * ページの初期化 (page.addInitScript)。WebGL2 のコンテキストを canvas ごとに覚える (`loseContext` で、ビューポートの canvas の
 * コンテキストを失わせるため。新しいコンテキストを作らずに引けるように)。
 */
export function PAGE_INIT() {
  const contexts = new WeakMap();
  const getContext = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, ...rest) {
    const context = getContext.call(this, type, ...rest);
    if (type === "webgl2" && context) contexts.set(this, context);
    return context;
  };
  window.__editorBrowser = {
    contexts,
    /** 最後に `render` を呼んだ WebPuppet (ビューポートが描いているもの)。 */
    current: null,
    /** 作って、まだ `free` していない WebPuppet (`hookWasm` の後に作ったもの)。 */
    live: new Set(),
    createDelayMs: 0,
    /** 最後に失わせたコンテキストの `WEBGL_lose_context`。 */
    lost: null,
    /** 失わせたときに描いていた WebPuppet。 */
    before: null,
  };
}

// --- ページの操作 ---

/** エディタを開き、見本のプロジェクト (着せ替えの本体) を開く。 */
async function openSample(page, ctx) {
  await page.goto(`${ctx.url}/`);
  const open = page.getByRole("button", { name: en("fileMenu.open.label"), exact: true }).first();
  await open.waitFor({ timeout: WAIT_MS });
  await page.evaluate((p) => {
    window.__uiAuditNextPath = p;
  }, ctx.sample.project);
  await open.click();
  await page.waitForFunction(() => document.querySelectorAll("[role=treeitem]").length > 0, null, {
    timeout: WAIT_MS,
  });
  await hookWasm(page, ctx);
}

/**
 * エディタと同じ wasm-bindgen のグルーを読み (同じ URL のモジュールは同じもの)、`WebPuppet` を包む。
 * `create` は作成の遅れと生きている WebPuppet を、`render` は描いている WebPuppet を、`free` は捨てたことを覚える。
 */
async function hookWasm(page, ctx) {
  await page.evaluate(async (url) => {
    const state = window.__editorBrowser;
    const { WebPuppet } = await import(url);
    const create = WebPuppet.create;
    WebPuppet.create = async function (...args) {
      const puppet = await create.apply(this, args);
      if (state.createDelayMs > 0) {
        await new Promise((resolve) => setTimeout(resolve, state.createDelayMs));
      }
      state.live.add(puppet);
      return puppet;
    };
    const render = WebPuppet.prototype.render;
    WebPuppet.prototype.render = function (...args) {
      state.current = this;
      return render.apply(this, args);
    };
    const free = WebPuppet.prototype.free;
    WebPuppet.prototype.free = function () {
      state.live.delete(this);
      if (state.current === this) state.current = null;
      return free.call(this);
    };
  }, ctx.wasm);
}

/** プレビューに入り、ビューポートがモデルを描くのを待つ。 */
async function enterPreview(page) {
  await page
    .getByRole("button", { name: en("app.preview.label"), exact: true })
    .first()
    .click();
  await dressUpSection(page).waitFor({ timeout: WAIT_MS });
  await untilDrawing(page, "the preview to draw");
}

function dressUpSection(page) {
  return page.getByRole("region", { name: en("preview.dressUp.title"), exact: true });
}

/** 描いている WebPuppet があり、GPU を失っておらず、ビューポートが待っていない (GPU の復元・読み込み) 。 */
async function untilDrawing(page, what) {
  await until(
    page,
    () => {
      const p = window.__editorBrowser.current;
      return p !== null && !p.is_lost() && p.has_model();
    },
    what,
  );
  await gpuWaiting(page).waitFor({ state: "hidden", timeout: WAIT_MS });
}

/** ビューポートの「GPU の復元を待っています」の文 (プレビューのパネルにも同じ文が出るので、ビューポートの中で探す)。 */
function gpuWaiting(page) {
  return page.getByRole("main").getByText(en("viewport.gpuWaiting"), { exact: true });
}

/** ページの中の条件 `fn` (引数 `arg`) が真になるのを待つ。 */
async function until(page, fn, what, arg) {
  try {
    await page.waitForFunction(fn, arg, { timeout: WAIT_MS, polling: 100 });
  } catch (e) {
    throw new Error(`timed out waiting for ${what}: ${e instanceof Error ? e.message : e}`);
  }
}

/** 「Load .ugoemod…」で見本のモジュールのファイル `file` を読み込み、読み終える (ボタンが押せるようになる) のを待つ。 */
async function loadModule(page, ctx, file) {
  await page.evaluate(
    (p) => {
      window.__uiAuditNextPath = p;
    },
    join(ctx.modules, file),
  );
  const button = dressUpSection(page).getByRole("button", {
    name: en("preview.dressUp.load.label"),
    exact: true,
  });
  await button.click();
  // 読んでいる間 (`busy`) は押せない
  await page.waitForTimeout(100);
  await button.waitFor({ state: "visible", timeout: WAIT_MS });
  await until(
    page,
    (name) =>
      [...document.querySelectorAll("section[aria-label] button")].some(
        (b) => b.textContent?.trim() === name && !b.disabled,
      ),
    "the dress-up panel to finish loading",
    en("preview.dressUp.load.label"),
  );
}

/** 描いている WebPuppet のコンテキストを `WEBGL_lose_context` で失わせ、ビューポートが復元を待ち始めるのを待つ。 */
async function loseContext(page) {
  await page.evaluate(() => {
    const state = window.__editorBrowser;
    const canvas = [...document.querySelectorAll("canvas")].find((c) => {
      const gl = state.contexts.get(c);
      return gl !== undefined && !gl.isContextLost();
    });
    if (canvas === undefined) throw new Error("no canvas with a live WebGL2 context");
    const ext = state.contexts.get(canvas).getExtension("WEBGL_lose_context");
    state.lost = ext;
    state.before = state.current;
    ext.loseContext();
  });
  await gpuWaiting(page).waitFor({ timeout: WAIT_MS });
  await until(
    page,
    () => window.__editorBrowser.before?.is_lost() === true,
    "the WebPuppet to know it lost the GPU",
  );
}

/**
 * 失わせたコンテキストを戻し、作り直しが済む (新しい WebPuppet が描き、ビューポートが待っていない) のを待つ。
 * 作り直しの途中 (新しい canvas はあるが WebPuppet はまだ無い) では満たさない。
 */
async function restoreAndRecreate(page) {
  await page.evaluate(() => window.__editorBrowser.lost.restoreContext());
  await until(
    page,
    () => {
      const state = window.__editorBrowser;
      return state.current !== null && state.current !== state.before;
    },
    "a new WebPuppet to draw after the recreation",
  );
  await untilDrawing(page, "the recreated WebPuppet to draw");
}

/**
 * コンテキストを失わせて戻し、作り直しの途中 (古い WebPuppet を `free` し、新しいものをまだ作り終えていない。ビューポートの
 * `puppet` が `null`) になるのを待つ。作成の遅れ (`createDelayMs`) は呼ぶ前に長くしておく。
 */
async function loseAndStartRecreating(page) {
  await loseContext(page);
  await page.evaluate(() => window.__editorBrowser.lost.restoreContext());
  await until(
    page,
    () => {
      const state = window.__editorBrowser;
      return state.current === null && !state.live.has(state.before) && state.live.size === 0;
    },
    "the old WebPuppet to be freed before the new one is made",
  );
}

/** まだ作り直しの途中 (WebPuppet が無い)。でなければ `message` で落とす。 */
async function expectRecreating(page, message) {
  check(
    await page.evaluate(
      () => window.__editorBrowser.current === null && window.__editorBrowser.live.size === 0,
    ),
    message,
  );
}

/**
 * 着せ替えの一覧の行 (名前・チェック・チェックボックスの説明 (待っている付け外しの文。T1666))、スロットの選択 (スロット →
 * 選んでいるもの) と、パネルの中の失敗の文 (`role=alert`)。
 */
function panelState(page) {
  return dressUpSection(page).evaluate((section) => ({
    rows: [...section.querySelectorAll("ul li")].map((li) => {
      const box = li.querySelector("[role=checkbox]");
      const describedBy = box?.getAttribute("aria-describedby");
      return {
        name: li.querySelector("label")?.textContent?.trim() ?? "",
        checked: box?.getAttribute("aria-checked") === "true",
        note: describedBy
          ? (document.getElementById(describedBy)?.textContent?.trim() ?? "")
          : null,
      };
    }),
    slots: Object.fromEntries(
      [...section.querySelectorAll(".slot")].map((row) => [
        row.querySelector(".slot-id")?.textContent?.trim() ?? "",
        row.querySelector(".ugoe-select__value")?.textContent?.trim() ?? "",
      ]),
    ),
    alerts: [...section.querySelectorAll("[role=alert]")].map((a) => a.textContent?.trim() ?? ""),
  }));
}

/** 描いている WebPuppet に付いているモジュール (名前の順)。 */
function attachedModules(page) {
  return page.evaluate(() => [...window.__editorBrowser.current.attached_modules()].sort());
}

/**
 * 描いている WebPuppet の読み戻し (`read_pixels`): 背景 (左上の画素の色) と違う画素の割合と、それがある一番上の行。
 * 見本の猫耳は頭の上に出るので、付いていれば一番上の行が上がる。
 */
function drawn(page) {
  return page.evaluate(async () => {
    const puppet = window.__editorBrowser.current;
    const width = puppet.width();
    const pixels = await puppet.read_pixels();
    const differs = (i) =>
      Math.abs(pixels[i] - pixels[0]) +
        Math.abs(pixels[i + 1] - pixels[1]) +
        Math.abs(pixels[i + 2] - pixels[2]) +
        Math.abs(pixels[i + 3] - pixels[3]) >
      24;
    let count = 0;
    let top = -1;
    for (let i = 0; i < pixels.length; i += 4) {
      if (!differs(i)) continue;
      count++;
      if (top < 0) top = Math.floor(i / 4 / width);
    }
    return { fraction: count / (pixels.length / 4), top };
  });
}

function check(ok, message) {
  if (!ok) throw new Error(message);
}

/** パネルに失敗の文が無い。 */
function expectNoAlerts(state, when) {
  check(state.alerts.length === 0, `${when}: the dress-up panel says ${state.alerts.join(" / ")}`);
}

/** 一覧の行のチェックが `expected` (名前 → 付いているか) のとおり。 */
function expectRows(state, expected, when) {
  for (const [name, checked] of Object.entries(expected)) {
    const row = state.rows.find((r) => r.name === name);
    check(row !== undefined, `${when}: no row for ${name} (${JSON.stringify(state.rows)})`);
    check(row.checked === checked, `${when}: ${name} is ${row.checked ? "" : "not "}ticked`);
  }
}

async function expectLivePuppets(page, when) {
  const live = await page.evaluate(() => window.__editorBrowser.live.size);
  // `hookWasm` の前に作った初めの WebPuppet は数えないので、作り直した 1 つだけが生きている
  check(live === 1, `${when}: ${live} live WebPuppets`);
}

// --- シナリオ ---

export const SCENARIOS = {
  /**
   * プレビューの着せ替えでモジュールを読み込む途中に GPU を失った (WebGL2 のコンテキストの喪失。`load_module` が
   * `device_lost`): 行に失敗の文を出さず、作り直した WebPuppet にそのモジュールを読み込んで付けて描く (T1644・T1656)。
   */
  async "dress-up-loss-during-load"(page, ctx) {
    await openSample(page, ctx);
    await enterPreview(page);
    const base = await drawn(page);
    check(base.fraction > 0.01, `the preview is empty (${(base.fraction * 100).toFixed(2)}%)`);
    await page.evaluate((ms) => {
      window.__editorBrowser.createDelayMs = ms;
    }, CREATE_DELAY_MS);

    await loseContext(page);
    await loadModule(page, ctx, "cat-ears.ugoemod");
    const whileLost = await panelState(page);
    expectNoAlerts(whileLost, "while the context is lost");
    check(
      await page.evaluate(() => window.__editorBrowser.current === window.__editorBrowser.before),
      "the WebPuppet was recreated before the context was restored",
    );

    await restoreAndRecreate(page);
    // 作り直した WebPuppet にモジュールを読み込んで付ける (`DressUp.loaded`)。一覧の行は WebPuppet の状態を写す
    await until(
      page,
      () => window.__editorBrowser.current.attached_modules().includes("CatEars"),
      "CatEars to be put on the recreated WebPuppet",
    );
    const after = await panelState(page);
    expectNoAlerts(after, "after the recreation");
    expectRows(after, { CatEars: true }, "after the recreation");
    const attached = await attachedModules(page);
    check(attached.join() === "CatEars", `attached modules: ${attached.join(", ")}`);
    const shot = await drawn(page);
    check(shot.fraction > 0.01, `the recreated preview is empty (${shot.fraction})`);
    check(
      shot.top >= 0 && shot.top < base.top - 5,
      `the cat ears are not drawn: the top row is ${shot.top} (${base.top} without them)`,
    );
    await expectLivePuppets(page, "after the recreation");
  },

  /**
   * 作り直しの途中 (古い WebPuppet を捨て、新しいものを作っている間。ビューポートの `puppet` が `null`) に読み込んだモジュールも
   * 取っておき、作り直した WebPuppet に読み込んで付けて描く (T1658)。作成を長く遅らせて、その間に読み込む。
   */
  async "dress-up-load-during-recreation"(page, ctx) {
    await openSample(page, ctx);
    await enterPreview(page);
    const base = await drawn(page);
    check(base.fraction > 0.01, `the preview is empty (${(base.fraction * 100).toFixed(2)}%)`);
    await page.evaluate((ms) => {
      window.__editorBrowser.createDelayMs = ms;
    }, RECREATION_DELAY_MS);

    await loseAndStartRecreating(page);
    await loadModule(page, ctx, "cat-ears.ugoemod");
    await expectRecreating(page, "the new WebPuppet was made before the module was loaded");
    expectNoAlerts(await panelState(page), "while the WebPuppet is being made again");

    await untilDrawing(page, "the recreated WebPuppet to draw");
    await until(
      page,
      () => window.__editorBrowser.current.attached_modules().includes("CatEars"),
      "CatEars to be put on the recreated WebPuppet",
    );
    const after = await panelState(page);
    expectNoAlerts(after, "after the recreation");
    expectRows(after, { CatEars: true }, "after the recreation");
    const attached = await attachedModules(page);
    check(attached.join() === "CatEars", `attached modules: ${attached.join(", ")}`);
    const shot = await drawn(page);
    check(shot.fraction > 0.01, `the recreated preview is empty (${shot.fraction})`);
    check(
      shot.top >= 0 && shot.top < base.top - 5,
      `the cat ears are not drawn: the top row is ${shot.top} (${base.top} without them)`,
    );
    await expectLivePuppets(page, "after the recreation");
  },

  /**
   * GPU を失っている間の付け外し (チェックで外す・スロットの選択で差し替える) は、付けたい組だけを変え、作り直した WebPuppet に
   * その組で付く (T1644・T1656)。
   */
  async "dress-up-change-while-lost"(page, ctx) {
    const withEars = await putOnThree(page, ctx);
    await page.evaluate((ms) => {
      window.__editorBrowser.createDelayMs = ms;
    }, CREATE_DELAY_MS);

    await loseContext(page);
    await takeOffEarsAndSwapHair(page);
    expectNoAlerts(await panelState(page), "after changing the modules while the context is lost");
    await expectWaitingChanges(page, "while the context is lost");

    await restoreAndRecreate(page);
    await expectOnlyLongHair(page, withEars);
  },

  /**
   * 作り直しの途中 (古い WebPuppet を捨て、新しいものを作っている間。ビューポートの `puppet` が `null`) の付け外しも、付けたい組
   * だけを変え、作り直した WebPuppet にその組で付く (T1659)。作成を長く遅らせて、その間に付け外す。
   */
  async "dress-up-change-during-recreation"(page, ctx) {
    const withEars = await putOnThree(page, ctx);
    await page.evaluate((ms) => {
      window.__editorBrowser.createDelayMs = ms;
    }, RECREATION_DELAY_MS);

    await loseAndStartRecreating(page);
    await takeOffEarsAndSwapHair(page);
    await expectRecreating(page, "the new WebPuppet was made before the modules were changed");
    expectNoAlerts(
      await panelState(page),
      "after changing the modules while there is no WebPuppet",
    );
    await expectWaitingChanges(page, "while there is no WebPuppet");
    await expectRecreating(page, "the new WebPuppet was made before the rows were read");

    await untilDrawing(page, "the recreated WebPuppet to draw");
    await expectOnlyLongHair(page, withEars);
  },
};

/**
 * 見本を開いてプレビューに入り、猫耳・長い髪・ツインテールを読み込む (猫耳とツインテールが付く)。猫耳を付けて描いた
 * 読み戻しを返す。
 */
async function putOnThree(page, ctx) {
  await openSample(page, ctx);
  await enterPreview(page);
  for (const file of ["cat-ears.ugoemod", "long-hair.ugoemod", "twin-tails.ugoemod"]) {
    await loadModule(page, ctx, file);
  }
  // 同じスロット (Hair) は後から読み込んだもの (TwinTails) に差し替わる
  const attached = await attachedModules(page);
  check(attached.join() === "CatEars,TwinTails", `attached before the loss: ${attached.join()}`);
  const before = await panelState(page);
  expectNoAlerts(before, "before the loss");
  expectRows(before, { CatEars: true, LongHair: false, TwinTails: true }, "before the loss");
  return drawn(page);
}

/** 猫耳のチェックを外し、スロット `Hair` の選択で長い髪に替える。 */
async function takeOffEarsAndSwapHair(page) {
  const section = dressUpSection(page);
  await section.getByRole("checkbox", { name: "CatEars", exact: true }).click();
  await section
    .locator(`[aria-label="${en("preview.dressUp.slots.aria").replace("{{slot}}", "Hair")}"]`)
    .click();
  await page.getByRole("option", { name: "LongHair", exact: true }).click();
  await page.waitForTimeout(200);
}

/**
 * 作り直す前から、一覧の行のチェックとスロットの選択が付け外し (猫耳を外し、長い髪に替えた) を出し、まだ届いていない行に
 * 待っている文を添える (T1666)。
 */
async function expectWaitingChanges(page, when) {
  const state = await panelState(page);
  expectRows(state, { CatEars: false, LongHair: true, TwinTails: false }, when);
  const notes = Object.fromEntries(state.rows.map((r) => [r.name, r.note]));
  const on = en("preview.dressUp.modules.waiting.on");
  const off = en("preview.dressUp.modules.waiting.off");
  check(
    notes.CatEars === off && notes.LongHair === on && notes.TwinTails === off,
    `${when}: the rows do not say what waits for the preview (${JSON.stringify(notes)})`,
  );
  check(
    state.slots.Hair === "LongHair",
    `${when}: the Hair slot shows ${state.slots.Hair} (${JSON.stringify(state.slots)})`,
  );
}

/** 作り直した WebPuppet に長い髪だけが付き、一覧の行もそのとおりで、猫耳が描かれない (`withEars` は猫耳を付けた読み戻し)。 */
async function expectOnlyLongHair(page, withEars) {
  await until(
    page,
    () => {
      const names = [...window.__editorBrowser.current.attached_modules()];
      return names.length === 1 && names[0] === "LongHair";
    },
    "only LongHair to be put on the recreated WebPuppet",
  );
  const after = await panelState(page);
  expectNoAlerts(after, "after the recreation");
  expectRows(after, { CatEars: false, LongHair: true, TwinTails: false }, "after the recreation");
  check(
    after.rows.every((r) => r.note === null),
    `after the recreation: a row still waits (${JSON.stringify(after.rows)})`,
  );
  const attached = await attachedModules(page);
  check(attached.join() === "LongHair", `attached after the recreation: ${attached.join()}`);
  const shot = await drawn(page);
  check(shot.fraction > 0.01, `the recreated preview is empty (${shot.fraction})`);
  check(
    shot.top > withEars.top + 5,
    `the cat ears are still drawn: the top row is ${shot.top} (${withEars.top} with them)`,
  );
  await expectLivePuppets(page, "after the recreation");
}
