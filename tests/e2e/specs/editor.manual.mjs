// エディタ (apps/editor) のマニュアルの画像 (docs/manual/images) を撮る (`just e2e --manual --only editor`)。
// マニュアルのエディタの章は英語の画面の名前で書くので、英語の画面で撮る。流し方は tests/e2e/README.md#マニュアルの画像。
//
// 見本の着せ替え (「Open Sample…」) と `assets/fixtures/character.psd` から撮る。チュートリアルの途中の姿 (メッシュ・キーの格子・
// ミラー・深度) は、docs/manual/tutorial.md の手順を画面で操作して作る。AI の提案は偽の AI のプロバイダ (lib/fake-ai.mjs) で、
// MCP は偽のエージェント (lib/fake-agent.mjs) で撮る。外部のサービスには何も送らない (送る前の確認は「Cancel」で閉じる)。

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { after, before, describe } from "node:test";
import { connectAgent } from "../lib/fake-agent.mjs";
import { LABELS, startFakeAi } from "../lib/fake-ai.mjs";
import { dialogAnswers, invoke, keyringSkip, launch, root, step, work } from "../lib/harness.mjs";
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

/**
 * ビューポートの「Edit mesh」を押す。ページの中で押す (WebDriver のクリックだとポインタがその場に残り、同じ所に出る無効の
 * 「Add midpoint」の Tooltip が開いたまま写る)。
 */
async function editMesh(session) {
  await waitFor(
    () =>
      session.execute(() => {
        const button = [...document.querySelectorAll(".viewport button")].find(
          (b) => !b.disabled && b.textContent.trim() === "Edit mesh",
        );
        button?.click();
        return button !== undefined;
      }),
    { message: "the Edit mesh button" },
  );
  await session.waitFindText(".viewport button", "Done");
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

/**
 * `scope` (CSS のセレクタか要素) の中で、見えるラベルが `label` の欄 (入力欄・選択欄) を返す。`<label for>` と、
 * `aria-labelledby` で結んだ `<span id>` (NumberField・Select) の両方を探す。
 */
function fieldIn(session, scope, label) {
  return waitFor(
    () =>
      session.execute(
        (scope, label) => {
          const roots = typeof scope === "string" ? document.querySelectorAll(scope) : [scope];
          for (const root of roots) {
            for (const l of root.querySelectorAll("label, span[id]")) {
              if (l.textContent.replace(/\s+/g, " ").trim() !== label) continue;
              if (l.control) return l.control;
              const c = l.id ? root.querySelector(`[aria-labelledby~="${l.id}"]:not(label)`) : null;
              if (c) return c;
            }
          }
          return null;
        },
        scope,
        label,
      ),
    { message: `the field ${label}` },
  );
}

/** 欄に `text` を打って (前の中身は置き換える)、フォーカスを外して確定する。Enter は押さない (フォームを送らないように)。 */
async function fill(session, field, text) {
  await session.execute((el) => {
    el.scrollIntoView({ block: "center" });
    el.focus();
    el.select?.();
  }, field);
  await field.type(text === "" ? Keys.Backspace : text);
  await session.execute((el) => el.blur(), field);
}

/**
 * 要素を押す。WebDriver のクリックが「element click intercepted」(Tooltip など、ほかの要素が真ん中に重なっている) になったら、
 * ページの中で `click()` を呼ぶ。
 */
async function press(session, el) {
  try {
    await el.click();
  } catch (e) {
    if (!/intercepted/.test(e.message)) throw e;
    await session.execute((el) => el.click(), el);
  }
}

/**
 * 選択欄 (`@ugoe/ui` の Select) を開き、文字が `pattern` に合う選択肢を選んで、一覧が閉じるまで待つ (閉じかけの一覧が
 * 次の欄に重なって押せないことがある)。
 */
async function choose(session, field, pattern) {
  const options = () => session.findAll("[role=option]");
  await waitFor(async () => (await options()).length === 0, { message: "the open list to close" });
  await field.click();
  await session.clickWhen(
    () =>
      session.execute((pattern) => {
        const re = new RegExp(pattern);
        return (
          [...document.querySelectorAll("[role=option]")].find((o) =>
            re.test(o.textContent.replace(/\s+/g, " ").trim()),
          ) ?? null
        );
      }, pattern),
    { message: `the option ${pattern}` },
  );
  await waitFor(async () => (await options()).length === 0, { message: "the list to close" });
}

/** `scope` の中の、文字が `text` のリンクのボタン (`.ed-link`)。 */
function linkIn(session, scope, text) {
  return session.execute(
    (scope, text) =>
      [...document.querySelectorAll(`${scope} button.ed-link`)].find(
        (b) => b.textContent.trim() === text,
      ) ?? null,
    scope,
    text,
  );
}

/** 右の列の節 (見出し `h3` が `title` のもの)。 */
function sectionOf(session, title) {
  return waitFor(
    () =>
      session.execute(
        (title) =>
          [...document.querySelectorAll("aside.right section")].find(
            (el) => el.querySelector("h3")?.textContent.trim() === title,
          ) ?? null,
        title,
      ),
    { message: `the ${title} section` },
  );
}

/** パラメータの値の欄に `value` を入れる (スライダーを動かすのと同じ)。 */
async function setParameter(session, name, value) {
  const field = await waitFor(
    () =>
      session.execute(
        (name) =>
          [...document.querySelectorAll(".pane.parameters input.value")].find(
            (el) => el.getAttribute("aria-label") === `${name} value`,
          ) ?? null,
        name,
      ),
    { message: `the ${name} field` },
  );
  await field.replace(String(value));
}

/** パラメータ `name` の行の「◆」を押し、今の値に選んだ要素のキーを足す (もうキーなら何もしない)。 */
async function addKey(session, name) {
  await session.clickWhen(
    () =>
      session.execute((name) => {
        for (const li of document.querySelectorAll(".pane.parameters li")) {
          if (li.querySelector("button.name")?.textContent.trim() !== name) continue;
          const key = [...li.querySelectorAll("button")].find((b) =>
            /^Add a key at /.test(b.getAttribute("aria-label") ?? ""),
          );
          return key ?? null;
        }
        return null;
      }, name),
    { message: `the key button of ${name}` },
  );
  await waitFor(
    () =>
      session.execute((name) => {
        for (const li of document.querySelectorAll(".pane.parameters li")) {
          if (li.querySelector("button.name")?.textContent.trim() !== name) continue;
          return [...li.querySelectorAll("button")].some((b) =>
            /^Remove the key /.test(b.getAttribute("aria-label") ?? ""),
          );
        }
        return false;
      }, name),
    { message: `a key of ${name}` },
  );
}

/** インスペクタの「Keyforms」で、軸 `axis` のキー `key` を押してそのセルへ移る。 */
async function goToKey(session, axis, key) {
  await session.clickWhen(
    () =>
      session.execute(
        (axis, key) => {
          for (const li of document.querySelectorAll("aside.right ul.axes > li")) {
            if (li.querySelector(".axis")?.textContent.trim() !== axis) continue;
            return (
              [...li.querySelectorAll("button.go")].find(
                (b) => b.getAttribute("aria-label") === `Go to key ${key}`,
              ) ?? null
            );
          }
          return null;
        },
        axis,
        key,
      ),
    { message: `the key ${key} of ${axis}` },
  );
  await new Promise((r) => setTimeout(r, 300));
}

/** インスペクタの「Keyforms」の格子の、行のキー `row`・列のキー `col` のセル。 */
function gridCell(session, row, col) {
  return waitFor(
    () =>
      session.execute(
        (row, col) => {
          const table = document.querySelector("aside.right table.grid");
          if (!table) return null;
          const cols = [...table.querySelectorAll("thead th")].map((th) => th.textContent.trim());
          const index = cols.indexOf(String(col));
          for (const tr of table.querySelectorAll("tbody tr")) {
            if (tr.querySelector("th")?.textContent.trim() !== String(row)) continue;
            return tr.children[index]?.querySelector("button.cell") ?? null;
          }
          return null;
        },
        row,
        col,
      ),
    { message: `the cell ${row}, ${col}` },
  );
}

/** ワープのギズモの制御点の中心 (表示域の CSS px。左上から行ごとの順)。 */
function warpPoints(session) {
  return waitFor(
    async () => {
      const points = await session.execute(() =>
        [...document.querySelectorAll(".gizmo .points rect.hit")].map((el) => {
          const r = el.getBoundingClientRect();
          return [r.left + r.width / 2, r.top + r.height / 2];
        }),
      );
      return points.length > 0 ? points : null;
    },
    { message: "the warp points" },
  );
}

/** 要素の中心 (表示域の CSS px)。 */
function centerOf(session, el) {
  return session.execute((el) => {
    const r = el.getBoundingClientRect();
    return [r.left + r.width / 2, r.top + r.height / 2];
  }, el);
}

/** 点 `from` を `[dx, dy]` だけドラッグする (途中の点を通って、離したときに確定する)。 */
function dragBy(session, from, [dx, dy], options) {
  return session.pointer(
    [from, [from[0] + dx / 4, from[1] + dy / 4], [from[0] + dx, from[1] + dy]],
    options,
  );
}

/**
 * ポインタの Tooltip とホバーの輪郭が出ないように、フォーカスを外してポインタをヘッダーの空いた所へ動かし、少し待つ。
 */
async function settle(session, ms = 600) {
  await session.execute(() => document.activeElement?.blur?.());
  await session.execute(() => {
    document.scrollingElement.scrollTop = 0;
  });
  await session.moveTo([700, 18]);
  await new Promise((r) => setTimeout(r, ms));
}

/** AI タブの「Use …」のチェックボックス (押せるようになるまで待つ)。 */
function useAiBox(session) {
  return waitFor(
    () =>
      session.execute(() => {
        const label = [...document.querySelectorAll("aside.right .ugoe-checkbox__label")].find(
          (l) => /^Use /.test(l.textContent.trim()),
        );
        const box = label && document.getElementById(label.htmlFor);
        return box && !box.disabled ? box : null;
      }),
    { message: "the Use AI checkbox" },
  );
}

/** AI タブの「Use …」を `on` にする。 */
async function setUseAi(session, on) {
  const box = await useAiBox(session);
  if (((await box.attribute("aria-checked")) === "true") !== on) await box.click();
  await waitFor(async () => ((await box.attribute("aria-checked")) === "true") === on, {
    message: `the Use AI checkbox to be ${on ? "checked" : "unchecked"}`,
  });
}

/** AI タブの注文の欄。 */
function instructionsField(session) {
  return waitFor(() => session.execute(() => document.querySelector("aside.right textarea")), {
    message: "the instructions field",
  });
}

/** 設定の「AI provider」を開く。 */
async function openAiSettings(session) {
  await clickButton(session, "^Settings$", "header");
  await session.waitFind("[role=dialog]");
  await clickButton(session, "^AI provider$", "[role=dialog]");
}

/** 開いているダイアログを Esc で閉じ、閉じるまで待つ。 */
async function closeDialog(session) {
  await session.keys(Keys.Escape);
  await waitFor(async () => (await session.findAll("[role=dialog]")).length === 0, {
    message: "the dialog to close",
  });
  // 「Settings」の Tooltip が右の列のタブに重ならないように、ポインタを離す
  await settle(session, 300);
}

/** 提案のカード (`#<id>`)。 */
function proposalCard(session, id) {
  return waitFor(
    () =>
      session.execute(
        (id) =>
          [...document.querySelectorAll("article.proposal")].find(
            (a) => a.querySelector("header .index")?.textContent.trim() === `#${id}`,
          ) ?? null,
        id,
      ),
    { timeout: 60_000, message: `the proposal #${id}` },
  );
}

/** プレビュー中の提案のスライダー (名前が `pattern` に合うもの) を最大にする (つまみにフォーカスして End)。 */
async function maxPreviewSlider(session, pattern) {
  const thumb = await waitFor(
    () =>
      session.execute((pattern) => {
        const re = new RegExp(pattern);
        for (const slider of document.querySelectorAll(
          "article.proposal.previewing .ugoe-slider",
        )) {
          const label = slider.querySelector(".ugoe-slider__label")?.textContent.trim() ?? "";
          if (re.test(label)) return slider.querySelector("[role=slider]");
        }
        return null;
      }, pattern),
    { message: `the preview slider ${pattern}` },
  );
  await session.execute((el) => el.focus(), thumb);
  await session.keys(Keys.End);
  await waitFor(
    async () =>
      (await thumb.attribute("aria-valuenow")) === (await thumb.attribute("aria-valuemax")),
    { message: `the preview slider ${pattern} to be at its maximum` },
  );
}

/** 右の列の中の `el` の下端が、送れる祖先の下端 (の少し上) に来るまで送る。 */
function scrollInRightToBottom(session, el) {
  return session.execute((el) => {
    let box = el.parentElement;
    while (box !== null) {
      const o = getComputedStyle(box).overflowY;
      if (box.scrollHeight > box.clientHeight && (o === "auto" || o === "scroll")) break;
      box = box.parentElement;
    }
    if (box === null) return;
    box.scrollTop += el.getBoundingClientRect().bottom - box.getBoundingClientRect().bottom + 8;
  }, el);
}

/**
 * AI タブを広く見せる: 右の列の区切りを 5 段 (80px) 左へ動かし、右の列の上下の区切りを下の端 (インスペクタの割合の最大) へ動かす
 * (どちらも区切りにフォーカスしてキーで。覚えた大きさは、この後の手順にも残る)。
 */
async function widenRight(session) {
  const separator = (css) =>
    session
      .execute((css) => document.querySelector(css), css)
      .then((el) => {
        if (!el) throw new Error(`no separator ${css}`);
        return el;
      });
  const right = await separator(".right-splitter [role=separator]");
  await session.execute((el) => el.focus(), right);
  for (let i = 0; i < 5; i++) await session.keys(Keys.ArrowLeft);
  const split = await separator("aside.right [role=separator][aria-orientation=horizontal]");
  await session.execute((el) => el.focus(), split);
  await session.keys(Keys.End);
}

/** 設定の「Connect agents」の待ち受けを入れ直し (つながったエージェントの一覧が空になる)、送り先とトークンを返す。 */
async function restartBridge(session) {
  const toggle = await session.waitFind("[role=dialog] [role=switch]");
  if ((await toggle.attribute("aria-checked")) === "true") {
    await toggle.click();
    await waitFor(async () => (await toggle.attribute("aria-checked")) === "false", {
      message: "the bridge to stop",
    });
  }
  await toggle.click();
  await session.waitFindText("[role=dialog] .state", "Listening on", 20_000);
  const status = await invoke(session, "mcp_bridge_status");
  return { url: status.url, token: await invoke(session, "get_mcp_bridge_token") };
}

/** 「Connected agents」に `name` が出るまで待つ。 */
function agentListed(session, name) {
  return session.waitFindText("[role=dialog] .clients li", name, 20_000);
}

describe("editor manual images", () => {
  const app = launch({ name: "editor" });
  const sampleDir = join(work(), "ugoe-dress-up-sample");
  const psd = join(root, "assets", "fixtures", "character.psd");
  const litProject = join(work(), "avatar-lit.ugoeproj");
  /** 法線マップの無い見本のアバター (AI タブの法線マップの提案)。 */
  const avatarProject = join(work(), "avatar.ugoeproj");
  /** `character.psd` から作って保存するプロジェクト (AI の提案と MCP。エージェントはパスで指す)。 */
  const characterProject = join(work(), "character.ugoeproj");
  /** 偽の AI のプロバイダ (lib/fake-ai.mjs)。 */
  let ai;

  // 法線マップのある見本のアバター (ライティングと検証の画像。`ugoe sample avatar --lit`) と、無いもの (法線マップの提案)
  before(async () => {
    for (const [lit, name] of [
      [true, "avatar-lit"],
      [false, "avatar"],
    ]) {
      const result = spawnSync(
        "just",
        [
          "cli",
          "sample",
          "avatar",
          ...(lit ? ["--lit"] : []),
          "-o",
          join(work(), `${name}.ugoe`),
          "--project",
          join(work(), `${name}.ugoeproj`),
        ],
        { cwd: root, stdio: "inherit" },
      );
      if (result.status !== 0) throw new Error(`ugoe sample avatar (${name}) failed`);
    }
    ai = await startFakeAi();
  });
  after(() => ai?.close());

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

  // チュートリアル (docs/manual/tutorial.md) の 3〜10 章を、手順のとおりに画面で操作して途中の姿を撮る
  step(app, "tutorial: mesh-edit (3)", async () => {
    const s = app.session;
    // ビューポートを広く使う (タイムラインは見本を開く前に開き直す)
    await clickButton(s, "^Collapse the timeline$");
    await selectRow(s, "顔");
    await editMesh(s);
    await settle(s);
    await manualShot(s, "mesh-edit");
    await clickButton(s, "^Done$");
  });

  step(app, "tutorial: wrap in deformers (4)", async () => {
    const s = app.session;
    const wrap = async (row, name, rows, cols, kind) => {
      await selectRow(s, row);
      const section = await sectionOf(s, "Wrap in deformer");
      await fill(s, await fieldIn(s, section, "Name"), name);
      if (rows !== null) {
        await fill(s, await fieldIn(s, section, "Rows"), String(rows));
        await fill(s, await fieldIn(s, section, "Cols"), String(cols));
      }
      await clickButton(s, `^${kind}$`, "aside.right");
    };
    // 包んだデフォーマが選ばれ、インスペクタの先頭に名前と ID が出る (チュートリアルに書いた ID)
    const selected = (label) => s.waitFindText("aside.right", `Deformer: ${label}`, 20_000);
    // 4.1 顔のワープ
    await wrap("頭", "顔のワープ", 3, 3, "Warp");
    await selected("顔のワープ (WarpGroup18)");
    // 回転の中心を置く先 (首の付け根) を、顔のワープの格子の範囲から決める
    const points = await warpPoints(s);
    const ys = points.map((p) => p[1]);
    const xs = points.map((p) => p[0]);
    const neck = [
      (Math.min(...xs) + Math.max(...xs)) / 2,
      Math.min(...ys) + (Math.max(...ys) - Math.min(...ys)) * 0.9,
    ];
    // 4.2 頭の回転 (中心の丸を首の付け根へ)
    await wrap("顔のワープ", "頭の回転", null, null, "Rotation");
    await selected("頭の回転 (RotationGroup18)");
    const origin = await s.waitFind(".gizmo circle.origin");
    const from = await centerOf(s, origin);
    await dragBy(s, from, [neck[0] - from[0], neck[1] - from[1]]);
    // 4.3 目のワープ
    await clickButton(s, "^Parts$");
    await wrap("左目", "", 2, 2, "Warp");
    await selected("左目のワープ (WarpLayer13)");
    await clickButton(s, "^Parts$");
    await wrap("右目", "", 2, 2, "Warp");
    await selected("右目のワープ (WarpLayer12)");
    await clickButton(s, "^Deformers$");
  });

  step(app, "tutorial: parameters (5)", async () => {
    const s = app.session;
    const parameters = [
      ["AngleX", -30, 0, 30],
      ["AngleY", -30, 0, 30],
      ["AngleZ", -30, 0, 30],
      ["EyeLOpen", 0, 1, 1],
      ["EyeROpen", 0, 1, 1],
    ];
    for (const [id, min, value, max] of parameters) {
      await clickButton(s, "^New parameter$", ".pane.parameters");
      const form = await s.waitFind(".pane.parameters form.editor");
      await fill(s, await fieldIn(s, form, "ID"), id);
      await fill(s, await fieldIn(s, form, "Min"), String(min));
      await fill(s, await fieldIn(s, form, "Max"), String(max));
      await fill(s, await fieldIn(s, form, "Default"), String(value));
      await clickButton(s, "^Create$", ".pane.parameters form.editor");
      await waitFor(
        () =>
          s.execute(
            (id) =>
              [...document.querySelectorAll(".pane.parameters button.name")].some(
                (b) => b.textContent.trim() === id,
              ),
            id,
          ),
        { message: `the parameter ${id}` },
      );
    }
  });

  step(app, "tutorial: keys (6)", async () => {
    const s = app.session;
    const keys = async (row, parameter, values) => {
      await selectRow(s, row);
      for (const value of values) {
        await setParameter(s, parameter, value);
        await addKey(s, parameter);
      }
      await clickButton(s, "^Reset$", ".pane.parameters");
    };
    await keys("顔のワープ", "AngleX", [0, 30, -30]);
    await keys("顔のワープ", "AngleY", [0, 30, -30]);
    await selectRow(s, "顔のワープ");
    await scrollInRight(s, await sectionOf(s, "Keyforms"));
    await settle(s);
    await manualShot(s, "keyform-grid");
    await keys("頭の回転", "AngleZ", [0, 30, -30]);
    await keys("左目のワープ", "EyeLOpen", [1, 0]);
    await keys("右目のワープ", "EyeROpen", [1, 0]);
  });

  step(app, "tutorial: edit the shapes on keys (7)", async () => {
    const s = app.session;
    // 7.1 顔のワープ: 格子の内側の 4 点を四角で囲んで選び、ドラッグする
    await selectRow(s, "顔のワープ");
    const moveInner = async (axis, key, delta) => {
      await goToKey(s, axis, key);
      const p = await warpPoints(s);
      const mid = (a, b) => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      await s.pointer([mid(p[0], p[5]), mid(p[5], p[10]), mid(p[10], p[15])]);
      await dragBy(s, (await warpPoints(s))[5], delta);
    };
    await moveInner("AngleX", 30, [25, 0]);
    await moveInner("AngleX", -30, [-25, 0]);
    await goToKey(s, "AngleX", 0);
    await moveInner("AngleY", 30, [0, -25]);
    await moveInner("AngleY", -30, [0, 25]);
    await goToKey(s, "AngleY", 0);
    // 7.2 頭の回転: 角度を数で入れる
    await selectRow(s, "頭の回転");
    for (const [key, angle] of [
      [30, 15],
      [-30, -15],
    ]) {
      await goToKey(s, "AngleZ", key);
      await fill(s, await fieldIn(s, "aside.right", "Angle (this cell)"), String(angle));
    }
    // 7.3 左目のワープ: 上の列の 3 点を選んで、真ん中の列のすぐ上まで下げる
    await selectRow(s, "左目のワープ");
    await goToKey(s, "EyeLOpen", 0);
    const p = await warpPoints(s);
    await s.pointer([p[0]]);
    await s.pointer([p[1]], { shift: true });
    await s.pointer([p[2]], { shift: true });
    await dragBy(s, p[0], [0, p[3][1] - p[0][1] - 3]);
    await clickButton(s, "^Reset$", ".pane.parameters");
  });

  step(app, "tutorial: mirror (9)", async () => {
    const s = app.session;
    // 9.1 パラメータの対
    const openEditor = (name) =>
      s.clickWhen(
        () =>
          s.execute(
            (name) =>
              [...document.querySelectorAll(".pane.parameters button.name")].find(
                (b) => b.textContent.trim() === name,
              ) ?? null,
            name,
          ),
        { message: `the parameter ${name}` },
      );
    await openEditor("EyeLOpen");
    await clickButton(s, "^Pair with EyeROpen$", ".pane.parameters");
    await openEditor("EyeLOpen");
    for (const name of ["AngleX", "AngleZ"]) {
      await openEditor(name);
      await clickButton(s, "^Pair with itself$", ".pane.parameters");
      await openEditor(name);
    }
    // 9.2 要素の対とミラー
    await selectRow(s, "右目のワープ");
    await choose(s, await fieldIn(s, "aside.right", "Partner"), "^左目のワープ \\(WarpLayer13\\)$");
    await clickButton(s, "^Set partner$", "aside.right");
    await clickButton(s, "^Mirror$", "aside.right");
    await s.waitFindText(
      "aside.right p",
      "Mirrored 1 cell from 左目のワープ (WarpLayer13). Kept 1 cell authored (not overwritten).",
    );
    await setParameter(s, "EyeROpen", 0);
    await scrollInRight(
      s,
      await s.execute(
        () =>
          [...document.querySelectorAll("aside.right h4")].find(
            (h) => h.textContent.trim() === "Mirror",
          ) ?? null,
      ),
    );
    await settle(s);
    await manualShot(s, "mirror");
    await clickButton(s, "^Reset$", ".pane.parameters");
  });

  step(app, "tutorial: depth rotation (10)", async () => {
    const s = app.session;
    await selectRow(s, "顔のワープ");
    // 10.1 深度
    const depth = await sectionOf(s, "Depth");
    await fill(s, await fieldIn(s, depth, "Dome peak"), "40");
    await clickButton(s, "^Dome$", "aside.right");
    await s.waitFindText("aside.right section", "16 control points · 0 – 35.28 px");
    // 10.2 推定の方法
    await choose(s, await fieldIn(s, "aside.right", "Method"), "^Depth rotation$");
    await clickButton(s, "^Apply settings$", "aside.right");
    // 10.3 端の 4 つのセルを選び、推定に戻して作り直す
    await (await gridCell(s, 0, -30)).click();
    for (const [row, col] of [
      [0, 30],
      [-30, 0],
      [30, 0],
    ]) {
      await s.pointer([await centerOf(s, await gridCell(s, row, col))], { shift: true });
    }
    await s.waitFindText("aside.right", "4 selected cells");
    await clickButton(s, "^Mark derived$", "aside.right");
    await clickButton(s, "^Infer \\(overwrite derived\\)$", "aside.right");
    await setParameter(s, "AngleX", 30);
    await scrollInRight(s, await sectionOf(s, "Keyforms"));
    await settle(s, 1000);
    await manualShot(s, "depth-rotation");
    await clickButton(s, "^Reset$", ".pane.parameters");
    // 後の画像のために、タイムラインとツリーの表示 (Parts) を元に戻す
    await clickButton(s, "^Expand the timeline$");
    await clickButton(s, "^Parts$");
    await scrollRightToTop(s);
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
    await editMesh(s);
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
    // モデルを入れて適用し、OpenRouter の API キーの欄を出す (適用する前は「Save」が押せず、先に適用する案内が出る)
    const model = await waitFor(
      () =>
        s.execute(
          () =>
            [...document.querySelectorAll("[role=dialog] label")].find(
              (l) => l.textContent.trim() === "Model",
            )?.control ?? null,
        ),
      { message: "the Model field" },
    );
    // 打った後の Enter で設定の欄を送る (「Apply」と同じ)
    await model.replace("openai/gpt-4o");
    await waitFor(
      () =>
        s.execute(() =>
          [...document.querySelectorAll("[role=dialog] h3")].some(
            (h) => h.textContent.trim() === "API key for OpenRouter",
          ),
        ),
      { message: "the OpenRouter API key section" },
    );
    await settle(s, 400);
    // 送り先の欄と API キーの欄。API キーの欄は鍵の保管庫が要る (run.mjs がテストの D-Bus のセッションで動かす。
    // 無ければ「保管庫が無い」の案内になるので、「Apply」の行までにする)
    const withKey = !keyringSkip();
    const rect = await s.execute(
      (dialog, withKey) => {
        const apply = [...dialog.querySelectorAll("button")].find(
          (b) => b.textContent.trim() === "Apply",
        );
        const keyForm = withKey
          ? [...dialog.querySelectorAll("label")]
              .find((l) => l.textContent.trim() === "API key")
              ?.closest("form")
          : null;
        const d = dialog.getBoundingClientRect();
        const bottom = (keyForm ?? apply ?? dialog).getBoundingClientRect().bottom + 12;
        return {
          x: Math.floor(d.left),
          y: Math.floor(d.top),
          width: Math.ceil(d.width),
          height: Math.ceil(Math.min(bottom, d.bottom) - d.top),
        };
      },
      dialog,
      withKey,
    );
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

  step(app, "lighting-point", async () => {
    const s = app.session;
    // 点光源を足し、取っ手を右の肩のそばへドラッグする (光の円の中心から、半径の 6 割ほど右)
    await openTab(s, "Lighting");
    await clickButton(s, "^Add point$", "aside.right");
    const [dial, handle] = await waitFor(
      () =>
        s.execute(() => {
          const dial = document.querySelector(".viewport circle.dial");
          const hits = document.querySelectorAll(".viewport circle.hit");
          if (!dial || hits.length < 2) return null;
          const d = dial.getBoundingClientRect();
          const h = hits[1].getBoundingClientRect();
          return [
            [d.left + d.width / 2, d.top + d.height / 2, d.width / 2],
            [h.left + h.width / 2, h.top + h.height / 2],
          ];
        }),
      { message: "the point light in the viewport" },
    );
    const to = [dial[0] + dial[2] * 0.6, dial[1]];
    await dragBy(s, handle, [to[0] - handle[0], to[1] - handle[1]]);
    const heading = await s.waitFindText("aside.right h3", "Preview lights");
    await scrollInRight(s, heading);
    await settle(s, 1000);
    await manualShot(s, "lighting-point");
    await openTab(s, "Inspector");
    await scrollRightToTop(s);
  });

  step(app, "module-workspace", async () => {
    const s = app.session;
    await openProject(s, join(sampleDir, "cat-ears.ugoeproj"));
    await s.waitFind(".project-info button.module", 20_000);
    await scrollRightToTop(s);
    await settle(s, 1000);
    await manualShot(s, "module-workspace");
  });

  // モジュールの設定・書き出し・比べる (docs/manual/dress-up.md)。見本の cat-ears・sailor・long-hair のプロジェクトで
  step(app, "module-settings", async () => {
    const s = app.session;
    // ここからはビューポートを広く使う (タイムラインは「expressions」で畳んだまま。開いていれば畳む)
    if ((await s.findAll("button[aria-label='Collapse the timeline']")).length > 0) {
      await clickButton(s, "^Collapse the timeline$");
    }
    await s.clickWhen(() => s.find(".project-info button.module"), { message: "the module badge" });
    const dialog = await s.waitFindText("[role=dialog]", "Module settings");
    await settle(s, 400);
    await manualShot(s, "module-settings", { clip: dialog });
    // 取り付け先の違うスロット (体のワープ) へ移すと「Keep where it appears」が出る
    await s.clickText("[role=dialog] label", "Outfit");
    await s.waitFindText("[role=dialog] label", "Keep where it appears");
    await settle(s, 400);
    await manualShot(s, "module-keep-look", { clip: dialog });
    await clickButton(s, "^Cancel$", "[role=dialog]");
  });

  step(app, "module-export-atlas", async () => {
    const s = app.session;
    const path = join(work(), "exported", "cat-ears.ugoemod");
    mkdirSync(dirname(path), { recursive: true });
    await s.clickText("nav.file-menu button", "Export Module…");
    const panel = await s.waitFind(".export-panel");
    const atlas = await fieldIn(s, panel, "Pack textures into atlases");
    if ((await atlas.attribute("aria-checked")) !== "true") await press(s, atlas);
    answerDialog("save-file", path);
    await clickButton(s, "^Export \\.ugoemod", ".export-panel");
    await s.waitFindText(".export-panel", "Packed", 30_000);
    await dismissToasts(s);
    await settle(s, 400);
    // ヘッダーの左から書き出しの欄の下まで
    const rect = await s.execute((panel) => {
      const r = panel.getBoundingClientRect();
      return { x: 0, y: 0, width: Math.ceil(r.right + 160), height: Math.ceil(r.bottom + 48) };
    }, panel);
    await manualShot(s, "module-export-atlas", { clip: rect });
    await s.keys(Keys.Escape);
  });

  /** プレビューに入り、`files` (`.ugoemod` のパス) を読み込む。 */
  async function previewWith(s, files) {
    await clickButton(s, "^Preview$", "header");
    await s.waitFindText(".preview-panel h2", "Preview");
    await viewportReady(s);
    for (const file of files) {
      const before = (await s.findAll(".dress-up .modules > li")).length;
      answerDialog("open-file", file);
      await clickButton(s, "^Load \\.ugoemod", ".dress-up");
      await waitFor(async () => (await s.findAll(".dress-up .modules > li")).length > before, {
        timeout: 20_000,
        message: `${file} in the preview`,
      });
    }
  }

  /** プレビューを出る。 */
  async function exitPreview(s) {
    await clickButton(s, "^Exit preview$", "header");
    await waitFor(async () => (await s.findAll(".preview-panel")).length === 0, {
      message: "the preview to close",
    });
  }

  /** プレビューの着せ替えの節を右の列の頭へ送り、違いの一覧 (あれば) を開いて、節を撮る。 */
  async function shotDressUp(s, name, { open = false, text } = {}) {
    const section = await s.waitFind(".dress-up");
    if (open) {
      await waitFor(
        () =>
          s.execute(() => {
            const details = document.querySelector(".dress-up details.differences");
            if (details && !details.open) details.querySelector("summary").click();
            return details?.open ?? false;
          }),
        { timeout: 20_000, message: "the differences" },
      );
      if (text) await s.waitFindText(".dress-up details.differences", text, 20_000);
    }
    await s.execute((el) => el.scrollIntoView({ block: "start" }), section);
    await settle(s, 600);
    await manualShot(s, name, { clip: section, pad: 8 });
  }

  step(app, "module-compare-exported", async () => {
    const s = app.session;
    // 書き出した cat-ears.ugoemod を読み込むと、いまの編集 (editing) と差し替わる。アトラスにまとめても同じと出る
    await previewWith(s, [join(work(), "exported", "cat-ears.ugoemod")]);
    await s.waitFindText(".dress-up", "Same as your current edits.", 20_000);
    await shotDressUp(s, "module-compare-exported");
    await exitPreview(s);
  });

  step(app, "module-compare-differences", async () => {
    const s = app.session;
    // 耳 (Ears) を消して比べる
    await selectRow(s, "Ears");
    await clickButton(s, "^Delete$", "aside.left");
    await s.clickText("[role=alertdialog] button", "Delete", 3_000).catch(() => {}); // 確かめの窓が出なければそのまま
    await waitFor(
      () =>
        s.execute(
          () =>
            ![...document.querySelectorAll("[role=treeitem] .name")].some(
              (el) => el.textContent.trim() === "Ears",
            ),
        ),
      { message: "Ears to be deleted" },
    );
    await previewWith(s, []);
    await s.waitFindText(".dress-up", "difference from your current edits", 20_000);
    await shotDressUp(s, "module-compare-differences", { open: true });
    await exitPreview(s);
  });

  step(app, "module-compare-details", async () => {
    const s = app.session;
    // Band の頂点を動かし (フォーム編集ですべての頂点を 10px 上へ)、描画順を 990 にする
    await selectRow(s, "Band");
    await clickButton(s, "^Form edit$");
    await s.keys(Keys.Control, "a");
    await s.keys(Keys.Shift, Keys.ArrowUp);
    await new Promise((r) => setTimeout(r, 500));
    await clickButton(s, "^Form edit$");
    await fill(s, await fieldIn(s, "aside.right", "Draw order"), "990");
    await clickButton(s, "^Set for all cells$", "aside.right");
    await previewWith(s, []);
    await s.waitFindText(".dress-up", "difference from your current edits", 20_000);
    await shotDressUp(s, "module-compare-details", { open: true, text: "changed: Band" });
    // 違いの名前を押すと、ビューポートでその要素が強調される
    await s.clickWhen(() => linkIn(s, ".dress-up table.items", "Band"), {
      message: "the Band link",
    });
    await settle(s, 800);
    await manualShot(s, "module-compare-highlight");
    await exitPreview(s);
  });

  step(app, "module-compare-pose", async () => {
    const s = app.session;
    // Sailor の袖 (WaveSleeve) を本体の差分 Wave に追従させるのをやめて、書き出した見本と比べる
    await openProject(s, join(sampleDir, "sailor.ugoeproj"));
    await openTab(s, "Poses");
    await s.clickText("aside.right details.pick > summary", "Links");
    await s.clickText("aside.right label", "WaveSleeve");
    await previewWith(s, [join(root, "assets", "samples", "dress-up", "sailor.ugoemod")]);
    await s.waitFindText(".dress-up", "difference from your current edits", 20_000);
    await s.execute(() => {
      const details = document.querySelector(".dress-up details.differences");
      if (details && !details.open) details.querySelector("summary").click();
    });
    // 本体の差分 Wave を見せる (手を上げ、ファイルの方では袖も一緒に出る)
    await s.clickWhen(
      () =>
        s.execute(
          () =>
            [
              ...document.querySelectorAll(
                ".preview-panel section[aria-label=Pose] button, .preview-panel section[aria-label=Pose] [role=radio]",
              ),
            ].find((b) => b.textContent.trim() === "Wave") ?? null,
        ),
      { message: "the Wave entry" },
    );
    await new Promise((r) => setTimeout(r, 1000));
    await s.clickWhen(() => linkIn(s, ".dress-up table.items", "Wave"), {
      message: "the Wave link",
    });
    // 「Fit」で寄せる。袖は細長く、寄せたままでは周りが見えないので、3 回縮小する
    await clickButton(s, "^Fit$", ".viewport");
    for (let i = 0; i < 3; i++) await clickButton(s, "^Zoom out$", ".viewport");
    await settle(s, 800);
    await manualShot(s, "module-compare-pose");
    await exitPreview(s);
  });

  step(app, "module-glue", async () => {
    const s = app.session;
    await openProject(s, join(sampleDir, "long-hair.ugoeproj"));
    await resizeTo(s, 1280, 1000);
    await openTab(s, "Glue");
    const add = await s.waitFind("aside.right section.add");
    await choose(s, await fieldIn(s, add, "Mesh A"), "^Back$");
    await choose(s, await fieldIn(s, add, "Mesh B"), "^Front$");
    await fill(s, await fieldIn(s, add, "Distance"), "30");
    await clickButton(s, "^Glue$", "aside.right section.add");
    const editor = await s.waitFind("aside.right section.editor", 20_000);
    // 重み 0.25 (スライダーの横の数の欄に入れる)
    await (await editor.find(".ugoe-slider input")).replace("0.25");
    await waitFor(
      async () =>
        Math.abs(
          Number(await (await editor.find("[role=slider]")).attribute("aria-valuenow")) - 0.25,
        ) < 1e-6,
      { message: "the weight 0.25" },
    );
    // 組の一覧 (Pairs) まで写るように、インスペクタを最も高くし (区切りで End)、グルーの一覧を頭へ送る
    const splitter = await s.waitFind(
      "[role=separator][aria-label='Resize the inspector and the parameters']",
    );
    await s.execute((el) => el.focus(), splitter);
    await s.keys(Keys.End);
    await scrollInRight(s, await s.find("aside.right ul.list"));
    await settle(s, 600);
    await manualShot(s, "module-glue", { clip: "aside.right .pane.inspector" });
    // 区切りを既定の高さに戻す (Enter)
    await s.execute((el) => el.focus(), splitter);
    await s.keys(Keys.Enter);
    await resizeTo(s, 1280, 800);
  });

  step(app, "module-glue-pairs", async () => {
    const s = app.session;
    await clickButton(s, "^Add pairs$", ".viewport");
    // 下の帯に隠れる頂点が出るように、縮小する
    await clickButton(s, "^Zoom out$", ".viewport");
    // まだ組んでいない A (Back) の頂点のうち、B の頂点と重ならないものを押す
    const vertex = await waitFor(
      () =>
        s.execute(() => {
          const center = (el) => {
            const r = el.getBoundingClientRect();
            return [r.left + r.width / 2, r.top + r.height / 2];
          };
          const view = document.querySelector(".viewport").getBoundingClientRect();
          const bs = [...document.querySelectorAll(".viewport circle.vertex.b")].map(center);
          for (const el of document.querySelectorAll(".viewport circle.vertex.a:not(.paired)")) {
            const [x, y] = center(el);
            const inside =
              x > view.left + 40 &&
              x < view.right - 40 &&
              y > view.top + 80 &&
              y < view.bottom - 60;
            if (inside && bs.every(([bx, by]) => Math.hypot(bx - x, by - y) > 10)) return [x, y];
          }
          return null;
        }),
      { message: "a free vertex of Back" },
    );
    await s.pointer([vertex]);
    await s.waitFindText(".viewport", "Now click a vertex of Front");
    await settle(s, 400);
    await manualShot(s, "module-glue-pairs", { clip: ".viewport" });
    await s.keys(Keys.Escape);
  });

  step(app, "module-base-readonly", async () => {
    const s = app.session;
    await openTab(s, "Inspector");
    await clickButton(s, "^Parts$");
    await selectRow(s, "Head");
    await s.waitFindText("aside.right", "belongs to the base model");
    await s.execute(() =>
      document
        .querySelector("[role=treeitem][aria-selected=true]")
        ?.scrollIntoView({ block: "center" }),
    );
    await settle(s, 600);
    await manualShot(s, "module-base-readonly");
  });

  step(app, "module-compare-preview", async () => {
    const s = app.session;
    // いまの編集 (LongHair) と、同じスロットの TwinTails を差し替えて比べる。服の Sailor も付ける
    const samples = join(root, "assets", "samples", "dress-up");
    await previewWith(s, [join(samples, "twin-tails.ugoemod"), join(samples, "sailor.ugoemod")]);
    const hair = await s.waitFind(".dress-up [aria-label='Module in slot Hair']");
    await choose(s, hair, "^LongHair \\(editing\\)$");
    await new Promise((r) => setTimeout(r, 1000));
    await shotDressUp(s, "module-compare-preview");
    await exitPreview(s);
  });

  step(app, "module-new", async () => {
    const s = app.session;
    await fileMenu(s, "New Module…");
    await discardIfAsked(s);
    const dialog = await s.waitFindText("[role=dialog]", "New dress-up module");
    // 欄に本体の場所が出る (作業の場所は run.mjs が決まった短い場所にする。tests/e2e/README.md#マニュアルの画像)
    answerDialog("open-directory", join(sampleDir, "dress-up.ugoeproj"));
    await clickButton(s, "^Project folder…$", "[role=dialog]");
    await s.clickText("[role=dialog] label", "Headwear", 20_000);
    await fill(s, await fieldIn(s, dialog, "Module name"), "CatEars");
    await settle(s, 400);
    await manualShot(s, "module-new", { clip: dialog });
    await clickButton(s, "^Cancel$", "[role=dialog]");
  });

  step(app, "module-problem", async () => {
    const s = app.session;
    // 本体の写しからスロット Headwear を消し (no-headwear)、物理を消す (no-physics)。モジュールの写しをその隣に置く
    const problems = join(work(), "problems");
    const copyBase = (name, change) => {
      const dir = join(problems, `${name}.ugoeproj`);
      cpSync(join(sampleDir, "dress-up.ugoeproj"), dir, { recursive: true });
      change(dir);
    };
    const copyModule = (module, base) => {
      const dir = join(problems, `${module}.ugoeproj`);
      cpSync(join(sampleDir, `${module}.ugoeproj`), dir, { recursive: true });
      const project = JSON.parse(readFileSync(join(dir, "project.json"), "utf8"));
      project.module.base = `../${base}.ugoeproj`;
      writeFileSync(join(dir, "project.json"), `${JSON.stringify(project, null, 2)}\n`);
      return dir;
    };
    copyBase("no-headwear", (dir) => {
      const path = join(dir, "model.json");
      const model = JSON.parse(readFileSync(path, "utf8"));
      model.slots = model.slots.filter((slot) => slot.id !== "Headwear");
      model.avatar.slots = model.avatar.slots.filter((slot) => slot !== "Headwear");
      writeFileSync(path, JSON.stringify(model));
    });
    copyBase("no-physics", (dir) => rmSync(join(dir, "physics.json")));
    const openWithProblem = async (dir, text) => {
      answerDialog("open-directory", dir);
      await fileMenu(s, "Open…");
      await discardIfAsked(s);
      return s.waitFindText(".problem", text, 20_000);
    };
    const slotProblem = await openWithProblem(
      copyModule("cat-ears", "no-headwear"),
      "no longer has slot",
    );
    await settle(s, 400);
    await manualShot(s, "module-problem", { clip: slotProblem, pad: 4 });
    const physicsProblem = await openWithProblem(
      copyModule("twin-tails", "no-physics"),
      "has no physics",
    );
    await clickButton(s, "^Open without these colliders…$", ".problem");
    await s.waitFindText(".problem", "Remove and open");
    await settle(s, 400);
    await manualShot(s, "module-problem-drop", { clip: physicsProblem, pad: 4 });
  });

  // AI の提案 (docs/manual/ai.md)。character.psd から作ったプロジェクトで。送る前の確認 (`ai-consent`) と注文の欄
  // (`ai-instructions`) は Claude (鍵の保管庫に偽のキー) で撮り、確認は「Cancel」で閉じる (何も送らない。run.mjs の止める口が
  // 外への要求を見張る)。提案・作り直しは偽の AI のプロバイダ (127.0.0.1) で作る
  step(app, "ai: a project from character.psd", async () => {
    const s = app.session;
    answerDialog("open-file", psd);
    await fileMenu(s, "New from PSD…");
    await discardIfAsked(s);
    await waitFor(() => s.execute(() => document.querySelectorAll("[role=treeitem]").length > 3), {
      timeout: 30_000,
      message: "the PSD in the tree",
    });
    await viewportReady(s);
    // 前の手順の本体の問題 (`module-problem-drop`) の知らせは、PSD から別のプロジェクトを作ると消える (T1672)
    await waitFor(async () => (await s.findAll(".problem")).length === 0, {
      timeout: 5_000,
      message: "the problem of the base to close",
    });
    answerDialog("save-file", characterProject);
    await fileMenu(s, "Save As");
    await waitFor(() => existsSync(join(characterProject, "project.json")), {
      timeout: 20_000,
      message: `${characterProject} to be written`,
    });
    await dismissToasts(s);
    await widenRight(s);
  });

  step(
    app,
    "ai-instructions and ai-consent",
    async () => {
      const s = app.session;
      // 既定のプロバイダの Claude に、偽のキーを使い捨ての鍵の保管庫に置く (送らないので本物でなくてよい)
      await openAiSettings(s);
      await s.clickWhen(() => s.find('[role=dialog] input[type=radio][value="claude"]'), {
        message: "the Claude choice",
      });
      await s.clickText("[role=dialog] form.provider button[type=submit]", "Apply");
      await waitFor(async () => (await invoke(s, "ai_settings")).provider.provider === "claude", {
        message: "Claude to be applied",
      });
      const key = await fieldIn(s, "[role=dialog]", "API key");
      await key.type("sk-ant-e2e-not-a-real-key");
      await s.clickText("[role=dialog] button[type=submit]", "Save");
      await waitFor(async () => (await invoke(s, "ai_settings")).key === "stored", {
        timeout: 20_000,
        message: "the API key to be stored",
      });
      await closeDialog(s);

      // 名前で分類しておく (AI に送らない)。「Propose rig」はリグの値だけを尋ねる (確認は 1 回)
      await openTab(s, "AI");
      await s.waitFindText(".psd", "character.psd");
      await setUseAi(s, false);
      await clickButton(s, "^Classify layers$", "aside.right");
      await s.waitFindText("aside.right p", "classified by name", 20_000);
      await setUseAi(s, true);
      await fill(
        s,
        await instructionsField(s),
        "Turn the head less. Close the eyes a little narrower. Make the hair sway more.",
      );
      await scrollRightToTop(s);
      await settle(s, 400);
      const rect = await s.execute(() => {
        const section = [...document.querySelectorAll("aside.right section")].find(
          (el) => el.querySelector("h3")?.textContent.trim() === "Auto rig from PSD",
        );
        const propose = [...section.querySelectorAll("button")].find(
          (b) => b.textContent.trim() === "Propose rig",
        );
        const r = section.getBoundingClientRect();
        const bottom = propose.closest(".row").getBoundingClientRect().bottom;
        return {
          x: Math.floor(r.left),
          y: Math.floor(r.top),
          width: Math.ceil(r.width),
          height: Math.ceil(bottom + 8 - r.top),
        };
      });
      await manualShot(s, "ai-instructions", { clip: rect });

      await clickButton(s, "^Propose rig$", "aside.right");
      const consent = await s.waitFindText(
        "[role=alertdialog]",
        "Send to Anthropic Claude API?",
        20_000,
      );
      await s.waitFindText("[role=alertdialog] dt", "Your instructions");
      await settle(s, 400);
      await manualShot(s, "ai-consent", { clip: consent });
      // 断ると何も送らずに終わる (提案は作らない)
      await s.clickText("[role=alertdialog] button", "Cancel");
      await waitFor(async () => (await s.findAll("[role=alertdialog]")).length === 0, {
        message: "the consent to close",
      });
      await s.waitFindText("aside.right button:not([disabled])", "Propose rig", 20_000);
      assert.equal((await s.findAll("article.proposal")).length, 0);
    },
    { skip: keyringSkip() },
  );

  step(app, "ai-proposal", async () => {
    const s = app.session;
    // 送り先を偽の AI のプロバイダ (ループバック。確認も API キーも要らない) にする
    await openAiSettings(s);
    await s.clickWhen(() => s.find('[role=dialog] input[type=radio][value="openai_compatible"]'), {
      message: "the OpenAI-compatible choice",
    });
    // 打った後の Enter で設定の欄を送る (「Apply」と同じ)
    await (await fieldIn(s, "[role=dialog] form.provider", "Base URL")).replace(ai.baseUrl);
    await waitFor(
      async () => {
        const settings = await invoke(s, "ai_settings");
        return settings.local && settings.provider.openai.base_url === ai.baseUrl;
      },
      { message: "the fake provider to be applied" },
    );
    await closeDialog(s);

    await openTab(s, "AI");
    await fill(s, await instructionsField(s), "");
    await setUseAi(s, true);
    await clickButton(s, "^Classify layers$", "aside.right");
    await s.waitFindText("aside.right p", "classified by AI", 30_000);
    await clickButton(s, "^Propose rig$", "aside.right");
    const card = await proposalCard(s, 1);
    await clickButton(s, "^Preview$", "article.proposal");
    await s.waitFind(".viewport .proposal-banner", 30_000);
    await maxPreviewSlider(s, "^Angle ?X$");
    await maxPreviewSlider(s, "^Mouth ?Open ?Y$");
    await scrollInRightToBottom(s, card);
    await settle(s, 1500);
    await manualShot(s, "ai-proposal");
  });

  step(app, "ai-revision", async () => {
    const s = app.session;
    await clickButton(s, "^End preview$", "article.proposal");
    await waitFor(async () => (await s.findAll(".viewport .proposal-banner")).length === 0, {
      message: "the preview to end",
    });
    await fill(
      s,
      await instructionsField(s),
      "Turn the head less. Close the eyes a little narrower. Make the hair sway more.",
    );
    await clickButton(s, "^Regenerate with instructions$", "article.proposal");
    const card = await proposalCard(s, 2);
    await s.waitFindText("article.proposal .unapplied", "hair");
    // 偽のプロバイダは従った文 (振り向きを控えめに・目を細く) の値を動かす (lib/fake-ai.mjs)。作り直した提案の値が前と違う
    const { proposals } = await invoke(s, "get_proposals");
    const chosen = (p) => /^Values chosen by the AI: (.*?) \(/.exec(p.rationale)?.[1];
    const [first, revised] = proposals.slice(-2).map(chosen);
    assert.equal(first, "turn 0.75, eye close line 0.60, bend +0.00, mouth open 0.55");
    assert.equal(revised, "turn 0.60, eye close line 0.50, bend +0.00, mouth open 0.55");
    // カードの頭 (題名・印・注文・Not applied・理由の始め) を撮る
    await scrollInRight(s, card);
    await settle(s, 600);
    await manualShot(s, "ai-revision", { clip: card, pad: [0, 4, 4, 4], maxHeight: 360 });
  });

  step(app, "ai-normal-maps", async () => {
    const s = app.session;
    await openProject(s, avatarProject);
    await openTab(s, "AI");
    const section = await waitFor(
      () =>
        s.execute(() =>
          document.querySelector('aside.right section[aria-label="Normal maps from the shapes"]'),
        ),
      { message: "the normal maps section" },
    );
    await waitFor(
      () => s.execute((el) => el.querySelectorAll(".normal-groups > li").length > 0, section),
      { timeout: 30_000, message: "the normal map groups" },
    );
    // 顔のまとまりの縁の丸みとならしを開く
    await s.clickWhen(
      () =>
        s.execute((el) => el.querySelector(".normal-groups > li details.shape summary"), section),
      { message: "the shape of the face group" },
    );
    await clickButton(s, "^Propose normal maps", "aside.right");
    const card = await proposalCard(s, 1);
    await s.execute((el) => el.querySelector("header")?.scrollIntoView(), card);
    await clickButton(s, "^Preview$", "article.proposal");
    await s.waitFind(".viewport .proposal-banner", 60_000);
    await viewportReady(s);
    await scrollInRight(s, section);
    await settle(s, 2000);
    await manualShot(s, "ai-normal-maps");
    await clickButton(s, "^End preview$", "article.proposal");
  });

  // MCP (docs/manual/mcp.md)。偽のエージェント (lib/fake-agent.mjs) が、Claude Desktop・Claude Code の名前でブリッジにつなぐ
  step(app, "mcp-claude-desktop", async () => {
    const s = app.session;
    await openProject(s, characterProject);
    await clickButton(s, "^Settings$", "header");
    const dialog = await s.waitFind("[role=dialog]");
    await clickButton(s, "^Connect agents$", "[role=dialog]");
    const bridge = await restartBridge(s);
    await connectAgent({ ...bridge, name: "claude-ai", version: "1.2.0" });
    await agentListed(s, "claude-ai");
    await clickButton(s, "^Claude Desktop$", "[role=dialog] [role=tablist]");
    const bundle = join(work(), "ugoe-editor.mcpb");
    answerDialog("save-file", bundle);
    await clickButton(s, "^Save \\.mcpb", "[role=dialog]");
    await waitFor(() => existsSync(bundle), {
      timeout: 60_000,
      message: `${bundle} to be written`,
    });
    await s.waitFindText("[role=dialog]", "Saved", 20_000);
    // ダイアログの全体が入るように、窓を高くして撮る
    await resizeTo(s, 1280, 1000);
    await settle(s, 400);
    await manualShot(s, "mcp-claude-desktop", { clip: dialog });
    await resizeTo(s, 1280, 800);
  });

  /** Claude Code の名前でつないだ偽のエージェント (適用の確認に使う)。 */
  let agent;
  step(app, "mcp-connect", async () => {
    const s = app.session;
    const dialog = await s.waitFind("[role=dialog]");
    const bridge = await restartBridge(s);
    agent = await connectAgent({ ...bridge, name: "claude-code", version: "2.5.0" });
    await agentListed(s, "claude-code");
    await clickButton(s, "^Claude Code$", "[role=dialog] [role=tablist]");
    await scrollRightToTop(s);
    await settle(s, 400);
    await manualShot(s, "mcp-connect", { clip: dialog });
    await closeDialog(s);
  });

  step(app, "mcp-approval", async () => {
    const s = app.session;
    // エージェントがレイヤーの名前で分類し (fake-ai.mjs と同じ表)、リグを提案して適用を求める
    const listed = await agent.call("get_psd_layers", {
      path: characterProject,
      psd,
      images: false,
    });
    const layers = listed.layers
      .filter((l) => l.has_pixels)
      .map((l) => {
        const [label, side] = LABELS[l.path.at(-1)] ?? ["other", "center"];
        return { index: l.index, label, side, confidence: 0.9 };
      });
    await agent.call("set_layer_labels", { path: characterProject, layers });
    const rig = await agent.call("propose_rig", {
      path: characterProject,
      title: "Rig the head turn, eyes and mouth",
    });
    const applied = agent.call("apply_proposal", {
      path: characterProject,
      proposal: rig.proposal,
    });
    applied.catch(() => {});
    await s.waitFindText(".approval h2", "claude-code wants to apply", 20_000);
    await clickButton(s, "^Show in AI tab$", ".approval");
    await s.waitFind(".viewport .proposal-banner", 30_000);
    await maxPreviewSlider(s, "^Angle ?X$");
    await scrollInRightToBottom(s, await proposalCard(s, 1));
    await settle(s, 1500);
    await manualShot(s, "mcp-approval");
    await clickButton(s, "^Apply$", ".approval");
    await applied;
  });
});
