// PuppetSurface のブラウザのテストのシナリオ (run.mjs が配るページで動く。T0936)。
//
// 本物の wasm (/wasm/ugoe_wasm.js) とヘッドレスの Chrome の WebGL2 (SwiftShader) で、PuppetSurface (/surface.js。
// src/surface.ts を JS にしたもの) の作成・描画・作り直し (WebGL2 の喪失で復元を待つ。T0935)・諦め・破棄を確かめる。
// 順序の細部は src/surface.test.ts が偽の wasm で確かめているので、ここでは本物の GPU と wasm でも通ることを見る。
//
// - `WebPuppet.create` と `free` を包んで、生きている WebPuppet を数える (GPU の資源が残らないことを確かめる)。
// - 描いたことは、描いた直後に canvas を 2D の canvas に写して、透明でない画素があるかで確かめる (背景は透明)。
// - コンテキストの喪失と復元は `WEBGL_lose_context` の `loseContext`・`restoreContext` で起こす。
// - `window.runScenarios(names)` が結果 ([{name, ok, error}]) を返す (names が null ならすべて)。

import { GpuLostRepeatedlyError, MAX_RESTARTS, PuppetSurface } from "/surface.js";
import init, { WebPuppet } from "/wasm/ugoe_wasm.js";

const MODEL = "/samples/avatar.ugoe";
/** 1 つの待ち (作成・作り直し) の上限 (ミリ秒)。SwiftShader の作成は数秒かかることがある。 */
const WAIT_MS = 30_000;

// --- 生きている WebPuppet を数える ---

const live = new Set();
/** 次の `WebPuppet.create` を呼んだ直後 (できるのを待つ前) に一度だけ呼ぶ (作っている間の `detach` を確かめるため)。 */
let onCreating = null;
const create = WebPuppet.create;
WebPuppet.create = async function (...args) {
  const pending = create.apply(this, args);
  const hook = onCreating;
  onCreating = null;
  hook?.();
  const puppet = await pending;
  live.add(puppet);
  return puppet;
};
const free = WebPuppet.prototype.free;
WebPuppet.prototype.free = function () {
  live.delete(this);
  return free.call(this);
};

// --- 補助 ---

function check(ok, message) {
  if (!ok) throw new Error(message);
}

function frames(n = 2) {
  return new Promise((resolve) => {
    const step = () => (--n <= 0 ? resolve() : requestAnimationFrame(step));
    requestAnimationFrame(step);
  });
}

/** `condition()` が真になるまでフレームを待つ。 */
async function until(condition, what) {
  const start = performance.now();
  while (!condition()) {
    if (performance.now() - start > WAIT_MS) throw new Error(`timed out waiting for ${what}`);
    await frames(1);
  }
}

/** シナリオが作った PuppetSurface (失敗したシナリオの後にも捨てるため)。 */
const surfaces = [];

let model = null;
async function modelBytes() {
  if (model === null) {
    const response = await fetch(MODEL);
    check(response.ok, `GET ${MODEL}: ${response.status}`);
    model = new Uint8Array(await response.arrayBuffer());
  }
  return model;
}

/** 描く領域 (PuppetSurface の container) を作る。 */
function stage() {
  const el = document.createElement("div");
  el.className = "stage";
  document.body.append(el);
  return el;
}

/**
 * PuppetSurface と、ホストへの知らせの記録。`created` でモデルを読んで描く (Player・ViewportController と同じ流れ)。
 * `failed` は記録するだけ。
 */
async function surfaceWithHost() {
  const bytes = await modelBytes();
  const log = { created: [], failed: [], lost: 0, released: 0 };
  const surface = new PuppetSurface({
    loadWasm: async () => ({ WebPuppet }),
    created(puppet, recovered) {
      puppet.load_bundle(bytes);
      puppet.fit();
      puppet.render();
      log.created.push({ puppet, recovered });
    },
    failed(error) {
      log.failed.push(error);
    },
    lost() {
      log.lost++;
    },
    released() {
      log.released++;
    },
    canvasClass: "surface",
  });
  surface.resize(160, 120);
  surfaces.push(surface);
  return { surface, log };
}

/** 描いて、その直後に canvas の透明でない画素の割合を数える (描画バッファが合成で消える前に写す)。 */
function drawnFraction(surface, container) {
  const canvas = canvasesIn(container)[0];
  surface.puppet.render();
  const copy = document.createElement("canvas");
  copy.width = canvas.width;
  copy.height = canvas.height;
  const context = copy.getContext("2d", { willReadFrequently: true });
  context.drawImage(canvas, 0, 0);
  const { data } = context.getImageData(0, 0, copy.width, copy.height);
  let opaque = 0;
  for (let i = 3; i < data.length; i += 4) if (data[i] > 0) opaque++;
  return opaque / (copy.width * copy.height);
}

function expectDrawn(surface, container, when) {
  const fraction = drawnFraction(surface, container);
  check(fraction > 0.01, `${when}: the canvas is empty (${(fraction * 100).toFixed(2)}% drawn)`);
}

function canvasesIn(container) {
  return [...container.querySelectorAll("canvas")];
}

/** 今の canvas の WebGL2 のコンテキストを失わせる。復元に使う拡張を返す (失った後は取り直せない)。 */
async function loseContext(container) {
  const canvases = canvasesIn(container);
  check(canvases.length === 1, `expected one canvas, found ${canvases.length}`);
  // wgpu が作ったコンテキストがそのまま返る (同じ canvas の 2 回目の getContext)
  const gl = canvases[0].getContext("webgl2");
  check(gl !== null, "the canvas has no WebGL2 context");
  const ext = gl.getExtension("WEBGL_lose_context");
  check(ext !== null, "WEBGL_lose_context is not available");
  ext.loseContext();
  return ext;
}

/** 失わせて、復元を待つ状態になったことを確かめてから復元し、作り直されるのを待つ。作り直した WebPuppet を返す。 */
async function loseAndRestore(surface, container, log) {
  const before = surface.puppet;
  const createdBefore = log.created.length;
  const ext = await loseContext(container);
  await until(() => surface.awaitingRestore, "webglcontextlost");
  check(before.is_lost(), "is_lost() is false after the context was lost");
  check(surface.checkLost(), "checkLost() is false while awaiting the restore");
  // 復元を待っている間は作り直さない (フレームを進めても同じ WebPuppet のまま)
  await frames(3);
  check(surface.puppet === before, "recreated before the context was restored");
  check(log.created.length === createdBefore, "created was called before the restore");
  ext.restoreContext();
  await until(
    () => (surface.puppet !== before && !surface.recovering) || log.failed.length > 0,
    "the recreation after webglcontextrestored",
  );
  return surface.puppet;
}

// --- シナリオ ---

const scenarios = {
  /** 作って描き、大きさを変え、捨てる。 */
  async "create-draw-detach"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    const result = await surface.attach(container);
    check(result === "ready", `attach returned ${result}`);
    check(log.created.length === 1 && !log.created[0].recovered, "created was not called once");
    check(surface.puppet?.backend() === "webgl2", `backend is ${surface.puppet?.backend()}`);
    check(surface.puppet.has_model(), "the model is not loaded");
    const canvases = canvasesIn(container);
    check(canvases.length === 1, `expected one canvas, found ${canvases.length}`);
    check(canvases[0].classList.contains("surface"), "canvasClass is not applied");
    expectDrawn(surface, container, "after attach");

    surface.resize(200, 100);
    const ratio = surface.pixelRatio;
    check(
      surface.puppet.width() === Math.round(200 * ratio) &&
        surface.puppet.height() === Math.round(100 * ratio),
      `resize: the puppet is ${surface.puppet.width()}x${surface.puppet.height()}`,
    );
    expectDrawn(surface, container, "after resize");

    surface.detach();
    check(surface.puppet === null, "the puppet remains after detach");
    check(canvasesIn(container).length === 0, "the canvas remains after detach");
    check(log.released === 1, `released was called ${log.released} times`);
    check(log.failed.length === 0, `failed: ${log.failed[0]}`);
  },

  /** WebGL2 のコンテキストを失ったら復元を待ち、復元されたら新しい canvas で作り直して描く (2 回続けて)。 */
  async "context-loss-restore"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    check((await surface.attach(container)) === "ready", "attach failed");
    for (let round = 1; round <= 2; round++) {
      const oldCanvas = canvasesIn(container)[0];
      const puppet = await loseAndRestore(surface, container, log);
      check(log.failed.length === 0, `round ${round}: failed: ${log.failed[0]}`);
      check(puppet !== null, `round ${round}: no puppet after the restore`);
      check(log.lost === round, `round ${round}: lost was called ${log.lost} times`);
      const last = log.created.at(-1);
      check(log.created.length === round + 1 && last.recovered, `round ${round}: not recovered`);
      check(
        puppet.has_model() && !puppet.is_lost(),
        `round ${round}: the new puppet is not usable`,
      );
      const canvases = canvasesIn(container);
      check(canvases.length === 1, `round ${round}: ${canvases.length} canvases`);
      check(canvases[0] !== oldCanvas, `round ${round}: the canvas was not replaced`);
      check(live.size === 1, `round ${round}: ${live.size} live puppets`);
      expectDrawn(surface, container, `round ${round}`);
    }
    surface.detach();
    check(canvasesIn(container).length === 0, "the canvas remains after detach");
  },

  /** 続けて失うと `MAX_RESTARTS` 回まで作り直し、次で諦める。次の `attach` で数え直して作れる。 */
  async "repeated-loss-gives-up"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    check((await surface.attach(container)) === "ready", "attach failed");
    for (let round = 1; round <= MAX_RESTARTS; round++) {
      await loseAndRestore(surface, container, log);
      check(log.failed.length === 0, `round ${round}: gave up too early`);
    }
    await loseAndRestore(surface, container, log);
    check(log.failed.length === 1, `failed was called ${log.failed.length} times`);
    check(log.failed[0] instanceof GpuLostRepeatedlyError, `failed with ${log.failed[0]}`);
    check(surface.puppet === null, "the puppet remains after giving up");
    check(canvasesIn(container).length === 0, "the canvas remains after giving up");
    check(live.size === 0, `${live.size} live puppets after giving up`);

    check((await surface.attach(container)) === "ready", "attach after giving up failed");
    expectDrawn(surface, container, "after attaching again");
    surface.detach();
  },

  /** 作っている間の `detach` と、続けての `attach`。遅れてできた WebPuppet と canvas は残らない。 */
  async "detach-while-creating"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    // WebPuppet.create を待っている間に捨てる
    onCreating = () => surface.detach();
    const first = await surface.attach(container);
    check(first === "superseded", `the detached attach returned ${first}`);
    check(canvasesIn(container).length === 0, "the canvas of the detached attach remains");
    check(live.size === 0, `${live.size} live puppets after the detached attach`);

    // WebPuppet.create を待っている間に次の attach が来る
    let third = null;
    onCreating = () => {
      third = surface.attach(container);
    };
    const second = surface.attach(container);
    check((await second) === "superseded", "the replaced attach was not superseded");
    check(third !== null && (await third) === "ready", "the last attach failed");
    check(canvasesIn(container).length === 1, `${canvasesIn(container).length} canvases`);
    check(live.size === 1, `${live.size} live puppets`);
    check(log.created.length === 1, `created was called ${log.created.length} times`);
    surface.detach();
  },
};

window.runScenarios = async (names) => {
  await init();
  const results = [];
  for (const [name, run] of Object.entries(scenarios)) {
    if (names && !names.includes(name)) continue;
    try {
      await run();
      // シナリオの後に WebPuppet が残っていない (GPU の資源を捨てた)
      check(live.size === 0, `${live.size} live puppets after the scenario`);
      results.push({ name, ok: true });
    } catch (e) {
      results.push({ name, ok: false, error: e instanceof Error ? e.message : String(e) });
    } finally {
      onCreating = null;
      for (const surface of surfaces.splice(0)) surface.detach();
      for (const puppet of live) puppet.free();
      document.body.replaceChildren();
    }
  }
  return results;
};
