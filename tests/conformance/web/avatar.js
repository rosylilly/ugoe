// 適合テストの `<ugoe-avatar>` (npm パッケージ @ugoe/runtime の要素) の実行器 (tests/conformance/README.md、T0440)。
//
// run.mjs のサーバーが配るページで動く。本物の wasm とヘッドレスの Chrome の WebGL2 (SwiftShader) で、要素の作成・読み込み・
// 切り離しと再接続・`backend` の変更・WebGL2 のコンテキストの喪失からの作り直しを、シナリオごとに確かめる。
// 順序は bindings/web/src/avatar.test.ts が偽物で確かめているので、ここでは本物の GPU と wasm でも通ることを見る。
//
// - パッケージは /pkg/ (bindings/web/dist)、モデルは /samples/<名前> (assets/samples。`?delay=ミリ秒` で応答を遅らせる)。
// - `WebPuppet.create` と `free` を包んで、生きている `WebPuppet` を数える (GPU の資源が残らないことを確かめる)。
// - 描いたことは、要素の canvas (shadow root) を 2D の canvas に写して、背景でない画素があるかで確かめる。
// - 結果はシナリオごとに /result/avatar.json に書き、/done で数を知らせる。

import "/pkg/element.js";
import { createPuppet, loadModel } from "/pkg/index.js";
import { WebPuppet } from "/pkg/wasm/ugoe_wasm.js";

const MODEL_A = "/samples/avatar.ugoe";
const MODEL_B = "/samples/dress-up/dress-up.ugoe";
/** 1 つの待ち (読み込み・作り直し) の上限 (ミリ秒)。SwiftShader の作成と読み込みは数秒かかることがある。 */
const WAIT_MS = 30_000;

// --- 生きている WebPuppet を数える ---

const live = new Set();
/** 作成中の `WebPuppet.create` の数。 */
let creating = 0;
const create = WebPuppet.create;
WebPuppet.create = async function (...args) {
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

async function main() {
  signatures.a = await modelSignature(MODEL_A);
  signatures.b = await modelSignature(MODEL_B);
  if (signatures.a === signatures.b) throw new Error("the two sample models look the same");
  const results = {};
  let failed = 0;
  for (const [name, run] of Object.entries(scenarios)) {
    await post("/start", name);
    try {
      try {
        await run();
      } finally {
        // 次のシナリオに前の要素を持ち込まない。外した後に残った WebPuppet はこのシナリオの失敗にする
        for (const el of document.querySelectorAll("ugoe-avatar")) el.remove();
      }
      await expectLive(0, `after ${name}`);
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
