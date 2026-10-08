// スタジオ (apps/studio) の主な流れ: 見本のプロファイルを開く・シーンを切り替える・出力ウィンドウ・
// 重い検出の下でのトラッキング (T0812 の回帰)・出力ウィンドウを隠しても止まらない・操作 API・ブラウザソース。
// 流し方は tests/e2e/README.md。

import assert from "node:assert/strict";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { join } from "node:path";
import { describe } from "node:test";
import {
  resolveSampleProfile,
  sampleProfile,
} from "../../../apps/studio/scripts/sample-profile.mjs";
import {
  captureLogs,
  checkNoCspViolations,
  configHome,
  invoke,
  launch,
  shot,
  step,
  switchToTauriWindow,
  takeCspViolations,
  work,
} from "../lib/harness.mjs";
import { countInvokes, installPageHelpers, invokeStats, percentile } from "../lib/page.mjs";
import { checkImageTracking, startFakeTracking, trackerState } from "../lib/tracking.mjs";
import { waitFor } from "../lib/webdriver.mjs";

/** 空いている TCP のポート (操作 API に使う。利用者のスタジオの既定のポートとぶつけない)。 */
function freePort() {
  return new Promise((resolve) => {
    const server = createServer();
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

/** 操作ウィンドウの、今の放送中のシーンの名前。 */
function liveScene(session) {
  return session.execute(`
    const live = document.querySelector(".scenes__item[aria-current=true] .scenes__name");
    return live ? live.textContent.trim() : null;
  `);
}

/** 出力ウィンドウに、届いたフレームを数える仕掛けを入れる (`window.__e2eFrames`)。 */
async function countOutputFrames(session) {
  await session.execute(`
    const player = window.__UGOE_STUDIO_OUTPUT__;
    if (!player) throw new Error("window.__UGOE_STUDIO_OUTPUT__ is missing");
    if (window.__e2eFrames) return;
    const stats = (window.__e2eFrames = { count: 0, lastSeq: null, spread: {} });
    const push = player.pushFrame.bind(player);
    player.pushFrame = (frame) => {
      stats.count++;
      stats.lastSeq = frame.seq;
      push(frame);
    };
  `);
}

const outputFrames = (session) => session.execute("return window.__e2eFrames?.count ?? 0");

/** JSON-RPC の小さなクライアント (操作 API。docs/studio-api.md)。 */
function rpcClient(url) {
  const ws = new WebSocket(url);
  let next = 1;
  const waiting = new Map();
  const events = [];
  ws.addEventListener("message", (e) => {
    const msg = JSON.parse(String(e.data));
    if (msg.id !== undefined && waiting.has(msg.id)) {
      waiting.get(msg.id)(msg);
      waiting.delete(msg.id);
    } else {
      events.push(msg);
    }
  });
  const opened = new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", () => reject(new Error(`cannot connect to ${url}`)), {
      once: true,
    });
  });
  return {
    ws,
    events,
    opened,
    /** 要求を送る。応答 (result か error を持つ) を待つ Promise を返す。 */
    call(method, params = {}) {
      const id = next++;
      ws.send(JSON.stringify({ jsonrpc: "2.0", id, method, params }));
      return new Promise((resolve) => waiting.set(id, resolve));
    },
  };
}

/**
 * 見本のプロファイルを書き、最近のプロファイルに置く (起動の前に)。開くダイアログは OS のもので、バックエンドのコマンドは
 * ダイアログで選んだパスと最近のプロファイルしか開かない (src-tauri/src/access.rs、T0838) ので、起動したスタジオが最後の
 * プロファイルとして開く。
 */
function placeSampleProfile() {
  const profile = resolveSampleProfile(sampleProfile("en"));
  // ホットキーは OS 全体に登録されるので、テストでは持たない (--headed で利用者の画面を使うときのため)
  profile.hotkeys = [];
  const path = join(work(), "sample-profile.json");
  writeFileSync(path, `${JSON.stringify(profile, null, 2)}\n`);
  const config = join(configHome(), "app.ugoe.studio");
  mkdirSync(config, { recursive: true });
  writeFileSync(join(config, "recent-profiles.json"), JSON.stringify({ profiles: [path] }));
  return path;
}

describe("studio", () => {
  const profilePath = placeSampleProfile();
  const app = launch({ name: "studio" });
  const names = { chat: "Chat", evening: "Evening (lighting)" };

  step(app, "starts with the control window", async () => {
    const s = app.session;
    await switchToTauriWindow(s, "main");
    await waitFor(() => s.execute("return !!window.__UGOE_TRACKER__"), {
      message: "the control window to start",
    });
    await installPageHelpers(s);
    // ホストにつながるとボタンが押せるようになる
    await waitFor(
      () => s.execute(`return __e2e.byText("button", "^Choose model")?.disabled === false`),
      { message: "the toolbar to be enabled" },
    );
  });

  step(app, "opens the last profile at startup", async () => {
    const s = app.session;
    await waitFor(async () => (await liveScene(s)) === names.chat, { message: "Chat on air" });
    // 選んでいないパスは開けない (T0838)
    const code = await s.run(
      (path) =>
        window.__TAURI_INTERNALS__.invoke("open_profile", { path }).then(
          () => null,
          (e) => e.code,
        ),
      join(work(), "not-chosen.json"),
    );
    assert.equal(code, "file.not_chosen");
    const scenes = await s.execute(
      `return [...document.querySelectorAll(".scenes__item .scenes__name")].map((e) => e.textContent.trim())`,
    );
    assert.deepEqual(scenes, ["Chat", "Evening (lighting)", "Dress-up", "Game (corner)"]);
    await waitFor(() => s.execute(`return __e2e.byText(".studio__facts dd", "avatar") !== null`), {
      message: "the model name in the state panel",
    });
    await shot(s, "studio-profile");
  });

  step(app, "switches scenes by clicking", async () => {
    const s = app.session;
    const evening = await s.waitFindText(".scenes__item", names.evening);
    await evening.click();
    await waitFor(async () => (await liveScene(s)) === names.evening, {
      message: "Evening on air",
    });
    const chat = await s.waitFindText(".scenes__item", names.chat);
    await chat.click();
    await waitFor(async () => (await liveScene(s)) === names.chat, { message: "Chat on air" });
  });

  // 仮想カメラの開始と停止はスレッドの終わりを待つ。メインのスレッドの外で行うので、その後も切り替えられる (T0819)。
  // 機器 (v4l2loopback) が無ければ始められずに止まるが、開始と停止の呼び出しは返る。
  step(app, "starts and stops the virtual camera", async () => {
    const s = app.session;
    const settings = { width: 1280, height: 720, fps: 30, background: [0, 255, 0] };
    await invoke(s, "start_virtual_camera", { settings });
    await invoke(s, "stop_virtual_camera");
    await waitFor(
      async () => ["stopped", "failed"].includes((await invoke(s, "virtual_camera_state")).state),
      { message: "the virtual camera to stop" },
    );
    const evening = await s.waitFindText(".scenes__item", names.evening);
    await evening.click();
    await waitFor(async () => (await liveScene(s)) === names.evening, {
      message: "Evening on air after the virtual camera",
    });
    const chat = await s.waitFindText(".scenes__item", names.chat);
    await chat.click();
    await waitFor(async () => (await liveScene(s)) === names.chat, { message: "Chat on air" });
  });

  step(app, "shows the output window and it receives frames", async () => {
    const s = app.session;
    const button = await s.waitFindText(".studio__toolbar button", "Output window");
    await button.click();
    await waitFor(() => s.execute(`return __e2e.toggleChecked("^Show the output window")`), {
      message: "the output window to be shown",
    });
    // 見本のプロファイルの大きさ (1920×1080) が「いまの状態」に出る。前は Linux で、隠れている間に大きさを変えた窓の
    // 大きさを tao が前のまま (960×540) 返し、「OS が小さくした」の注意が出ていた (T0919)
    await waitFor(
      () => s.execute(`return __e2e.listItem("^Status$", "^Output 1920 × 1080$") !== null`),
      { message: "Output 1920 × 1080 in Status" },
    );
    await switchToTauriWindow(s, "output");
    await captureLogs(s);
    await countOutputFrames(s);
    const before = await outputFrames(s);
    await waitFor(async () => (await outputFrames(s)) > before + 30, {
      timeout: 10_000,
      message: "frames in the output window",
    });
    await waitFor(() => s.execute("return window.__UGOE_STUDIO_OUTPUT__.modelName"), {
      message: "the model in the output window",
    });
    await shot(s, "studio-output");
    // 出力ウィンドウは受け手のコマンドしか呼べず、CSP が掛かっている (T0820)
    const denied = await s.run(() =>
      window.__TAURI_INTERNALS__.invoke("api_token").then(
        () => null,
        (e) => String(e),
      ),
    );
    // CSP は応答のヘッダで付く (ページからは読めない) ので、`'unsafe-eval'` を許していないことで確かめる
    const evaluated = await s.execute(
      "try { return new Function('return true')(); } catch (e) { return String(e); }",
    );
    // わざと止めさせたものは集めた違反に入る (最後の「CSP の違反が無い」の手順のために取り除く)
    const violations = await waitFor(
      async () => {
        const taken = await takeCspViolations(s);
        return taken.length > 0 ? taken : null;
      },
      { message: "the blocked eval in the collected CSP violations" },
    );
    await switchToTauriWindow(s, "main");
    assert.match(denied ?? "", /not allowed/, "the output window could read the API token");
    assert.notEqual(evaluated, true, "the output window has no CSP");
    assert.ok(
      violations.every((v) => v.directive === "script-src" && v.blocked === "eval"),
      `unexpected CSP violations: ${JSON.stringify(violations)}`,
    );
  });

  step(app, "warns in Background when transparency does not work (T0846, T0849)", async () => {
    const s = app.session;
    // Xvfb にはコンポジタが無いので `unsupported`。`--headed` で合成する画面に出したときは `supported`。
    const { transparency } = await invoke(s, "output_state");
    assert.ok(
      ["supported", "unsupported"].includes(transparency),
      `Linux should know whether the screen is composited: ${transparency}`,
    );
    // 「雑談」は透過のシーン。効かなければ「出力ウィンドウ」のパネルの「背景」の欄に注意を出す
    const notice = () =>
      s.execute(`
        const facts = document.querySelector(".output-panel__facts");
        return facts ? /Transparency is not working/.test(facts.textContent) : null;
      `);
    await waitFor(async () => (await notice()) === (transparency === "unsupported"), {
      message: `the transparency notice to match ${transparency}`,
    });
    if (transparency !== "unsupported") return;
    // 注意の「背景をクロマキーにする」で、配信しているシーンの背景がクロマキーになり、注意が消える (T0849)
    const chroma = await s.waitFindText(".output-panel__facts button", "Use chroma key background");
    await chroma.click();
    await waitFor(
      () =>
        s.execute(`
          const facts = document.querySelector(".output-panel__facts");
          return facts !== null && /Chroma key/.test(facts.textContent) && !/Transparency is not working/.test(facts.textContent);
        `),
      { message: "the live scene background to become chroma key" },
    );
    // 「シーンの編集」の「背景」で透過に戻すと、注意がまた出る (後の手順は透過の「雑談」のまま進める)
    const transparent = await s.waitFindText(".editor .ugoe-segmented__item", "Transparent");
    await transparent.click();
    await waitFor(async () => (await notice()) === true, {
      message: "the transparency notice after going back to transparent",
    });
  });

  step(
    app,
    "tracking under heavy detection reaches the control and the output (T0812)",
    async () => {
      const s = app.session;
      await countInvokes(s, ["set_tracking"]);
      // MediaPipe の検出 1 回 (WSL2 の WebKitGTK で約 200ms) を模す
      await startFakeTracking(s, { costMs: 200, seconds: 6 });
      const angleX = [];
      const started = Date.now();
      while (Date.now() - started < 10_000) {
        angleX.push(await s.execute(`return __e2e.sliderValue("^Angle ?X( |$)")`));
        await new Promise((r) => setTimeout(r, 250));
      }
      const stats = await invokeStats(s);
      const tracker = await trackerState(s);
      const done = stats.done.set_tracking ?? 0;
      const p50 = percentile(stats.latency.set_tracking ?? [], 0.5);
      const report = { done, errors: stats.errors.set_tracking ?? 0, p50, tracker, angleX };
      writeFileSync(
        join(process.env.E2E_OUT, "tracking.json"),
        `${JSON.stringify(report, null, 2)}\n`,
      );
      assert.equal(tracker.status, "running", JSON.stringify(tracker));
      // 直す前は 50 秒で 3 回しか届かず、応答に何秒もかかった (docs/tasks/T0812)
      assert.ok(done >= 10, `set_tracking reached the backend only ${done} times in 10 s`);
      assert.ok(p50 !== null && p50 < 1000, `set_tracking took ${p50} ms (p50)`);
      const seen = angleX.filter((v) => v !== null);
      assert.ok(seen.length > 0, "no AngleX slider in the control window");
      const spread = Math.max(...seen) - Math.min(...seen);
      assert.ok(spread > 5, `AngleX in the control window moved only ${spread}`);

      // 「フレームレートと負荷」に、値が届いてから刻みで使われるまでの待ちが出る (届いていない間の「—」ではない。T0863)
      const trigger = await s.waitFind('[data-panel="load"] .ugoe-section__trigger');
      const wasOpen = (await trigger.attribute("aria-expanded")) === "true";
      if (!wasOpen) await trigger.click();
      const trackingWait = () =>
        s.execute(() => {
          const dt = [...document.querySelectorAll('[data-panel="load"] dt')].find(
            (el) => el.textContent.trim() === "Tracking wait",
          );
          return dt?.nextElementSibling?.textContent.trim() ?? null;
        });
      await waitFor(
        async () => /^Average .+ ms, max .+ ms, .+% missed their tick$/.test(await trackingWait()),
        {
          timeout: 5_000,
          message: "the tracking wait in the load panel",
        },
      ).catch(async (e) => {
        throw new Error(`${e.message}: ${JSON.stringify(await trackingWait())}`);
      });
      // 測った待ちも残す (実機の値と比べるため)
      report.trackingWait = await trackingWait();
      writeFileSync(
        join(process.env.E2E_OUT, "tracking.json"),
        `${JSON.stringify(report, null, 2)}\n`,
      );
      if (!wasOpen) await trigger.click();

      await switchToTauriWindow(s, "output");
      const output = await s.execute("return window.__UGOE_STUDIO_OUTPUT__.values");
      await switchToTauriWindow(s, "main");
      assert.ok(
        output && Object.keys(output).length > 0,
        "the output window has no parameter values",
      );
    },
  );

  step(app, "keeps sending frames while the output window is hidden", async () => {
    const s = app.session;
    const sw = await s.execute(`return __e2e.toggle("^Show the output window")`);
    await sw.click();
    await waitFor(
      async () =>
        (await s.execute(`return __e2e.toggleChecked("^Show the output window")`)) === false,
      {
        message: "the output window to be hidden",
      },
    );
    await switchToTauriWindow(s, "output");
    const before = await outputFrames(s);
    const hidden = await s.execute("return document.hidden");
    await new Promise((r) => setTimeout(r, 3000));
    const after = await outputFrames(s);
    await switchToTauriWindow(s, "main");
    // 刻みは 60 回/秒。3 秒で少なくとも 1 秒分
    assert.ok(
      after - before >= 60,
      `only ${after - before} frames in 3 s while hidden (document.hidden=${hidden})`,
    );
    const tracker = await trackerState(s);
    assert.equal(tracker.status, "running");
  });

  step(app, "tracks a face in an image with MediaPipe", async () => {
    const s = app.session;
    // 記録の再生 (前の手順) を止めて、操作ウィンドウのトラッキングのパネルに画像を入れる
    const result = await checkImageTracking(s, "studio-tracking-image");
    assert.ok(result.after > result.before, JSON.stringify(result));
  });

  step(app, "connects a client to the studio API", async () => {
    const s = app.session;
    const port = await freePort();
    await invoke(s, "set_api_port", { port });
    const sw = await s.execute(`return __e2e.toggle("^Let other apps")`);
    await sw.click();
    await waitFor(() => s.execute(`return __e2e.listItem("^Status$", "^API on$") !== null`), {
      message: "the API on badge in Status",
    });
    const token = await invoke(s, "api_token");
    const client = rpcClient(`ws://127.0.0.1:${port}/api`);
    await client.opened;
    try {
      const hello = client.call("session.hello", {
        name: "e2e client",
        token,
        permission: "control",
      });
      // 操作ウィンドウで承認する (読み取りと操作)
      const control = await s.waitFindText(".ugoe-segmented__item", "Read and control");
      await control.click();
      const approve = await s.waitFindText("button", "Approve");
      await approve.click();
      const hi = await hello;
      assert.equal(hi.error, undefined, JSON.stringify(hi.error));
      assert.equal(hi.result.permission, "control");
      assert.match(hi.result.key, /^[0-9a-f]{64}$/);
      // 承認の後に「取り消す」の知らせが出る。承認したクライアントは承認を待つ間と違う ID を持つので、
      // 画面がその ID で探さないと出ない (偽のホストでは同じ ID で、本物でだけ出なかった。T0919)
      await s.waitFindText(".ugoe-notice", "Approved e2e client");

      const list = await client.call("scene.list");
      assert.deepEqual(
        list.result.scenes.map((sc) => sc.name),
        ["Chat", "Evening (lighting)", "Dress-up", "Game (corner)"],
      );
      const activate = await client.call("scene.activate", { scene: "evening" });
      assert.equal(activate.error, undefined, JSON.stringify(activate.error));
      await waitFor(async () => (await liveScene(s)) === names.evening, {
        message: "the API to switch the scene",
      });
      const state = await client.call("studio.state");
      assert.equal(state.result.scene.id, "evening");
    } finally {
      client.ws.close();
    }
  });

  step(app, "serves the browser source", async () => {
    const s = app.session;
    const sw = await s.execute(`return __e2e.toggle("^Serve a page for the OBS browser source")`);
    await sw.click();
    await waitFor(() => s.execute(`return __e2e.listItem("^Status$", "^Source on$") !== null`), {
      message: "the Source on badge in Status",
    });
    const url = await invoke(s, "source_url");
    const page = await fetch(url);
    assert.equal(page.status, 200);
    assert.match(page.headers.get("content-type") ?? "", /text\/html/);
    // ページと同じ口 (lib/feed.ts の framesUrl) でシーンとフレームを受ける
    const frames = new URL("frames", url);
    frames.protocol = "ws:";
    const ws = new WebSocket(frames);
    ws.binaryType = "arraybuffer";
    const got = { scene: 0, frames: 0 };
    ws.addEventListener("message", (e) => {
      if (typeof e.data === "string") got.scene++;
      else got.frames++;
    });
    try {
      await waitFor(() => got.scene > 0 && got.frames > 30, {
        timeout: 10_000,
        message: `scene and frames from the browser source (${JSON.stringify(got)})`,
      });
      await waitFor(
        () => s.execute(`return __e2e.byText("p, span, div", "^Open pages: 1$") !== null`),
        {
          message: "the open page count in the control window",
        },
      );
    } finally {
      ws.close();
    }
  });

  step(app, "saves a new scene to the profile and opens it again", async () => {
    const s = app.session;
    const add = await s.waitFindText(".scenes__actions button", "Add");
    await add.click();
    const path = profilePath;
    // 保存先のあるプロファイルは、変えた後に自動で保存される
    await waitFor(
      () => JSON.parse(readFileSync(path, "utf8")).scenes.some((sc) => sc.name === "Scene 5"),
      { message: "the new scene in the saved profile" },
    );
    await waitFor(() => s.execute(`return __e2e.byText(".studio__saved", "^Saved$") !== null`), {
      message: "the saved mark",
    });
    await invoke(s, "new_profile");
    await waitFor(
      () =>
        s.execute(
          `return document.querySelector(".studio__profile-name")?.textContent.trim() === "Untitled profile"`,
        ),
      { message: "a new profile" },
    );
    await invoke(s, "open_profile", { path });
    await s.waitFindText(".scenes__item .scenes__name", "Scene 5");
  });

  checkNoCspViolations(app);
});
