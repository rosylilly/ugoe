// 適合テストの `<ugoe-avatar>` (npm パッケージ @ugoe/runtime の要素) の実行器 (tests/conformance/README.md、T0440)。
//
// run.mjs のサーバーが配るページで動く。本物の wasm とヘッドレスの Chrome の WebGL2 (SwiftShader) で、要素の作成・読み込み・
// 切り離しと再接続・`backend` の変更・WebGL2 のコンテキストの喪失からの作り直し (復元の時間切れ・作れないときの試し直しと
// 諦め)・GPU プロセスの喪失からの作り直しを、シナリオごとに確かめる。
// 順序は bindings/web/src/avatar.test.ts が偽物で確かめているので、ここでは本物の GPU と wasm でも通ることを見る。
//
// - パッケージは /pkg/ (bindings/web/dist)、モデルは /samples/<名前> (assets/samples。`?delay=ミリ秒` で応答を遅らせる)。
// - `WebPuppet.create` と `free` を包んで、生きている `WebPuppet` を数える (GPU の資源が残らないことを確かめる)。
// - 描いたことは、要素の canvas (shadow root) を 2D の canvas に写して、背景でない画素があるかで確かめる。
// - GPU プロセスは、run.mjs のサーバーに /crash-gpu を送って落とさせる (CDP の `Browser.crashGpuProcess`。T0484)。
// - wasm の panic (`console_error_panic_hook` がコンソールに書く `panicked at`) を見たら、そのシナリオを失敗にする。
// - 結果はシナリオごとに /result/avatar.json に書き、/done で数を知らせる。

import "/pkg/element.js";
import { createPuppet, loadModel } from "/pkg/index.js";
import { WebPuppet } from "/pkg/wasm/ugoe_wasm.js";

const MODEL_A = "/samples/avatar.ugoe";
const MODEL_B = "/samples/dress-up/dress-up.ugoe";
/** 1 つの待ち (読み込み・作り直し) の上限 (ミリ秒)。SwiftShader の作成と読み込みは数秒かかることがある。 */
const WAIT_MS = 30_000;
/** 要素が WebGL2 の復元を待つ時間 (bindings/web/src/avatar.ts の `RESTORE_TIMEOUT_MS`。作れなかった作り直しを試し直すまでも同じ)。 */
const RESTORE_TIMEOUT_MS = 5_000;

// --- wasm の panic ---

/** 今のシナリオで見た wasm の panic (最初の行)。 */
const panics = [];
const consoleError = console.error;
console.error = (...args) => {
  const text = args.map(String).join(" ");
  if (text.includes("panicked at")) panics.push(text.split("\n")[0]);
  consoleError.apply(console, args);
};

// --- 生きている WebPuppet を数える ---

const live = new Set();
/** 作成中の `WebPuppet.create` の数。 */
let creating = 0;
/**
 * 0 より大きければ、`WebPuppet.create` の前に canvas の WebGL2 のコンテキストを失わせる (1 回ごとに 1 減らす)。本物の wasm が
 * `CreateError` (`context_unavailable`) で失敗する (T0475)。GPU が戻らずに作り直しで作れない場面の代わり。
 */
let loseContextsBeforeCreate = 0;
const create = WebPuppet.create;
WebPuppet.create = async function (...args) {
  if (loseContextsBeforeCreate > 0) {
    loseContextsBeforeCreate--;
    loseContextOf(args[0]);
  }
  creating++;
  try {
    const puppet = await create.apply(this, args);
    live.add(puppet);
    return puppet;
  } finally {
    creating--;
  }
};
const free = WebPuppet.prototype.free;
WebPuppet.prototype.free = function () {
  live.delete(this);
  return free.call(this);
};

// --- 補助 ---

async function post(path, body) {
  const response = await fetch(path, { method: "POST", body });
  if (!response.ok) throw new Error(`POST ${path}: ${response.status}`);
}

function log(message) {
  return post("/log", message);
}

function check(ok, message) {
  if (!ok) throw new Error(message);
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function frames(n = 3) {
  return new Promise((resolve) => {
    const step = () => (--n <= 0 ? resolve() : requestAnimationFrame(step));
    requestAnimationFrame(step);
  });
}

/** `condition()` が真になるまで待つ (`WAIT_MS` を過ぎるか wasm が panic したら失敗)。 */
async function until(condition, what) {
  const deadline = performance.now() + WAIT_MS;
  while (!condition()) {
    if (panics.length > 0) throw new Error(`wasm panicked while waiting for ${what}: ${panics[0]}`);
    if (performance.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await sleep(20);
  }
}

/** 要素が `type` を送るまで待つ (`error` を先に送ったらその理由で失敗)。 */
function once(el, type, what = type) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`timed out waiting for ${what}`));
    }, WAIT_MS);
    const onEvent = (e) => {
      cleanup();
      resolve(e);
    };
    const onError = (e) => {
      cleanup();
      reject(new Error(`error event while waiting for ${what}: ${e.detail?.error?.message}`));
    };
    function cleanup() {
      clearTimeout(timer);
      el.removeEventListener(type, onEvent);
      if (type !== "error") el.removeEventListener("error", onError);
    }
    el.addEventListener(type, onEvent);
    if (type !== "error") el.addEventListener("error", onError);
  });
}

/** 作成中のものが終わって (要素が古い作成の結果を捨てて) から、生きている `WebPuppet` が `n` 個かを確かめる。 */
async function expectLive(n, when) {
  const until = performance.now() + WAIT_MS;
  while (creating > 0 && performance.now() < until) await sleep(50);
  check(creating === 0, `${when}: WebPuppet.create did not finish`);
  // 作成が終わった後の `free` (要素の `#start` の続き) を待つ
  await sleep(100);
  check(live.size === n, `${when}: ${live.size} WebPuppet(s) alive, expected ${n}`);
}

function avatar(attributes) {
  const el = document.createElement("ugoe-avatar");
  for (const [k, v] of Object.entries(attributes)) el.setAttribute(k, v);
  return el;
}

/** canvas の WebGL2 のコンテキストを失わせる (`WEBGL_lose_context`)。復元させるための拡張を返す。 */
function loseContextOf(canvas) {
  // 2 回目の getContext は設定によらず同じコンテキストを返す
  const gl = canvas.getContext("webgl2", { antialias: false });
  check(gl !== null, "the canvas has no WebGL2 context");
  const ext = gl.getExtension("WEBGL_lose_context");
  check(ext !== null, "WEBGL_lose_context is not available");
  ext.loseContext();
  check(gl.isContextLost(), "the context is not lost");
  return ext;
}

/**
 * GPU プロセスを落とす (run.mjs が CDP の `Browser.crashGpuProcess` を送る)。`delay` (ミリ秒) を渡すと落とすのを待たずに戻り、
 * その後で落ちる (時機は決まらない)。渡さなければ落ちるまで待つ。
 */
function crashGpuProcess(delay) {
  return post("/crash-gpu", JSON.stringify({ delay }));
}

/** 新しい canvas に WebGL2 のコンテキストを作れて、失っていない (GPU プロセスが戻った) ことを待つ。 */
async function untilGpuIsBack() {
  await until(() => {
    const gl = document.createElement("canvas").getContext("webgl2");
    const ok = gl !== null && !gl.isContextLost();
    // 試しに作ったコンテキストは、Chrome の上限に数えられないようにすぐ手放す
    gl?.getExtension("WEBGL_lose_context")?.loseContext();
    return ok;
  }, "the GPU process to come back");
}

/** 要素が送った `error` が、wasm のコードを持つ例外 (`CreateError`・`LoadError`・`RenderError`) である。 */
function expectCodedError(error, when) {
  check(
    error instanceof Error &&
      ["CreateError", "LoadError", "RenderError"].includes(error.name) &&
      typeof error.code === "string",
    `${when}: an error without a code: ${error?.name}: ${error?.message}`,
  );
}

/** 要素の `error` と `gpulost` を数える (`stop()` で聞くのをやめる)。 */
function recordEvents(el) {
  const log = { errors: [], gpulost: 0 };
  const onError = (e) => log.errors.push(e.detail.error);
  const onLost = () => log.gpulost++;
  el.addEventListener("error", onError);
  el.addEventListener("gpulost", onLost);
  log.stop = () => {
    el.removeEventListener("error", onError);
    el.removeEventListener("gpulost", onLost);
  };
  return log;
}

function canvasOf(el) {
  const canvases = el.shadowRoot.querySelectorAll("canvas");
  check(canvases.length === 1, `the shadow root has ${canvases.length} canvases, expected 1`);
  return canvases[0];
}

/** モデルの見分け (描画するメッシュの ID の並び。要素を使わずに `WebPuppet` で読んで求める)。 */
async function modelSignature(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`GET ${url}: ${response.status}`);
  const canvas = document.createElement("canvas");
  const puppet = await createPuppet(canvas, { backend: "webgl2" });
  try {
    loadModel(puppet, await response.arrayBuffer());
    return puppet.drawable_ids().join(",");
  } finally {
    puppet.free();
  }
}

/** 要素が今読んでいるモデルの見分け。 */
function signature(el) {
  check(el.puppet?.has_model(), "the element has no model");
  return el.puppet.drawable_ids().join(",");
}

/** 要素の canvas に描いたか (背景でない画素の数が全体の 1% を超えるか)。 */
async function expectDrawn(el, when) {
  // 描き終えた後のフレームで写す (WebGL2 の描画バッファは、合成された後も Chrome では読める)
  let opaque = 0;
  let total = 0;
  for (let attempt = 0; attempt < 20; attempt++) {
    el.requestRender();
    await frames(2);
    const canvas = canvasOf(el);
    const copy = document.createElement("canvas");
    copy.width = canvas.width;
    copy.height = canvas.height;
    const context = copy.getContext("2d", { willReadFrequently: true });
    context.drawImage(canvas, 0, 0);
    const { data } = context.getImageData(0, 0, copy.width, copy.height);
    total = copy.width * copy.height;
    opaque = 0;
    for (let i = 3; i < data.length; i += 4) if (data[i] > 0) opaque++;
    if (opaque > total / 100) return;
  }
  throw new Error(`${when}: the canvas is empty (${opaque} of ${total} pixels drawn)`);
}

/** 値域の中で既定値と違う値を持つパラメータを 1 つ選ぶ。 */
function pickParameter(el) {
  const def = el.parameterDefinitions.find((d) => !d.internal && d.max > d.min);
  check(def, "the model has no adjustable parameter");
  const value = def.default === def.max ? def.min : def.max;
  return [def.id, value];
}

// --- シナリオ ---

/** 見本のモデルの見分け (`a` = MODEL_A、`b` = MODEL_B)。 */
const signatures = {};

const scenarios = {
  /** `src` を読み込んで `load` を送り、canvas に描く。外すと WebPuppet を解放する。 */
  async "load-and-draw"() {
    const el = avatar({ backend: "webgl2", src: MODEL_B });
    const loaded = once(el, "load");
    document.body.append(el);
    const event = await loaded;
    check(event.detail.backend === "webgl2", `backend is ${event.detail.backend}`);
    check(el.backend === "webgl2", `el.backend is ${el.backend}`);
    check((await el.ready) === el.puppet, "ready does not resolve to the puppet");
    check(signature(el) === signatures.b, "a different model after load");
    await expectDrawn(el, "after load");
    await expectLive(1, "after load");

    const next = once(el, "load");
    el.src = MODEL_A;
    await next;
    check(signature(el) === signatures.a, "a different model after changing src");
    await expectDrawn(el, "after changing src");
    await expectLive(1, "after changing src");

    el.remove();
    await expectLive(0, "after removing");
    check(el.puppet === null, "the puppet is kept after removing");
  },

  /** WebGL2 のコンテキストを失って戻ると、新しい canvas で作り直し、モデルとパラメータの値を戻す。 */
  async "context-loss"() {
    const el = avatar({ backend: "webgl2", src: MODEL_A });
    const loaded = once(el, "load");
    document.body.append(el);
    await loaded;
    const [id, value] = pickParameter(el);
    check(el.setParameter(id, value), `setParameter(${id}) failed`);
    await expectDrawn(el, "before losing the context");

    for (let round = 1; round <= 2; round++) {
      const old = canvasOf(el);
      const oldPuppet = el.puppet;
      const ext = old.getContext("webgl2")?.getExtension("WEBGL_lose_context");
      check(ext, "WEBGL_lose_context is not available");
      const errors = [];
      const onError = (e) => errors.push(e.detail.error.message);
      el.addEventListener("error", onError);
      check(!oldPuppet.is_lost(), `round ${round}: is_lost() before losing the context`);
      ext.loseContext();
      // 失っている間は作り直さない (復元を待つ) し、失ったコンテキストに描いて `error` を送らない
      await frames(5);
      // wasm も喪失を知らせる (T0932。wgpu の GL は知らせないので、WebPuppet が canvas のイベントを聞く)
      check(oldPuppet.is_lost(), `round ${round}: is_lost() is false after losing the context`);
      // 失った後の描画と読み戻しは `code` が `device_lost` の `RenderError` (T0465)
      for (const [name, call] of [
        ["render", () => oldPuppet.render()],
        ["read_pixels", () => oldPuppet.read_pixels()],
      ]) {
        let thrown = null;
        try {
          await call();
        } catch (e) {
          thrown = e;
        }
        check(
          thrown?.name === "RenderError" && thrown.code === "device_lost",
          `round ${round}: ${name} after losing the context threw ${thrown?.name}/${thrown?.code}`,
        );
      }
      check(el.puppet === oldPuppet, `round ${round}: recreated before the context was restored`);
      check(errors.length === 0, `round ${round}: error while lost: ${errors.join("; ")}`);
      const reloaded = once(el, "load", "the reload after restoring the context");
      ext.restoreContext();
      await reloaded;
      el.removeEventListener("error", onError);
      const canvas = canvasOf(el);
      check(canvas !== old, `round ${round}: the canvas was not replaced`);
      check(!old.isConnected, `round ${round}: the old canvas is still in the shadow root`);
      check(el.puppet !== oldPuppet, `round ${round}: the puppet was not recreated`);
      check(signature(el) === signatures.a, `round ${round}: a different model after restoring`);
      const got = el.getParameter(id);
      check(
        Math.abs(got - value) < 1e-6,
        `round ${round}: ${id} is ${got} after restoring, expected ${value}`,
      );
      check(errors.length === 0, `round ${round}: error while restoring: ${errors.join("; ")}`);
      await expectDrawn(el, `round ${round}: after restoring`);
      await expectLive(1, `round ${round}: after restoring`);
    }

    el.remove();
    await expectLive(0, "after removing");
  },

  /**
   * WebGL2 のコンテキストを失って復元が届かなければ、`gpulost` を送って待ち、`RESTORE_TIMEOUT_MS` の後に新しい canvas で
   * 作り直して、モデルとパラメータの値を戻して描く (T0938)。
   */
  async "context-loss-timeout"() {
    const el = avatar({ backend: "webgl2", src: MODEL_A });
    const loaded = once(el, "load");
    document.body.append(el);
    await loaded;
    const [id, value] = pickParameter(el);
    check(el.setParameter(id, value), `setParameter(${id}) failed`);
    await expectDrawn(el, "before losing the context");
    const old = canvasOf(el);
    const oldPuppet = el.puppet;
    const oldReady = el.ready;
    check(!el.waitingForGpu, "waitingForGpu before losing the context");
    const events = recordEvents(el);
    const lost = once(el, "gpulost");
    const lostAt = performance.now();
    loseContextOf(old);
    await lost;
    check(el.waitingForGpu, "waitingForGpu is false after gpulost");
    const ready = el.ready;
    check(ready !== oldReady, "ready was not replaced after gpulost");

    // 復元させない。時間切れまでは作り直さず、`error` も送らない
    const reloaded = once(el, "load", "the recreation after the restore timeout");
    await sleep(RESTORE_TIMEOUT_MS / 2);
    check(el.puppet === oldPuppet, "recreated before the restore timeout");
    check(canvasOf(el) === old, "the canvas was replaced before the restore timeout");
    check(el.waitingForGpu, "stopped waiting before the restore timeout");
    await reloaded;
    const waited = performance.now() - lostAt;
    await log(`context-loss-timeout: recreated ${waited.toFixed(0)} ms after the loss`);
    check(waited >= RESTORE_TIMEOUT_MS - 100, `recreated ${waited.toFixed(0)} ms after the loss`);
    events.stop();
    check(!el.waitingForGpu, "still waitingForGpu after the recreation");
    check(events.gpulost === 1, `gpulost was sent ${events.gpulost} times`);
    check(events.errors.length === 0, `error: ${events.errors.map((e) => e.message).join("; ")}`);
    check((await ready) === el.puppet, "ready does not resolve to the recreated puppet");
    const canvas = canvasOf(el);
    check(canvas !== old, "the canvas was not replaced");
    check(!old.isConnected, "the old canvas is still in the shadow root");
    check(el.puppet !== oldPuppet && !el.puppet.is_lost(), "the puppet was not recreated");
    check(signature(el) === signatures.a, "a different model after the recreation");
    const got = el.getParameter(id);
    check(Math.abs(got - value) < 1e-6, `${id} is ${got} after the recreation, expected ${value}`);
    await expectDrawn(el, "after the recreation");
    await expectLive(1, "after the recreation");
    el.remove();
    await expectLive(0, "after removing");
  },

  /**
   * 作り直しで作れなければ (GPU が戻らない)、`RESTORE_TIMEOUT_MS` ごとに試し直し、試し直しも作り直しの回数に数える。
   * 上限 (bindings/web/src/avatar.ts の `MAX_RESTARTS` = 3 回) で諦めて `error` を送り、`ready` を reject する。`restart()` で作り直して描く (T0938)。
   * 作れないことは、作成の前に新しい canvas のコンテキストを失わせて起こす (本物の wasm の `CreateError`。T0475)。
   */
  async "gpu-lost-repeatedly"() {
    const el = avatar({ backend: "webgl2", src: MODEL_A });
    const loaded = once(el, "load");
    document.body.append(el);
    await loaded;
    const [id, value] = pickParameter(el);
    check(el.setParameter(id, value), `setParameter(${id}) failed`);
    const events = recordEvents(el);

    // 1 回目の喪失: 復元は届くが作り直し (1 回目) で作れず、待って試し直し (2 回目) で作る
    loseContextsBeforeCreate = 1;
    const retried = once(el, "load", "the reload after retrying");
    let lost = once(el, "gpulost");
    let ext = loseContextOf(canvasOf(el));
    await lost;
    // `gpulost` は `webglcontextlost` の処理の中で送られるので、その処理が終わってから戻す (でなければ Chrome は復元を許さない)
    await frames(2);
    let restoredAt = performance.now();
    ext.restoreContext();
    await until(() => loseContextsBeforeCreate === 0 && creating === 0, "the failed recreation");
    check(el.puppet === null, "a puppet remains after the failed recreation");
    check(el.waitingForGpu, "not waitingForGpu after the failed recreation");
    check(events.errors.length === 0, "error after the failed recreation");
    await retried;
    let waited = performance.now() - restoredAt;
    await log(`gpu-lost-repeatedly: retried ${waited.toFixed(0)} ms after the restore`);
    check(waited >= RESTORE_TIMEOUT_MS - 100, `retried ${waited.toFixed(0)} ms after the restore`);
    check(!el.waitingForGpu, "still waitingForGpu after the retry");
    check(events.gpulost === 1, `gpulost was sent ${events.gpulost} times`);
    check(events.errors.length === 0, `error: ${events.errors.map((e) => e.message).join("; ")}`);
    check(signature(el) === signatures.a, "a different model after the retry");
    await expectDrawn(el, "after the retry");
    await expectLive(1, "after the retry");

    // 2 回目の喪失: 作り直し (3 回目 = 上限) で作れず、試し直す時に上限を超えるので諦める
    loseContextsBeforeCreate = 1;
    const failed = once(el, "error");
    lost = once(el, "gpulost");
    ext = loseContextOf(canvasOf(el));
    await lost;
    await frames(2);
    const waiting = el.ready;
    let rejected = null;
    waiting.catch((e) => {
      rejected = e;
    });
    restoredAt = performance.now();
    ext.restoreContext();
    const error = (await failed).detail.error;
    waited = performance.now() - restoredAt;
    await log(`gpu-lost-repeatedly: gave up ${waited.toFixed(0)} ms after the restore`);
    check(loseContextsBeforeCreate === 0, "the recreation after the second loss was not tried");
    check(waited >= RESTORE_TIMEOUT_MS - 100, `gave up ${waited.toFixed(0)} ms after the restore`);
    check(/lost repeatedly/.test(error.message), `error: ${error.message}`);
    await sleep(0);
    check(rejected === error, `ready was not rejected with the error (${rejected})`);
    check(events.gpulost === 2, `gpulost was sent ${events.gpulost} times`);
    check(events.errors.length === 1, `error was sent ${events.errors.length} times`);
    check(!el.waitingForGpu, "still waitingForGpu after giving up");
    check(el.puppet === null, "a puppet remains after giving up");
    check(
      el.shadowRoot.querySelectorAll("canvas").length === 0,
      "a canvas remains after giving up",
    );
    await expectLive(0, "after giving up");

    // `restart()` で作り直し (回数は数え直す)、モデルとパラメータの値を戻して描く
    const restarted = once(el, "load", "the reload after restart()");
    el.restart();
    await restarted;
    events.stop();
    check(events.errors.length === 1, `error was sent ${events.errors.length} times`);
    check((await el.ready) === el.puppet, "ready does not resolve to the restarted puppet");
    check(signature(el) === signatures.a, "a different model after restart()");
    const got = el.getParameter(id);
    check(Math.abs(got - value) < 1e-6, `${id} is ${got} after restart(), expected ${value}`);
    await expectDrawn(el, "after restart()");
    await expectLive(1, "after restart()");
    el.remove();
    await expectLive(0, "after removing");
  },

  /** 作成中に外して戻す (何度か) と、作成中の WebPuppet を捨て、最後の接続の 1 つだけが残る。 */
  async "detach-while-creating"() {
    const el = avatar({ backend: "webgl2", src: MODEL_A });
    const loaded = once(el, "load");
    document.body.append(el);
    el.remove();
    document.body.append(el);
    await sleep(0);
    el.remove();
    document.body.append(el);
    await loaded;
    await expectLive(1, "after reconnecting while creating");
    check(signature(el) === signatures.a, "a different model after reconnecting");
    check(el.shadowRoot.querySelectorAll("canvas").length === 1, "stale canvases are left");
    await expectDrawn(el, "after reconnecting while creating");

    // 外したまま作成を終えさせても、WebPuppet は残らない
    el.remove();
    document.body.append(el);
    el.remove();
    await expectLive(0, "after removing while creating");
  },

  /** 新しい `src` の取得中に外して戻すと、前のモデルではなく新しい `src` を描く。 */
  async "detach-while-fetching"() {
    const el = avatar({ backend: "webgl2", src: MODEL_A });
    const loaded = once(el, "load");
    document.body.append(el);
    await loaded;
    el.src = `${MODEL_B}?delay=1500`;
    await sleep(100);
    el.remove();
    await expectLive(0, "after removing while fetching");
    await sleep(100);
    const sources = [];
    const onLoad = () => sources.push(signature(el));
    el.addEventListener("load", onLoad);
    document.body.append(el);
    // 戻した直後は前のモデルを読み直さず、取得の結果を待つ
    const until = performance.now() + WAIT_MS;
    while (!sources.includes(signatures.b) && performance.now() < until) await sleep(50);
    el.removeEventListener("load", onLoad);
    check(sources.includes(signatures.b), "the new src was not loaded after reconnecting");
    check(
      sources.at(-1) === signatures.b,
      `the last model is not the new src (loads: ${sources.length})`,
    );
    await frames(5);
    check(signature(el) === signatures.b, "the model was replaced after loading the new src");
    await expectLive(1, "after reconnecting while fetching");
    await expectDrawn(el, "after reconnecting while fetching");
    el.remove();
    await expectLive(0, "after removing");
  },

  /** `backend` を変えると作り直し、パラメータの値を引き継ぐ。取得中に変えても、新しい `src` を描く。 */
  async "backend-change"() {
    const el = avatar({ backend: "webgl2", src: MODEL_A });
    const loaded = once(el, "load");
    document.body.append(el);
    await loaded;
    const [id, value] = pickParameter(el);
    el.setParameter(id, value);

    // `auto` は WebGPU を試し (このページの Chrome では使えないか、使えればそれで描く)、だめなら WebGL2 に戻る
    const recreated = once(el, "load", "the reload after changing backend");
    el.setAttribute("backend", "auto");
    await recreated;
    await log(`backend-change: auto drew with ${el.backend}`);
    check(signature(el) === signatures.a, "a different model after changing backend");
    const got = el.getParameter(id);
    check(
      Math.abs(got - value) < 1e-6,
      `${id} is ${got} after changing backend, expected ${value}`,
    );
    await expectLive(1, "after changing backend");
    await expectDrawn(el, `after changing backend (${el.backend})`);

    // 取得中に変える (作成中にもう一度変える)
    const sources = [];
    const onLoad = () => sources.push(signature(el));
    el.addEventListener("load", onLoad);
    el.src = `${MODEL_B}?delay=1500`;
    await sleep(100);
    el.setAttribute("backend", "webgl2");
    el.setAttribute("backend", "auto");
    await sleep(0);
    el.setAttribute("backend", "webgl2");
    const until = performance.now() + WAIT_MS;
    while (!sources.includes(signatures.b) && performance.now() < until) await sleep(50);
    el.removeEventListener("load", onLoad);
    check(sources.includes(signatures.b), "the new src was not loaded after changing backend");
    await frames(5);
    check(signature(el) === signatures.b, "the last model is not the new src");
    check(el.backend === "webgl2", `backend is ${el.backend}`);
    await expectLive(1, "after changing backend while fetching");
    await expectDrawn(el, "after changing backend while fetching");
    el.remove();
    await expectLive(0, "after removing");
  },
};

// GPU プロセスを落とすシナリオ (T0484)。Chrome が GPU プロセスを作り直すと、落ちる前の WebGL も影響を受けるので最後に動かす
Object.assign(scenarios, {
  /**
   * 描いている間に GPU プロセスが落ちたら、WebGL2 のコンテキストの喪失として `gpulost` を送って復元を待ち、新しい canvas で
   * 作り直して、モデルとパラメータの値を戻して描く (2 回続けて)。
   */
  async "gpu-process-crash"() {
    const el = avatar({ backend: "webgl2", src: MODEL_A });
    const loaded = once(el, "load");
    document.body.append(el);
    await loaded;
    const [id, value] = pickParameter(el);
    check(el.setParameter(id, value), `setParameter(${id}) failed`);
    await expectDrawn(el, "before the crash");
    const events = recordEvents(el);

    for (let round = 1; round <= 2; round++) {
      const old = canvasOf(el);
      const oldPuppet = el.puppet;
      const reloaded = once(el, "load", `round ${round}: the recreation after the crash`);
      const crashedAt = performance.now();
      await crashGpuProcess();
      await reloaded;
      await log(
        `gpu-process-crash: round ${round} recreated ${(performance.now() - crashedAt).toFixed(0)} ms after the crash`,
      );
      check(events.gpulost === round, `round ${round}: gpulost was sent ${events.gpulost} times`);
      check(
        events.errors.length === 0,
        `round ${round}: error: ${events.errors.map((e) => e.message).join("; ")}`,
      );
      check(!el.waitingForGpu, `round ${round}: still waitingForGpu after the recreation`);
      check((await el.ready) === el.puppet, `round ${round}: ready is not the recreated puppet`);
      const canvas = canvasOf(el);
      check(canvas !== old, `round ${round}: the canvas was not replaced`);
      check(!old.isConnected, `round ${round}: the old canvas is still in the shadow root`);
      check(
        el.puppet !== oldPuppet && !el.puppet.is_lost(),
        `round ${round}: the puppet was not recreated`,
      );
      check(signature(el) === signatures.a, `round ${round}: a different model after the crash`);
      const got = el.getParameter(id);
      check(
        Math.abs(got - value) < 1e-6,
        `round ${round}: ${id} is ${got} after the crash, expected ${value}`,
      );
      await expectDrawn(el, `round ${round}: after the crash`);
      await expectLive(1, `round ${round}: after the crash`);
      // 次の回が前の GPU プロセスの後始末と重ならないように待つ
      await untilGpuIsBack();
    }
    events.stop();
    el.remove();
    await expectLive(0, "after removing");
  },

  /**
   * 要素の作成・読み込みの途中に GPU プロセスが落ちても (サーバーの側から時機をずらして落とす)、wasm は panic せず、要素は
   * 作り直して描くか、コードを持つ例外 (`CreateError` など) の `error` を送って止まる。止まったら `restart()` で作り直して描く。
   * 落ちた時機は決まらない (どの段に当たるかは走るたびに変わる) ので、どう終わったかは数えて書くだけにする。
   */
  async "gpu-process-crash-during-create"() {
    const outcomes = new Map();
    for (const delay of [0, 20, 50, 100, 150, 200, 300, 500]) {
      const el = avatar({ backend: "webgl2", src: MODEL_A });
      const events = recordEvents(el);
      let loads = 0;
      el.addEventListener("load", () => loads++);
      await crashGpuProcess(delay);
      document.body.append(el);
      // 落ちてから (落とす命令が届くまでの余裕を見る)、GPU プロセスが戻るのを待つ
      await sleep(delay + 300);
      await untilGpuIsBack();
      // 落ちたことが要素に届いて、作り直しか失敗で落ち着くのを待つ
      await frames(5);
      await until(
        () =>
          creating === 0 &&
          !el.waitingForGpu &&
          (el.puppet?.has_model() || (el.puppet === null && events.errors.length > 0)),
        `delay ${delay}: the element to settle`,
      );
      for (const error of events.errors) expectCodedError(error, `delay ${delay}`);
      let outcome;
      if (el.puppet === null) {
        // 作成で失敗して止まった。`restart()` で作り直す
        outcome = `failed (${events.errors.map((e) => `${e.name}/${e.code}`).join(", ")}), restarted`;
        const restarted = once(el, "load", `delay ${delay}: the reload after restart()`);
        el.restart();
        await restarted;
      } else {
        outcome =
          events.gpulost > 0
            ? `recreated (gpulost x${events.gpulost}, load x${loads})`
            : `not affected (load x${loads})`;
        if (events.errors.length > 0)
          outcome += ` with ${events.errors.map((e) => `${e.name}/${e.code}`).join(", ")}`;
      }
      events.stop();
      outcomes.set(outcome, (outcomes.get(outcome) ?? 0) + 1);
      check(signature(el) === signatures.a, `delay ${delay}: a different model`);
      await expectDrawn(el, `delay ${delay}`);
      await expectLive(1, `delay ${delay}`);
      el.remove();
      await expectLive(0, `delay ${delay}: after removing`);
    }
    await log(
      `gpu-process-crash-during-create: ${[...outcomes].map(([k, v]) => `${k} x${v}`).join("; ")}`,
    );
  },
});

async function main() {
  signatures.a = await modelSignature(MODEL_A);
  signatures.b = await modelSignature(MODEL_B);
  if (signatures.a === signatures.b) throw new Error("the two sample models look the same");
  const results = {};
  let failed = 0;
  for (const [name, run] of Object.entries(scenarios)) {
    await post("/start", name);
    panics.length = 0;
    try {
      try {
        await run();
      } finally {
        // 次のシナリオに前の要素を持ち込まない。外した後に残った WebPuppet はこのシナリオの失敗にする
        for (const el of document.querySelectorAll("ugoe-avatar")) el.remove();
      }
      await expectLive(0, `after ${name}`);
      check(panics.length === 0, `wasm panicked: ${panics[0]}`);
      results[name] = "ok";
    } catch (e) {
      failed += 1;
      results[name] = String(e?.message ?? e);
      await log(`${name}: ${e?.stack ?? e}`);
      // 残ったものを次のシナリオで数えない
      live.clear();
    }
  }
  await post("/result/avatar.json", JSON.stringify(results, null, 2));
  await post("/done", JSON.stringify({ total: Object.keys(results).length, failed }));
}

main().catch((e) => post("/done", JSON.stringify({ error: String(e?.stack ?? e) })));
