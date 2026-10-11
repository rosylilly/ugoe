// スタジオ (apps/studio) のマニュアルの画像 (docs/manual/images の studio-*.png) を撮る (`just e2e --manual --only studio`)。
// マニュアルのスタジオの章は日本語の画面で書くので、日本語の画面と日本語の見本のプロファイル (sample-ja.json) で撮る。
// 流し方は tests/e2e/README.md#マニュアルの画像。

import { mkdirSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { join } from "node:path";
import { describe } from "node:test";
import {
  resolveSampleProfile,
  sampleProfile,
} from "../../../apps/studio/scripts/sample-profile.mjs";
import { configHome, invoke, launch, step, switchToTauriWindow, work } from "../lib/harness.mjs";
import { manualShot, resizeTo, useLocale } from "../lib/manual.mjs";
import { installPageHelpers } from "../lib/page.mjs";
import { startFakeTracking } from "../lib/tracking.mjs";
import { Keys, waitFor } from "../lib/webdriver.mjs";

/**
 * 操作 API とブラウザソースのポート。画像の接続先・URL・ポートの欄がマニュアルの文 (既定は `ws://127.0.0.1:28451/api`) と
 * 合い、撮るたびに変わらないように、筋書き (specs/studio.test.mjs) の空いているポートではなく既定のポートで開く (T0977)。
 */
const API_PORT = 28451;

/** `API_PORT` が空いていることを確かめる (利用者のスタジオなどが使っていれば、理由を出して止める)。 */
function checkApiPortFree() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", () =>
      reject(
        new Error(
          `port ${API_PORT} (the studio's default API port) is in use; quit the studio using it before taking the manual images`,
        ),
      ),
    );
    server.listen(API_PORT, "127.0.0.1", () => server.close(() => resolve()));
  });
}

/**
 * 日本語の見本のプロファイルを書き、最近のプロファイルに置く (起動の前に。specs/studio.test.mjs と同じ)。
 * ホットキーはパネルの画像に要るので残す。Xvfb の画面に登録されるだけで、利用者の画面には届かない
 * (`--headed` のときだけ外す)。
 */
function placeSampleProfile() {
  const profile = resolveSampleProfile(sampleProfile("ja"));
  if (process.env.E2E_HEADED) profile.hotkeys = [];
  const path = join(work(), "sample-ja.json");
  writeFileSync(path, `${JSON.stringify(profile, null, 2)}\n`);
  const config = join(configHome(), "app.ugoe.studio");
  mkdirSync(config, { recursive: true });
  writeFileSync(join(config, "recent-profiles.json"), JSON.stringify({ profiles: [path] }));
  return path;
}

/** 操作ウィンドウの、今の放送中のシーンの名前。 */
function liveScene(session) {
  return session.execute(
    () =>
      document
        .querySelector(".scenes__item[aria-current=true] .scenes__name")
        ?.textContent.trim() ?? null,
  );
}

async function switchScene(session, name) {
  const item = await session.waitFindText(".scenes__item", name);
  await item.click();
  await waitFor(async () => (await liveScene(session)) === name, { message: `${name} on air` });
}

/** 見出しが `title` のパネル (@ugoe/ui の Section)。開いて、表示域の頭まで送って返す。 */
async function panel(session, title) {
  const section = await waitFor(
    () =>
      session.execute((title) => {
        for (const section of document.querySelectorAll(".ugoe-section")) {
          const heading = section.querySelector(
            ":scope > .ugoe-section__header .ugoe-section__heading",
          );
          if (heading?.textContent.trim() === title) return section;
        }
        return null;
      }, title),
    { message: `the panel ${title}` },
  );
  const trigger = await section.find(".ugoe-section__trigger");
  if ((await trigger.attribute("aria-expanded")) === "false") await trigger.click();
  await session.execute((el) => el.scrollIntoView({ block: "start" }), section);
  return section;
}

/** パネルの周りの余白 (右は列のスクロールバーを入れないように狭く)。 */
const PANEL_PAD = [8, 2, 8, 8];

/** パネルを撮る (パネルの全体。`maxHeight` か表示域より高ければそこまで)。 */
async function panelShot(session, title, name, maxHeight) {
  const section = await panel(session, title);
  await new Promise((r) => setTimeout(r, 300));
  await manualShot(session, name, { clip: section, pad: PANEL_PAD, maxHeight });
}

/** 操作 API のクライアント (承認を求める)。specs/studio.test.mjs の rpcClient の小さなもの。 */
function connectClient(url, hello) {
  const ws = new WebSocket(url);
  const opened = new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", () => reject(new Error(`cannot connect to ${url}`)), {
      once: true,
    });
  });
  const answered = new Promise((resolve) => {
    ws.addEventListener("message", (e) => {
      const msg = JSON.parse(String(e.data));
      if (msg.id === 1) resolve(msg);
    });
  });
  opened.then(() =>
    ws.send(JSON.stringify({ jsonrpc: "2.0", id: 1, method: "session.hello", params: hello })),
  );
  return { ws, opened, answered };
}

describe("studio manual images", () => {
  placeSampleProfile();
  const app = launch({ name: "studio" });
  const names = { chat: "雑談", dressUp: "着せ替え" };
  /** 承認した操作 API のクライアント (最後に閉じる)。 */
  const clients = [];

  step(app, "starts in Japanese with the sample profile", async () => {
    const s = app.session;
    await checkApiPortFree();
    await switchToTauriWindow(s, "main");
    await waitFor(() => s.execute("return !!window.__UGOE_TRACKER__"), {
      message: "the control window to start",
    });
    await useLocale(s, "ugoe.studio.locale", "ja");
    await waitFor(() => s.execute("return !!window.__UGOE_TRACKER__"), {
      message: "the control window to start again",
    });
    await installPageHelpers(s);
    await waitFor(
      () => s.execute(`return __e2e.byText("button", "^モデルを選ぶ")?.disabled === false`),
      { message: "the toolbar to be enabled" },
    );
    await waitFor(async () => (await liveScene(s)) === names.chat, { message: "雑談 on air" });
    await resizeTo(s, 1280, 800);
  });

  step(app, "studio-overview", async () => {
    const s = app.session;
    // 記録の再生でトラッキングを動かし、表情 smile を付ける。出力ウィンドウは隠したまま (Xvfb にはコンポジタが無く、
    // 透過のシーンを出すと出力の印が注意になる。出した様子は studio-output-panel で撮る)
    await startFakeTracking(s, { seconds: 120 });
    await s.clickText(".studio__center button", "smile");
    await s.execute(() => document.querySelector(".studio__panels")?.scrollTo(0, 0));
    await s.execute(() => document.querySelector(".studio__center")?.scrollTo(0, 0));
    await new Promise((r) => setTimeout(r, 1500));
    await manualShot(s, "studio-overview");
  });

  step(app, "studio-motion", async () => {
    const s = app.session;
    await panelShot(s, "表情とモーション", "studio-motion");
  });

  step(app, "studio-scenes", async () => {
    const s = app.session;
    await switchScene(s, names.dressUp);
    await s.execute(() => document.querySelector(".studio__panels")?.scrollTo(0, 0));
    await s.execute(() => document.querySelector(".studio__center")?.scrollTo(0, 0));
    await new Promise((r) => setTimeout(r, 1000));
    await manualShot(s, "studio-scenes");
  });

  step(app, "studio-pose", async () => {
    const s = app.session;
    const section = await panel(s, "ポーズ");
    await s.clickWhen(
      () =>
        s.execute(
          (section) =>
            [...section.querySelectorAll("button, [role=radio]")].find(
              (b) => b.textContent.trim() === "Wave",
            ) ?? null,
          section,
        ),
      { message: "the Wave pose" },
    );
    await new Promise((r) => setTimeout(r, 500));
    await manualShot(s, "studio-pose", { clip: section, pad: PANEL_PAD });
    await switchScene(s, names.chat);
  });

  step(app, "studio-inputs", async () => {
    const s = app.session;
    await resizeTo(s, 1280, 1000);
    await panelShot(s, "入力", "studio-inputs");
  });

  step(app, "studio-inputs-tuning", async () => {
    const s = app.session;
    // 記録の再生の首が正面の近くを通る時にキャリブレーションする (中立が正面からずれて、メーターが片寄らないように)
    await waitFor(
      () =>
        s.execute(() => {
          const head = window.__UGOE_TRACKER__?.tuningPreview?.before.head;
          return !!head && Math.abs(head.yaw) < 3 && Math.abs(head.roll) < 3;
        }),
      { interval: 20, message: "the replayed head near the front" },
    );
    await s.clickText(".studio__panels button", "キャリブレーション");
    // 値を既定から動かし、メーターに遊び・境目の点線が出るようにする (マニュアルの調整の節の例)
    await s.execute(() => {
      const tracker = window.__UGOE_TRACKER__;
      tracker.setTuning({
        ...tracker.tuning,
        yaw: { gain: 1.5, deadZone: 3 },
        roll: { gain: 1, deadZone: 2 },
        eyeBlink: { low: 0.1, high: 0.7 },
        jawOpen: { low: 0.1, high: 0.75 },
      });
    });
    const section = await panel(s, "調整 (感度と境目)");
    await new Promise((r) => setTimeout(r, 300));
    // 下はすぐ後に「記録」の行が続くので、余白を狭く
    await manualShot(s, "studio-inputs-tuning", { clip: section, pad: [8, 2, 2, 8] });
    // 戻して閉じる (この後の画像に写さない)
    await s.clickText(".studio__panels button", "既定に戻す");
    await (await section.find(".ugoe-section__trigger")).click();
  });

  step(app, "studio-output-panel", async () => {
    const s = app.session;
    await s.clickText(".studio__toolbar button", "出力ウィンドウ");
    await waitFor(() => s.execute(`return __e2e.toggleChecked("^出力ウィンドウを表示")`), {
      message: "the output window to be shown",
    });
    await switchToTauriWindow(s, "main");
    // Xvfb にはコンポジタが無く透過が効かないので、「背景」の欄に注意と「背景をクロマキーにする」が出る。マニュアルは
    // 「はじめての配信の準備」でこの注意を説明し、画像はその例なので、出ていることを確かめる (T0977)
    await s.waitFindText(".ugoe-notice", "透過が効いていません");
    await panelShot(s, "出力ウィンドウ", "studio-output-panel");
  });

  step(app, "studio-hotkeys", async () => {
    const s = app.session;
    // 割り当ての多い見本では長いので、頭 (シーンの切り替えと表情) だけ
    await panelShot(s, "ホットキー", "studio-hotkeys", 620);
  });

  step(app, "studio-load", async () => {
    const s = app.session;
    // 数値は撮った計算機の測定のまま。注意は読者の画面 (GPU で描く) ではふつう出ないので、出ていない時に撮る。
    // Xvfb はソフトウェアで描くので、出力ウィンドウの描画の遅れの注意が出たり消えたりする。撮った後も出ていなければよし (T0977)
    const section = await panel(s, "フレームレートと負荷");
    const calm = async () => {
      const state = await s.execute(
        (section) => ({
          rows: [...section.querySelectorAll("dt")].map((dt) => dt.textContent.trim()),
          warnings: [...section.querySelectorAll(".ugoe-notice")].map((n) => n.textContent.trim()),
        }),
        section,
      );
      return state.rows.includes("出力ウィンドウ") && state.warnings.length === 0 ? true : state;
    };
    let last = null;
    for (let attempt = 0; attempt < 10; attempt++) {
      last = await waitFor(async () => (await calm()) === true, {
        timeout: 10_000,
        message: "the load panel without warnings",
      }).catch((e) => e);
      if (last !== true) continue;
      await manualShot(s, "studio-load", { clip: section, pad: PANEL_PAD });
      last = await calm();
      if (last === true) return;
    }
    throw new Error(
      `the load panel kept warnings (last: ${last instanceof Error ? last.message : JSON.stringify(last)}). Retake on a less busy machine`,
    );
  });

  step(app, "studio-source", async () => {
    const s = app.session;
    await panel(s, "ブラウザソース (OBS)");
    const sw = await s.execute(`return __e2e.toggle("^OBS のブラウザソース")`);
    if (!sw) throw new Error("no browser source switch");
    await sw.click();
    await waitFor(() => s.execute(`return __e2e.listItem("", "^ソース オン$|^ソース$") !== null`), {
      timeout: 5_000,
      message: "the source badge",
    }).catch(() => {});
    await panelShot(s, "ブラウザソース (OBS)", "studio-source");
  });

  step(app, "studio-api-approval", async () => {
    const s = app.session;
    await resizeTo(s, 1280, 800);
    await panel(s, "操作 API");
    const sw = await s.execute(`return __e2e.toggle("^この計算機のほかのアプリから")`);
    await sw.click();
    await waitFor(async () => (await invoke(s, "api_state"))?.running === true, {
      timeout: 5_000,
      message: "the API to start",
    }).catch(() => {});
    const token = await invoke(s, "api_token");
    const client = connectClient(`ws://127.0.0.1:${API_PORT}/api`, {
      name: "Chat bot",
      token,
      permission: "control",
    });
    clients.push(client);
    await client.opened;
    const notice = await s.waitFindText(".ugoe-notice", "接続を求めています");
    await s.clickText(".ugoe-notice .ugoe-segmented__item", "読む・操作する");
    await new Promise((r) => setTimeout(r, 300));
    await manualShot(s, "studio-api-approval", { clip: notice, pad: 4 });
    await s.clickText(".ugoe-notice button", "承認");
    await client.answered;
    const approved = await s.waitFindText(".ugoe-notice", "を承認しました");
    await new Promise((r) => setTimeout(r, 300));
    await manualShot(s, "studio-api-approved", { clip: approved, pad: 4 });
  });

  step(app, "studio-api", async () => {
    const s = app.session;
    await panelShot(s, "操作 API", "studio-api");
  });

  step(app, "studio-settings", async () => {
    const s = app.session;
    await s.clickWhen(() => s.find(".studio__toolbar button[aria-label=設定]"), {
      message: "the settings button",
    });
    const dialog = await s.waitFind("[role=dialog]");
    await new Promise((r) => setTimeout(r, 500));
    await manualShot(s, "studio-settings", { clip: dialog });
    await s.keys(Keys.Escape);
    for (const client of clients) client.ws.close();
  });
});
