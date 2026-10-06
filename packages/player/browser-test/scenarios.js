// PuppetSurface と Player のブラウザのテストのシナリオ (run.mjs が配るページで動く。T0936・T0939・T0938・T0944)。
//
// 本物の wasm (/wasm/ugoe_wasm.js) とヘッドレスの Chrome の WebGL2 (SwiftShader) で、PuppetSurface (/player.js。
// @ugoe/player を Vite でまとめたもの) の作成・描画・作り直し (WebGL2 の喪失で復元を待つ。T0935。復元が届かなければ
// 時間切れで作り直す。T0938)・諦めと `restart`・破棄・devicePixelRatio の追従を確かめる。Player は、作り直しと
// 「作り直す」(`restart`) でモデル・モジュール・値を戻すことと、倍率が変わったときのビューと読み戻しと、ポーズの欄が
// モーションに追従すること (T0736) を確かめる (`player-*`)。順序の細部は src/surface.test.ts・player*.test.ts が偽の wasm で
// 確かめているので、ここでは本物の GPU と wasm でも通ることを見る。
//
// - `WebPuppet.create` と `free` を包んで、生きている WebPuppet を数える (GPU の資源が残らないことを確かめる)。
// - 描いたことは、描いた直後に canvas を 2D の canvas に写して、透明でない画素があるかで確かめる (背景は透明)。
// - コンテキストの喪失と復元は `WEBGL_lose_context` の `loseContext`・`restoreContext` で起こす。
// - GPU プロセスごと失うのは、run.mjs が出す `crashGpuProcess` (CDP の `Browser.crashGpuProcess`) で起こす (T0944)。
//   作成の途中の決まった時機での喪失は、WebGL2 の呼び出しを数えて、N 回目の前に `loseContext` を呼んで起こす。
// - wasm の panic (`console_error_panic_hook` がコンソールに出す `panicked at`) が 1 つでもあれば、そのシナリオは失敗する。
// - devicePixelRatio だけの変化は、run.mjs が出す `setDevicePixelRatio` (CDP の `Emulation.setDeviceMetricsOverride`) で起こす (T0939)。
// - `window.runScenarios(names)` が結果 ([{name, ok, error}]) を返す (names が null ならすべて)。

import {
  GpuLostRepeatedlyError,
  MAX_RESTARTS,
  Player,
  PuppetSurface,
  RESTORE_TIMEOUT_MS,
} from "/player.js";
import init, { WebPuppet } from "/wasm/ugoe_wasm.js";

const MODEL = "/samples/avatar.ugoe";
/** 着せ替えの見本 (Player が作り直しでモジュールを付け直すことを確かめる)。 */
const DRESS_UP = "/samples/dress-up/dress-up.ugoe";
const DRESS_UP_MODULE = "/samples/dress-up/cat-ears.ugoemod";
/** Player が覚えるものの localStorage の接頭辞 (シナリオの後に消す)。 */
const STORAGE_PREFIX = "player-browser-test";
/** 1 つの待ち (作成・作り直し) の上限 (ミリ秒)。SwiftShader の作成は数秒かかることがある。 */
const WAIT_MS = 30_000 + RESTORE_TIMEOUT_MS;

// --- 生きている WebPuppet を数える ---

const live = new Set();
/** 次の `WebPuppet.create` を呼んだ直後 (できるのを待つ前) に一度だけ呼ぶ (作っている間の `detach` を確かめるため)。 */
let onCreating = null;
/**
 * これから呼ぶ `WebPuppet.create` のうち、この回数だけ、渡す canvas の WebGL2 のコンテキストを先に作って失わせておく
 * (Chrome が失ったコンテキストを返すとき (描画バッファを作れない・上限に当たったなど) と同じ状態。T0475)。
 */
let loseContextsBeforeCreate = 0;
/**
 * `WebPuppet.create` ができてから返すまでに待つ時間 (ミリ秒)。CI の SwiftShader のように作成が遅いときを再現する
 * (作り直しの途中の、新しい canvas はあるが WebPuppet はまだ無い間を数フレーム続ける。T0960)。
 */
let createDelayMs = 0;
/**
 * WebGL2 の呼び出し (WebGL2RenderingContext のプロトタイプのメソッド。wasm-bindgen のグルーはその場でメソッドを引くので、
 * 包めば数えられる) の数。`loseAtCall` 回目の呼び出しの前に、そのコンテキストを失わせる (作成の途中の喪失。T0944)。
 * `invalidBlockIndex` なら `getUniformBlockIndex` が `INVALID_INDEX` を返す (GPU プロセスが落ちた直後の Chrome と同じ。
 * そのとき `isContextLost()` はまだ偽。`loseContext` では起きない)。
 */
const glCalls = { count: 0, loseAtCall: 0, invalidBlockIndex: false };
for (const name of Object.getOwnPropertyNames(WebGL2RenderingContext.prototype)) {
  const original = Object.getOwnPropertyDescriptor(WebGL2RenderingContext.prototype, name).value;
  if (typeof original !== "function" || name === "constructor") continue;
  WebGL2RenderingContext.prototype[name] = function (...args) {
    if (++glCalls.count === glCalls.loseAtCall) {
      WebGL2RenderingContext.prototype.getExtension.call(this, "WEBGL_lose_context")?.loseContext();
    }
    const result = original.apply(this, args);
    return name === "getUniformBlockIndex" && glCalls.invalidBlockIndex ? 0xffffffff : result;
  };
}

// --- wasm の panic ---

/** 今のシナリオで見た panic (`console_error_panic_hook` の `console.error`)。 */
const panics = [];
/** panic を待つもの (`within`。panic の後は作成の Promise が決着しないので、時間切れを待たずに失敗する)。 */
const panicWaiters = new Set();
const consoleError = console.error;
console.error = (...args) => {
  const text = args.map(String).join(" ");
  if (text.includes("panicked at")) {
    panics.push(text.split("\n")[0]);
    for (const reject of panicWaiters) reject(new Error(`wasm panicked: ${panics.at(-1)}`));
  }
  consoleError.apply(console, args);
};

const create = WebPuppet.create;
WebPuppet.create = async function (...args) {
  if (loseContextsBeforeCreate > 0) {
    loseContextsBeforeCreate--;
    loseContextOf(args[0]);
  }
  const pending = create.apply(this, args);
  const hook = onCreating;
  onCreating = null;
  hook?.();
  const puppet = await pending;
  if (createDelayMs > 0) await new Promise((resolve) => setTimeout(resolve, createDelayMs));
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

const files = new Map();
async function fileBytes(path) {
  if (!files.has(path)) {
    const response = await fetch(path);
    check(response.ok, `GET ${path}: ${response.status}`);
    files.set(path, new Uint8Array(await response.arrayBuffer()));
  }
  return files.get(path);
}

function modelBytes() {
  return fileBytes(MODEL);
}

/** シナリオが作った Player (失敗したシナリオの後にも捨てるため)。 */
const players = [];

/** 本物の wasm で動く Player (ビューア・スタジオと同じ。ファイルの読み直しは使わない)。 */
function newPlayer() {
  const player = new Player({
    loadWasm: async () => ({ WebPuppet }),
    readFile: () => Promise.reject(new Error("no files")),
    storagePrefix: STORAGE_PREFIX,
  });
  player.resize(160, 120);
  players.push(player);
  return player;
}

/** `player.capture()` (`read_pixels`) の透明でない画素の割合 (背景を透明にして呼ぶ)。 */
async function capturedFraction(player) {
  const shot = await player.capture();
  check(shot !== null, "capture returned null");
  let opaque = 0;
  for (let i = 3; i < shot.pixels.length; i += 4) if (shot.pixels[i] > 0) opaque++;
  return { shot, fraction: opaque / (shot.width * shot.height) };
}

async function expectCaptured(player, when) {
  const { fraction } = await capturedFraction(player);
  check(fraction > 0.01, `${when}: the capture is empty (${(fraction * 100).toFixed(2)}% drawn)`);
}

function near(a, b, epsilon = 1e-3) {
  return Math.abs(a - b) <= epsilon * Math.max(1, Math.abs(a), Math.abs(b));
}

/** Player の `view` (CSS ピクセル) が `expected` と同じか。 */
function isView(view, expected) {
  return (
    view !== null &&
    expected !== null &&
    near(view.scale, expected.scale) &&
    near(view.x, expected.x) &&
    near(view.y, expected.y)
  );
}

/** 失敗の文に出す `view`。 */
function describeView(view) {
  return view === null ? "null" : `{scale: ${view.scale}, x: ${view.x}, y: ${view.y}}`;
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
  const log = { created: [], failed: [], lost: 0, waiting: 0, released: 0, pixelRatioChanged: 0 };
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
    waiting() {
      log.waiting++;
    },
    released() {
      log.released++;
    },
    pixelRatioChanged() {
      log.pixelRatioChanged++;
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

/**
 * Player が失ったコンテキストの後に作り直し終えるのを待つ (諦めて `unavailable` になったときも返る)。作り直しの途中も
 * `status` は前のモデルの `ready` のままで、新しい canvas は WebPuppet ができる前に置かれるので、`waitingForGpu` が
 * 下りる (`created`) まで待つ (T0960)。
 */
async function untilPlayerRecreated(player, container, oldCanvas, what) {
  await until(() => {
    const canvases = canvasesIn(container);
    return (
      (!player.waitingForGpu &&
        canvases.length === 1 &&
        canvases[0] !== oldCanvas &&
        player.status === "ready") ||
      player.status === "unavailable"
    );
  }, what);
}

/** devicePixelRatio を変える (run.mjs の CDP)。ページに届くまで待つ。 */
async function setPixelRatio(ratio) {
  await window.setDevicePixelRatio(ratio);
  await until(() => devicePixelRatio === ratio, `devicePixelRatio ${ratio}`);
}

/** WebPuppet と canvas の描画バッファが、CSS の大きさ × `ratio` になっている。 */
function expectBufferSize(surface, container, [width, height], ratio, when) {
  const expected = [Math.round(width * ratio), Math.round(height * ratio)];
  const puppet = [surface.puppet.width(), surface.puppet.height()];
  const canvas = canvasesIn(container)[0];
  check(
    puppet[0] === expected[0] && puppet[1] === expected[1],
    `${when}: the puppet is ${puppet.join("x")}, expected ${expected.join("x")}`,
  );
  check(
    canvas.width === expected[0] && canvas.height === expected[1],
    `${when}: the canvas is ${canvas.width}x${canvas.height}, expected ${expected.join("x")}`,
  );
}

/**
 * `canvas` の WebGL2 のコンテキストを (無ければ wgpu と同じ設定で作って) 失わせる。復元に使う拡張を返す
 * (失った後は取り直せない)。
 */
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
 * `promise` を待つ。`WAIT_MS` を過ぎるか wasm が panic したら失敗する (wasm が panic すると、作成の Promise は
 * 決着しない)。
 */
function within(promise, what) {
  let timer;
  let waiter;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`timed out waiting for ${what}`)), WAIT_MS);
      waiter = reject;
      panicWaiters.add(waiter);
    }),
  ]).finally(() => {
    clearTimeout(timer);
    panicWaiters.delete(waiter);
  });
}

/** GPU プロセスを落として、新しい GPU プロセスで WebGL2 を作れるようになるまで待つ (T0944)。 */
async function crashGpuProcessAndWait() {
  await window.crashGpuProcess();
  await untilGpuIsBack();
}

/** 新しい canvas の WebGL2 のコンテキストを作れて、失っていない状態になるまで待つ。 */
async function untilGpuIsBack() {
  await until(() => {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl2");
    const ok = gl !== null && !gl.isContextLost();
    gl?.getExtension("WEBGL_lose_context")?.loseContext();
    return ok;
  }, "the GPU process to come back");
}

/**
 * `WebPuppet.create` で作ったものを読み込んで描き、大きさを変えて描く (どこかで GPU を失ってもよい)。失敗はコードを持つ
 * 例外 (`CreateError`・`LoadError`・`RenderError`) だけを許し、何が起きたかを返す。
 */
async function createLoadRender(canvas, bytes) {
  let puppet;
  try {
    puppet = await within(WebPuppet.create(canvas, "webgl2"), "create");
  } catch (e) {
    check(
      e instanceof Error && e.name === "CreateError" && typeof e.code === "string",
      `create failed without a CreateError: ${e?.name}: ${e}`,
    );
    return `create failed (${e.code})`;
  }
  try {
    puppet.load_bundle(bytes);
    puppet.fit();
    puppet.render();
    puppet.resize(100, 80);
    puppet.render();
    return "drawn";
  } catch (e) {
    check(
      e instanceof Error && typeof e.code === "string",
      `using the puppet failed without a code: ${e?.name}: ${e}`,
    );
    return `${e.name} (${e.code})`;
  } finally {
    puppet.free();
  }
}

/** 作成の失敗が、コンテキストを作れなかったことを示す `CreateError` である (T0475)。 */
function expectContextUnavailable(error, when) {
  check(
    error instanceof Error && error.name === "CreateError" && error.code === "context_unavailable",
    `${when}: expected a CreateError with code context_unavailable, got ${error?.name} ${error?.code}: ${error}`,
  );
}

/** 今の canvas の WebGL2 のコンテキストを失わせる。復元に使う拡張を返す (失った後は取り直せない)。 */
async function loseContext(container) {
  const canvases = canvasesIn(container);
  check(canvases.length === 1, `expected one canvas, found ${canvases.length}`);
  // wgpu が作ったコンテキストがそのまま返る (同じ canvas の 2 回目の getContext)
  return loseContextOf(canvases[0]);
}

/** 失わせて、復元を待つ状態になったことを確かめてから復元し、作り直されるのを待つ。作り直した WebPuppet を返す。 */
async function loseAndRestore(surface, container, log) {
  const before = surface.puppet;
  const createdBefore = log.created.length;
  const failedBefore = log.failed.length;
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
    () => (surface.puppet !== before && !surface.recovering) || log.failed.length > failedBefore,
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

  /**
   * devicePixelRatio だけが変わったら (ResizeObserver は知らせない)、`matchMedia` の `change` で描画バッファを新しい倍率に
   * 合わせ直して `pixelRatioChanged` を知らせる。続けて変わっても追い、`detach` の後は聞かない (T0939)。
   */
  async "pixel-ratio-change"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    try {
      check((await surface.attach(container)) === "ready", "attach failed");
      check(surface.pixelRatio === 1, `the initial ratio is ${surface.pixelRatio}`);
      expectBufferSize(surface, container, [160, 120], 1, "after attach");
      let changes = 0;
      for (const ratio of [2, 1.5, 1]) {
        await setPixelRatio(ratio);
        changes++;
        await until(
          () => log.pixelRatioChanged === changes,
          `pixelRatioChanged after devicePixelRatio became ${ratio}`,
        );
        check(surface.pixelRatio === ratio, `ratio ${ratio}: pixelRatio is ${surface.pixelRatio}`);
        expectBufferSize(surface, container, [160, 120], ratio, `ratio ${ratio}`);
        expectDrawn(surface, container, `ratio ${ratio}`);
      }
      // 大きさが変わったときも今の倍率で合わせる
      await setPixelRatio(2);
      await until(() => log.pixelRatioChanged === 4, "pixelRatioChanged after the 4th change");
      surface.resize(100, 50);
      expectBufferSize(surface, container, [100, 50], 2, "resize at ratio 2");
      check(log.failed.length === 0, `failed: ${log.failed[0]}`);
      check(log.created.length === 1, `created was called ${log.created.length} times`);

      surface.detach();
      await setPixelRatio(1);
      await frames(3);
      check(log.pixelRatioChanged === 4, "pixelRatioChanged was called after detach");
    } finally {
      await setPixelRatio(1);
    }
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
    // どの作り直しも作れていた (作っては失った) ので、作れなかった理由は無い (T0946)
    check(log.failed[0].cause === undefined, `failed with the cause ${log.failed[0].cause}`);
    check(surface.puppet === null, "the puppet remains after giving up");
    check(canvasesIn(container).length === 0, "the canvas remains after giving up");
    check(live.size === 0, `${live.size} live puppets after giving up`);

    check((await surface.attach(container)) === "ready", "attach after giving up failed");
    expectDrawn(surface, container, "after attaching again");
    surface.detach();
  },

  /**
   * WebGL2 のコンテキストを失ったまま復元が届かなければ、すぐ `waiting` で知らせ、`RESTORE_TIMEOUT_MS` の後に新しい
   * canvas で作り直して描く (T0938)。
   */
  async "context-loss-timeout"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    check((await surface.attach(container)) === "ready", "attach failed");
    const before = surface.puppet;
    const oldCanvas = canvasesIn(container)[0];
    await loseContext(container);
    await until(() => surface.awaitingRestore, "webglcontextlost");
    check(log.waiting === 1, `waiting was called ${log.waiting} times`);
    const lostAt = performance.now();
    // 復元しない。時間切れまでは作り直さない
    await until(
      () => surface.puppet !== before && !surface.recovering,
      "the recreation on timeout",
    );
    const waited = performance.now() - lostAt;
    check(waited >= RESTORE_TIMEOUT_MS - 100, `recreated after ${waited.toFixed(0)} ms`);
    check(log.failed.length === 0, `failed: ${log.failed[0]}`);
    check(log.lost === 1, `lost was called ${log.lost} times`);
    const last = log.created.at(-1);
    check(log.created.length === 2 && last.recovered, "not recovered");
    const canvases = canvasesIn(container);
    check(canvases.length === 1 && canvases[0] !== oldCanvas, "the canvas was not replaced");
    check(!surface.puppet.is_lost() && surface.puppet.has_model(), "the new puppet is not usable");
    check(live.size === 1, `${live.size} live puppets`);
    expectDrawn(surface, container, "after the timeout");
    surface.detach();
  },

  /** 続けて失って諦めた後、`restart` で同じ container に作り直して描く (回数は数え直す。T0938)。 */
  async "restart-after-giving-up"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    check((await surface.attach(container)) === "ready", "attach failed");
    for (let round = 0; round <= MAX_RESTARTS; round++)
      await loseAndRestore(surface, container, log);
    check(log.failed[0] instanceof GpuLostRepeatedlyError, `failed with ${log.failed[0]}`);
    check(surface.puppet === null, "the puppet remains after giving up");

    await surface.restart();
    check(log.failed.length === 1, `failed was called ${log.failed.length} times`);
    check(surface.puppet !== null && log.created.at(-1).recovered, "not restarted");
    check(canvasesIn(container).length === 1, `${canvasesIn(container).length} canvases`);
    check(live.size === 1, `${live.size} live puppets`);
    expectDrawn(surface, container, "after restart");
    // 回数は数え直したので、もう一度失っても作り直す
    await loseAndRestore(surface, container, log);
    check(log.failed.length === 1, "gave up right after restart");
    expectDrawn(surface, container, "after a loss after restart");
    surface.detach();
  },

  /**
   * 失った WebGL2 のコンテキストしか取れない canvas では、`WebPuppet.create` は panic せずに `CreateError`
   * (`code` が `context_unavailable`) で失敗し、その後も wasm は使える (T0475)。
   */
  async "create-on-lost-context"() {
    for (const backend of ["webgl2", "auto"]) {
      const canvas = document.createElement("canvas");
      document.body.append(canvas);
      loseContextOf(canvas);
      let error = null;
      try {
        (await within(WebPuppet.create(canvas, backend), `create (${backend})`)).free();
      } catch (e) {
        error = e;
      }
      expectContextUnavailable(error, backend);
      check(live.size === 0, `${backend}: ${live.size} live puppets after the failure`);
    }
    // 失敗の後も、新しい canvas なら作って描ける
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    check((await within(surface.attach(container), "attach")) === "ready", "attach failed");
    check(log.failed.length === 0, `failed: ${log.failed[0]}`);
    expectDrawn(surface, container, "after the failures");
    surface.detach();
  },

  /** 最初の作成でコンテキストを作れなければ、`attach` は `"failed"` で、`failed` に `CreateError` を渡す (T0475)。 */
  async "attach-on-lost-context"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    // "auto" と、落ちた先の "webgl2" の両方
    loseContextsBeforeCreate = 2;
    const result = await within(surface.attach(container), "attach");
    check(result === "failed", `attach returned ${result}`);
    check(log.failed.length === 1, `failed was called ${log.failed.length} times`);
    expectContextUnavailable(log.failed[0], "attach");
    check(canvasesIn(container).length === 0, "the canvas remains after the failure");
    check(live.size === 0, `${live.size} live puppets`);
    // 次の attach では作れる
    check((await within(surface.attach(container), "attach again")) === "ready", "attach failed");
    expectDrawn(surface, container, "after attaching again");
    surface.detach();
  },

  /**
   * 最初の作成で作れず、`restart` で試し直しても作れないまま上限に当たったら、最後の `CreateError` を `cause` に持つ
   * `GpuLostRepeatedlyError` で諦める (使う側は作れなかった理由の文を出す。T0946)。その後の `restart` で作れる。
   */
  async "restart-on-lost-context-gives-up-with-cause"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    // "auto" と、落ちた先の "webgl2" の両方 (WebGL2 で作れていないので、試し直しも毎回 2 つ)
    loseContextsBeforeCreate = 2;
    check((await within(surface.attach(container), "attach")) === "failed", "attach did not fail");
    expectContextUnavailable(log.failed[0], "attach");

    loseContextsBeforeCreate = 2 * MAX_RESTARTS;
    await within(surface.restart(), "restart");
    check(loseContextsBeforeCreate === 0, `${loseContextsBeforeCreate} creations were not tried`);
    check(log.failed.length === 2, `failed was called ${log.failed.length} times`);
    const gaveUp = log.failed[1];
    check(gaveUp instanceof GpuLostRepeatedlyError, `failed with ${gaveUp}`);
    expectContextUnavailable(gaveUp.cause, "the cause");
    check(log.waiting === MAX_RESTARTS, `waiting was called ${log.waiting} times`);
    check(surface.puppet === null, "the puppet remains after giving up");
    check(canvasesIn(container).length === 0, "the canvas remains after giving up");
    check(live.size === 0, `${live.size} live puppets after giving up`);

    await within(surface.restart(), "restart again");
    check(surface.puppet !== null && log.created.at(-1)?.recovered, "not restarted");
    expectDrawn(surface, container, "after restarting again");
    surface.detach();
  },

  /**
   * GPU を失った後の作り直しでコンテキストを作れなければ (`CreateError`)、`waiting` で知らせて `RESTORE_TIMEOUT_MS` 後に
   * 試し直し、作り直しの回数に数える (T0938 の上限で諦める。T0475)。
   */
  async "recreate-on-lost-context"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    check((await surface.attach(container)) === "ready", "attach failed");
    const waitingBefore = log.waiting;
    // 作り直しの 1 回目は "auto" と "webgl2" のどちらも失ったコンテキストになる
    loseContextsBeforeCreate = 2;
    const puppet = await loseAndRestore(surface, container, log);
    check(loseContextsBeforeCreate === 0, "the recreation did not try both backends");
    check(log.failed.length === 0, `failed: ${log.failed[0]}`);
    check(puppet !== null && !puppet.is_lost(), "no usable puppet after the retry");
    // 復元を待ったときと、作れずに試し直しを待ったときの 2 回
    check(
      log.waiting - waitingBefore === 2,
      `waiting was called ${log.waiting - waitingBefore} times`,
    );
    check(log.created.length === 2 && log.created[1].recovered, "not recovered");
    check(live.size === 1, `${live.size} live puppets`);
    expectDrawn(surface, container, "after the retry");
    // 作れなかった作り直しも数えたので (2 回)、あと 1 回で上限 (MAX_RESTARTS) に当たり、次で諦める
    for (let round = 2; round < MAX_RESTARTS; round++) {
      await loseAndRestore(surface, container, log);
      check(log.failed.length === 0, `round ${round}: gave up too early`);
    }
    await loseAndRestore(surface, container, log);
    check(log.failed.length === 1, `failed was called ${log.failed.length} times`);
    check(log.failed[0] instanceof GpuLostRepeatedlyError, `failed with ${log.failed[0]}`);
    check(live.size === 0, `${live.size} live puppets after giving up`);
  },

  /**
   * 作成の途中で WebGL2 のコンテキストを失っても (作成の WebGL2 の呼び出しのどの前でも)、`WebPuppet.create` は panic
   * せずに `CreateError` (`context_unavailable`) で失敗する。その後も作って描ける (T0944)。
   */
  async "lose-context-during-create"() {
    const bytes = await modelBytes();
    // 作成の呼び出しの数を数える (失わずに 1 回作る)
    glCalls.count = 0;
    const canvas = document.createElement("canvas");
    document.body.append(canvas);
    const counted = await within(WebPuppet.create(canvas, "webgl2"), "create");
    const calls = glCalls.count;
    counted.free();
    check(calls > 10, `create made only ${calls} WebGL2 calls`);
    for (let n = 1; n <= calls; n++) {
      const canvas = document.createElement("canvas");
      document.body.append(canvas);
      glCalls.count = 0;
      glCalls.loseAtCall = n;
      let error = null;
      try {
        (await within(WebPuppet.create(canvas, "webgl2"), `create (lost before call ${n})`)).free();
      } catch (e) {
        error = e;
      } finally {
        glCalls.loseAtCall = 0;
        canvas.remove();
      }
      expectContextUnavailable(error, `lost before call ${n} of ${calls}`);
    }
    check(live.size === 0, `${live.size} live puppets after the failures`);
    // 失わなければ作って描ける
    const fresh = document.createElement("canvas");
    document.body.append(fresh);
    const result = await createLoadRender(fresh, bytes);
    check(result === "drawn", `after the losses: ${result}`);
  },

  /**
   * GPU プロセスが落ちた直後のように、リンクできたプログラムの `getUniformBlockIndex` が `INVALID_INDEX` を返しても
   * (`isContextLost()` は偽のまま)、`WebPuppet.create` は panic しない (T0944)。
   */
  async "invalid-block-index-during-create"() {
    const canvas = document.createElement("canvas");
    document.body.append(canvas);
    glCalls.invalidBlockIndex = true;
    try {
      (await within(WebPuppet.create(canvas, "webgl2"), "create")).free();
    } catch (e) {
      // 作れなくてもよい (コードを持つ失敗なら)
      check(e instanceof Error && e.name === "CreateError", `create failed with ${e?.name}: ${e}`);
    } finally {
      glCalls.invalidBlockIndex = false;
    }
    // 戻せば作って描ける
    const fresh = document.createElement("canvas");
    document.body.append(fresh);
    const result = await createLoadRender(fresh, await modelBytes());
    check(result === "drawn", `after the invalid indices: ${result}`);
  },

  /**
   * 作って描いている間に GPU プロセスごと失っても (Node の側から時機をずらして落とし、その間は作って描くことを
   * 繰り返す)、wasm は panic せず、失敗はコードを持つ例外で返る。落ちた時機は決まらない (どの段に当たるかは走るたびに
   * 変わる) ので、panic が無いことと、最後に作って描けることだけを確かめる (T0944)。
   */
  async "gpu-process-crash-during-create"() {
    const bytes = await modelBytes();
    const outcomes = new Map();
    for (const delay of [0, 30, 60, 100, 150, 200, 300, 400]) {
      await window.crashGpuProcess(delay);
      const until = performance.now() + delay + 600;
      while (performance.now() < until) {
        const canvas = document.createElement("canvas");
        document.body.append(canvas);
        const outcome = await createLoadRender(canvas, bytes);
        outcomes.set(outcome, (outcomes.get(outcome) ?? 0) + 1);
        canvas.remove();
      }
      await untilGpuIsBack();
    }
    console.log(
      `gpu-process-crash-during-create: ${[...outcomes].map(([k, v]) => `${k} x${v}`).join(", ")}`,
    );
    check(live.size === 0, `${live.size} live puppets`);
    const canvas = document.createElement("canvas");
    document.body.append(canvas);
    const result = await createLoadRender(canvas, bytes);
    check(result === "drawn", `after the crashes: ${result}`);
  },

  /**
   * 描いている途中で GPU プロセスが落ちたら、`PuppetSurface` は WebGL2 のコンテキストの喪失として復元を待ち、新しい
   * canvas で作り直して描く (2 回続けて。T0944)。
   */
  async "gpu-process-crash"() {
    const container = stage();
    const { surface, log } = await surfaceWithHost();
    check((await surface.attach(container)) === "ready", "attach failed");
    for (let round = 1; round <= 2; round++) {
      const before = surface.puppet;
      const oldCanvas = canvasesIn(container)[0];
      await crashGpuProcessAndWait();
      await until(
        () => (surface.puppet !== before && !surface.recovering) || log.failed.length > 0,
        `round ${round}: the recreation after the GPU process crashed`,
      );
      check(log.failed.length === 0, `round ${round}: failed: ${log.failed[0]}`);
      check(log.lost === round, `round ${round}: lost was called ${log.lost} times`);
      const last = log.created.at(-1);
      check(log.created.length === round + 1 && last.recovered, `round ${round}: not recovered`);
      const canvases = canvasesIn(container);
      check(
        canvases.length === 1 && canvases[0] !== oldCanvas,
        `round ${round}: the canvas was not replaced`,
      );
      check(live.size === 1, `round ${round}: ${live.size} live puppets`);
      expectDrawn(surface, container, `round ${round}`);
    }
    surface.detach();
  },

  /**
   * GPU プロセスが落ちたら、Player は新しい canvas の WebPuppet でモデルを読み直し、モジュールを付け直し、値を戻して描く
   * (T0944)。
   */
  async "player-gpu-process-crash"() {
    const container = stage();
    const player = newPlayer();
    player.setAutoIdle(false);
    player.setAutoEyeBlink(false);
    player.setBackground("#000000", true);
    await player.attach(container);
    const module = { name: "cat-ears.ugoemod", bytes: await fileBytes(DRESS_UP_MODULE) };
    await player.load({ name: "dress-up.ugoe", bytes: await fileBytes(DRESS_UP) }, [
      { file: module, attach: true },
    ]);
    check(player.status === "ready", `status after load is ${player.status} (${player.error})`);
    const attached = () => player.modules.filter((m) => m.attached).map((m) => m.name);
    const moduleName = attached()[0];
    check(moduleName !== undefined, "no module is attached");
    player.setParameter("MouthForm", 0.7);
    await frames(2);
    const oldCanvas = canvasesIn(container)[0];
    await crashGpuProcessAndWait();
    await until(() => {
      const canvases = canvasesIn(container);
      return (
        (canvases.length === 1 &&
          canvases[0] !== oldCanvas &&
          player.status === "ready" &&
          !player.waitingForGpu) ||
        player.status === "unavailable"
      );
    }, "the recreation of the player after the GPU process crashed");
    check(player.status === "ready", `status ${player.status} (${player.error})`);
    check(player.modelName === "dress-up.ugoe", `model ${player.modelName}`);
    check(
      attached().length === 1 && attached()[0] === moduleName,
      `attached modules: ${attached().join(", ")}`,
    );
    await frames(2);
    check(near(player.values.MouthForm, 0.7), `MouthForm is ${player.values.MouthForm}`);
    check(live.size === 1, `${live.size} live puppets`);
    await expectCaptured(player, "after the crash");
    player.detach();
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

  /**
   * Player (player.svelte.ts) が WebGL2 のコンテキストを失ったら、新しい canvas の WebPuppet でモデルを読み直し、
   * 付けていたモジュールを付け直し、パラメータの値を戻して描く (T0939)。利用者のズームとパン (`view`) も保ち、
   * 作り直しの後に利用者が開いたモデルは fit で見せる (T0942)。
   */
  async "player-loss-restore"() {
    const container = stage();
    const player = newPlayer();
    // 時間で動くもの (待機モーション・目パチ) を止めて、戻した値を比べられるようにする
    player.setAutoIdle(false);
    player.setAutoEyeBlink(false);
    player.setBackground("#000000", true);
    await player.attach(container);
    check(player.status === "empty", `status after attach is ${player.status}`);
    check(player.backend === "webgl2", `backend is ${player.backend}`);
    const module = { name: "cat-ears.ugoemod", bytes: await fileBytes(DRESS_UP_MODULE) };
    await player.load({ name: "dress-up.ugoe", bytes: await fileBytes(DRESS_UP) }, [
      { file: module, attach: true },
    ]);
    check(player.status === "ready", `status after load is ${player.status} (${player.error})`);
    const attached = () => player.modules.filter((m) => m.attached).map((m) => m.name);
    check(attached().length === 1, `attached modules: ${attached().join(", ")}`);
    const moduleName = attached()[0];
    player.setParameter("MouthForm", 0.7);
    await frames(2);
    check(near(player.values.MouthForm, 0.7), `MouthForm is ${player.values.MouthForm}`);
    await expectCaptured(player, "after load");
    const fitted = player.view;
    check(fitted !== null, "no view after load");
    player.zoomBy(2, 40, 30);
    player.panBy(12, -8);
    const view = player.view;
    check(
      view !== null && near(view.scale, fitted.scale * 2),
      `view after the zoom: ${describeView(view)}`,
    );
    // CI の SwiftShader のように作り直しを遅くする (T0960)
    createDelayMs = 500;

    for (let round = 1; round <= 2; round++) {
      const oldCanvas = canvasesIn(container)[0];
      const ext = await loseContext(container);
      await frames(3);
      // 復元を待っている間は作り直さない
      check(canvasesIn(container)[0] === oldCanvas, `round ${round}: recreated before the restore`);
      ext.restoreContext();
      await untilPlayerRecreated(
        player,
        container,
        oldCanvas,
        `round ${round}: the recreation of the player`,
      );
      check(player.status === "ready", `round ${round}: status ${player.status} (${player.error})`);
      check(player.error === null, `round ${round}: error ${player.error}`);
      check(player.modelName === "dress-up.ugoe", `round ${round}: model ${player.modelName}`);
      check(
        attached().length === 1 && attached()[0] === moduleName,
        `round ${round}: attached modules: ${attached().join(", ")}`,
      );
      await frames(2);
      check(
        near(player.values.MouthForm, 0.7),
        `round ${round}: MouthForm is ${player.values.MouthForm} after the recreation`,
      );
      check(
        isView(player.view, view),
        `round ${round}: view ${describeView(player.view)}, expected ${describeView(view)}`,
      );
      check(near(player.zoom, 2), `round ${round}: zoom ${player.zoom}`);
      check(live.size === 1, `round ${round}: ${live.size} live puppets`);
      await expectCaptured(player, `round ${round}`);
    }
    // 利用者が開き直したモデルは fit で見せる
    await player.load({ name: "dress-up.ugoe", bytes: await fileBytes(DRESS_UP) });
    check(
      isView(player.view, fitted),
      `view after reopening ${describeView(player.view)}, expected ${describeView(fitted)}`,
    );
    player.detach();
    check(canvasesIn(container).length === 0, "the canvas remains after detach");
  },

  /**
   * Player の読み込みの途中に WebGL2 のコンテキストを失ったら (wasm の `load_bundle`・`load_module` が `device_lost`)、エラーの文を
   * 出さずに、開こうとしたモデルとモジュールを作り直した後に開いて描く (T0955。`<ugoe-avatar>` の T0486 と同じ扱い)。
   * 開いたモデルなので fit で見せる。初めのモデル (前のモデルが無い) でも同じ。
   */
  async "player-loss-during-load"() {
    const container = stage();
    const player = newPlayer();
    player.setBackground("#000000", true);
    await player.attach(container);
    check(player.status === "empty", `status after attach is ${player.status}`);
    // CI の SwiftShader のように作り直しを遅くして、作り終える前に確かめないことを確かめる (T0960)
    createDelayMs = 500;

    // 初めのモデルを開く途中に失った
    let oldCanvas = canvasesIn(container)[0];
    let ext = await loseContext(container);
    await until(() => player.waitingForGpu, "waitingForGpu after the loss");
    await player.load({ name: "avatar.ugoe", bytes: await modelBytes() });
    check(player.error === null, `error while the context is lost: ${player.error}`);
    check(player.modelName !== "avatar.ugoe", "the model was opened while the context is lost");
    ext.restoreContext();
    await untilPlayerRecreated(
      player,
      container,
      oldCanvas,
      "the first model after the recreation",
    );
    check(player.status === "ready", `status ${player.status} (${player.error})`);
    check(player.error === null, `error after the recreation: ${player.error}`);
    check(player.modelName === "avatar.ugoe", `model ${player.modelName}`);
    await expectCaptured(player, "the first model after the recreation");
    player.zoomBy(2, 40, 30);

    // 別のモデルとモジュールを開く途中に失った
    oldCanvas = canvasesIn(container)[0];
    ext = await loseContext(container);
    await until(() => player.waitingForGpu, "waitingForGpu after the second loss");
    const module = { name: "cat-ears.ugoemod", bytes: await fileBytes(DRESS_UP_MODULE) };
    await player.load({ name: "dress-up.ugoe", bytes: await fileBytes(DRESS_UP) }, [
      { file: module, attach: true },
    ]);
    check(player.error === null, `error while the context is lost: ${player.error}`);
    check(player.dressUpError === null, `dress-up error while lost: ${player.dressUpError}`);
    check(player.modelName === "avatar.ugoe", `model while lost ${player.modelName}`);
    ext.restoreContext();
    await untilPlayerRecreated(
      player,
      container,
      oldCanvas,
      "the second model after the recreation",
    );
    check(player.status === "ready", `status ${player.status} (${player.error})`);
    check(player.error === null, `error after the recreation: ${player.error}`);
    check(player.dressUpError === null, `dress-up error: ${player.dressUpError}`);
    check(player.modelName === "dress-up.ugoe", `model ${player.modelName}`);
    const attached = player.modules.filter((m) => m.attached).map((m) => m.name);
    check(attached.length === 1, `attached modules: ${attached.join(", ")}`);
    check(near(player.zoom, 1), `zoom after opening a model ${player.zoom}`);
    check(live.size === 1, `${live.size} live puppets`);
    await expectCaptured(player, "the second model after the recreation");
    player.detach();
  },

  /**
   * Player のポーズの欄 (`poseShown`) は、モーションによる切り替えに追従する: 着せ替えの見本の `wave` で `HandsDown` → `Wave` →
   * `HandsDown` と変わり、モーションが終わった後に利用者が選んだ要素は残る (T0736。本物の wasm の `pose_visible_json`)。
   */
  async "player-pose-follows-motion"() {
    const container = stage();
    const player = newPlayer();
    player.setAutoIdle(false);
    await player.attach(container);
    await player.load({ name: "dress-up.ugoe", bytes: await fileBytes(DRESS_UP) });
    const group = player.poseGroups.findIndex((g) => g.parts.includes("Wave"));
    check(group >= 0, `no group with Wave: ${JSON.stringify(player.poseGroups)}`);
    const shown = () => player.poseShown[group];
    check(shown() === "HandsDown", `shown ${shown()} before the motion`);
    check(player.playMotion("wave"), "wave did not start");
    await until(() => shown() === "Wave", "the choice to follow the motion to Wave");
    await until(() => shown() === "HandsDown", "the choice to follow the motion back to HandsDown");
    await until(() => player.currentMotion === null, "the motion to end");
    check(player.showPose("Wave"), "showPose(Wave) failed");
    await frames(10);
    check(shown() === "Wave", `shown ${shown()} after choosing Wave`);
    player.detach();
  },

  /**
   * Player は WebGL2 のコンテキストを失ったら `waitingForGpu` になり、復元が届かなければ時間切れで作り直して描く。
   * 続けて失って諦めたら `canRestart` になり、`restart` でモデル・モジュール・値・ビューを戻して描く (T0938・T0942)。
   */
  async "player-timeout-restart"() {
    const container = stage();
    const player = newPlayer();
    player.setAutoIdle(false);
    player.setAutoEyeBlink(false);
    player.setBackground("#000000", true);
    await player.attach(container);
    const module = { name: "cat-ears.ugoemod", bytes: await fileBytes(DRESS_UP_MODULE) };
    await player.load({ name: "dress-up.ugoe", bytes: await fileBytes(DRESS_UP) }, [
      { file: module, attach: true },
    ]);
    check(player.status === "ready", `status after load is ${player.status} (${player.error})`);
    const attached = () => player.modules.filter((m) => m.attached).map((m) => m.name);
    const moduleName = attached()[0];
    check(moduleName !== undefined, "no module is attached");
    player.setParameter("MouthForm", 0.7);
    await frames(2);
    player.zoomBy(1.5, 100, 20);
    player.panBy(-6, 9);
    const view = player.view;

    /** モデル・モジュール・値・ビューが戻って描いている。 */
    const expectRestored = async (when) => {
      check(player.status === "ready", `${when}: status ${player.status} (${player.error})`);
      check(player.modelName === "dress-up.ugoe", `${when}: model ${player.modelName}`);
      check(
        attached().length === 1 && attached()[0] === moduleName,
        `${when}: attached modules: ${attached().join(", ")}`,
      );
      await frames(2);
      check(near(player.values.MouthForm, 0.7), `${when}: MouthForm is ${player.values.MouthForm}`);
      check(
        isView(player.view, view),
        `${when}: view ${describeView(player.view)}, expected ${describeView(view)}`,
      );
      check(live.size === 1, `${when}: ${live.size} live puppets`);
      await expectCaptured(player, when);
    };

    // 復元が届かない: すぐ待っていると出し、時間切れで作り直す
    const oldCanvas = canvasesIn(container)[0];
    await loseContext(container);
    await until(() => player.waitingForGpu, "waitingForGpu after the loss");
    await until(
      () =>
        canvasesIn(container)[0] !== oldCanvas &&
        player.status === "ready" &&
        !player.waitingForGpu,
      "the recreation of the player on timeout",
    );
    await expectRestored("after the timeout");

    // 続けて失って諦める (復元はすぐ届く)
    for (let round = 0; round < MAX_RESTARTS; round++) {
      const canvas = canvasesIn(container)[0];
      const ext = await loseContext(container);
      await until(() => player.waitingForGpu, `round ${round}: waitingForGpu`);
      ext.restoreContext();
      // 待つ間も status は ready のままなので、待ち終わった (`waitingForGpu` が偽) ことも見る
      await until(
        () =>
          !player.waitingForGpu &&
          ((canvasesIn(container)[0] !== canvas && player.status === "ready") ||
            player.status === "unavailable"),
        `round ${round}: the recreation`,
      );
    }
    check(player.status === "unavailable", `status after giving up is ${player.status}`);
    check(player.canRestart, "canRestart is false after giving up");
    check(!player.waitingForGpu, "still waiting after giving up");
    check(canvasesIn(container).length === 0, "the canvas remains after giving up");

    // 「作り直す」
    await player.restart();
    await until(() => player.status === "ready" || player.status === "unavailable", "restart");
    check(!player.canRestart, "canRestart remains after restart");
    check(player.error === null, `error after restart: ${player.error}`);
    await expectRestored("after restart");
    player.detach();
  },

  /**
   * devicePixelRatio だけが変わったら、Player の描画バッファは新しい倍率になり、CSS ピクセルで見たビュー (`view`) と
   * 読み戻す画像の大きさが合う (T0939)。
   */
  async "player-pixel-ratio"() {
    const container = stage();
    const player = newPlayer();
    try {
      await player.attach(container);
      await player.load({ name: "avatar.ugoe", bytes: await modelBytes() });
      check(player.status === "ready", `status after load is ${player.status} (${player.error})`);
      const before = player.view;
      check(before !== null, "no view after load");
      for (const ratio of [2, 1.5]) {
        await setPixelRatio(ratio);
        await until(() => {
          const canvas = canvasesIn(container)[0];
          return canvas?.width === Math.round(160 * ratio);
        }, `the canvas at ratio ${ratio}`);
        const canvas = canvasesIn(container)[0];
        check(
          canvas.height === Math.round(120 * ratio),
          `ratio ${ratio}: the canvas is ${canvas.width}x${canvas.height}`,
        );
        const view = player.view;
        check(
          view !== null &&
            near(view.scale, before.scale) &&
            near(view.x, before.x) &&
            near(view.y, before.y),
          `ratio ${ratio}: the view moved from ${JSON.stringify(before)} to ${JSON.stringify(view)}`,
        );
        const { shot } = await capturedFraction(player);
        check(
          shot.width === canvas.width && shot.height === canvas.height,
          `ratio ${ratio}: the capture is ${shot.width}x${shot.height}`,
        );
        // CSS ピクセルの点を中心にズームしても、その点にあるキャンバスの点は動かない (換算に今の倍率を使う)
        const [x, y] = [40, 30];
        const anchor = [(x - view.x) / view.scale, (y - view.y) / view.scale];
        player.zoomBy(2, x, y);
        const zoomed = player.view;
        const moved = [zoomed.x + zoomed.scale * anchor[0], zoomed.y + zoomed.scale * anchor[1]];
        check(
          near(zoomed.scale, view.scale * 2) && near(moved[0], x) && near(moved[1], y),
          `ratio ${ratio}: zooming at (${x}, ${y}) moved the point to (${moved.join(", ")})`,
        );
        player.fit();
      }
      player.detach();
    } finally {
      await setPixelRatio(1);
    }
  },
};

window.runScenarios = async (names) => {
  await init();
  const results = [];
  for (const [name, run] of Object.entries(scenarios)) {
    if (names && !names.includes(name)) continue;
    panics.length = 0;
    try {
      await run();
      // シナリオの後に WebPuppet が残っていない (GPU の資源を捨てた)
      check(live.size === 0, `${live.size} live puppets after the scenario`);
      check(panics.length === 0, `wasm panicked: ${panics[0]}`);
      results.push({ name, ok: true });
    } catch (e) {
      results.push({ name, ok: false, error: e instanceof Error ? e.message : String(e) });
    } finally {
      onCreating = null;
      loseContextsBeforeCreate = 0;
      createDelayMs = 0;
      glCalls.loseAtCall = 0;
      glCalls.invalidBlockIndex = false;
      for (const surface of surfaces.splice(0)) surface.detach();
      for (const player of players.splice(0)) player.detach();
      for (let i = localStorage.length - 1; i >= 0; i--) {
        const key = localStorage.key(i);
        if (key?.startsWith(STORAGE_PREFIX)) localStorage.removeItem(key);
      }
      for (const puppet of live) puppet.free();
      document.body.replaceChildren();
    }
  }
  return results;
};
