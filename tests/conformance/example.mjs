// 見本 (bindings/web/examples/basic) をヘッドレスの Chrome で開き、「camera (MediaPipe)」で Face Landmarker を作った後に
// MediaPipe の利用状況の送信 (`odml.pa.googleapis.com`) がネットワークに出ないことを確かめる (run.mjs の `example`。T0487)。
// 手順と記録の取り方は tests/conformance/README.md の「見本の MediaPipe の利用状況の送信」。
//
// - 見本は書き換えずに配る。import map の `./node_modules/@ugoe/runtime/` は bindings/web (パッケージ)、`/samples/` は
//   assets/samples を指す (serve.mjs と同じ配り方で、tgz を作らない)。
// - 見本が CDN から読む MediaPipe (jsdelivr の @mediapipe/tasks-vision) とモデル (storage.googleapis.com) は、CDP の Fetch で
//   止めて手元のファイルで応える (ネットワークに頼らない)。MediaPipe は packages/face-tracking が入れている
//   @mediapipe/tasks-vision (見本と同じ版でなければ止まる)、モデルは packages/face-tracking の mediapipe-assets.mjs が
//   取得して SHA-256 を確かめたもの。
// - カメラは Chrome の偽の映像 (`--use-fake-device-for-media-stream`)。
// - 記録: ページの CDP の `Network.requestWillBeSent` で、ページが出した要求をすべて記録する (Fetch で止めた要求も含む)。
//   見本の `fetch` の包みが断った要求は、ネットワークの層に届かないので記録されない。
// - MediaPipe は送信を 60 秒ごとの `setInterval` で行うので、ページを開く前に 60 秒の `setInterval` を FLUSH_MS に縮める。
// - 2 回開く: 見本のまま (`guarded`。送信を断った回数が 1 以上で、送り先への要求が 0 件) と、見本の包みを外した対照
//   (`unguarded`。送り先への要求が記録される。テストが送信を見つけられることの確かめ)。対照の要求は Fetch で止めるので、
//   実際には送らない。

import { spawn, spawnSync } from "node:child_process";
import {
  createWriteStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, extname, join, normalize, sep } from "node:path";
import { browserCdp } from "./cdp.mjs";

/** MediaPipe の利用状況の送り先 (bindings/web/README.md の「MediaPipe の利用状況の送信」)。 */
const USAGE_LOG_HOST = "odml.pa.googleapis.com";
/** MediaPipe の送信の間隔 (ミリ秒) と、テストで縮めた間隔。 */
const MEDIAPIPE_FLUSH_MS = 60_000;
const FLUSH_MS = 1000;

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
};

/**
 * 見本を `guarded` と `unguarded` の 2 回開いて確かめ、通ったかを返す。結果は `<out>/example.json`、Chrome のログは
 * `<out>/chrome-<guarded|unguarded>.log`。モデルは `cache` に置く (run.mjs は毎回 `out` を消すので、その外に)。
 */
export async function runExample({ chrome, root, out, cache }) {
  mkdirSync(out, { recursive: true });
  const example = join(root, "bindings", "web", "examples", "basic");
  const html = readFileSync(join(example, "index.html"), "utf8");
  const mediapipe = html.match(/const MEDIAPIPE = "([^"]+)"/)?.[1];
  const faceModel = html.match(/const FACE_MODEL =\s*"([^"]+)"/)?.[1];
  if (!mediapipe || !faceModel) {
    console.error("example: MEDIAPIPE or FACE_MODEL was not found in examples/basic/index.html");
    return false;
  }

  // 見本と同じ版の @mediapipe/tasks-vision (packages/face-tracking の依存)
  const faceTracking = join(root, "packages", "face-tracking");
  let tasksVision;
  try {
    tasksVision = dirname(
      createRequire(join(faceTracking, "package.json")).resolve("@mediapipe/tasks-vision"),
    );
  } catch {
    console.error("example: @mediapipe/tasks-vision is not installed; run `pnpm install`");
    return false;
  }
  const installed = JSON.parse(readFileSync(join(tasksVision, "package.json"), "utf8")).version;
  const wanted = mediapipe.match(/@mediapipe\/tasks-vision@([^/]+)$/)?.[1];
  if (installed !== wanted) {
    console.error(
      `example: the example loads @mediapipe/tasks-vision@${wanted} but packages/face-tracking has ${installed}; ` +
        "update MEDIAPIPE in examples/basic/index.html together with the package",
    );
    return false;
  }

  // モデル (取得済みで SHA-256 が合えば取得しない)
  const assets = join(faceTracking, "scripts", "mediapipe-assets.mjs");
  if (!readFileSync(assets, "utf8").includes(faceModel)) {
    console.error(`example: ${faceModel} is not the model that mediapipe-assets.mjs downloads`);
    return false;
  }
  const modelDir = cache;
  const fetched = spawnSync("node", [assets, modelDir], { cwd: root, stdio: "inherit" });
  const model = join(modelDir, faceModel.split("/").at(-1));
  if (fetched.status !== 0 || !existsSync(model)) {
    console.error("example: could not prepare the Face Landmarker model");
    return false;
  }

  const results = {};
  for (const mode of ["guarded", "unguarded"]) {
    results[mode] = await openExample({
      chrome,
      root,
      out,
      mode,
      example,
      mediapipe,
      tasksVision,
      faceModel,
      model,
    });
    console.log(
      `example (${mode}): ${results[mode].ok ? "ok" : `FAILED: ${results[mode].reason}`}`,
    );
  }
  writeFileSync(join(out, "example.json"), `${JSON.stringify(results, null, 2)}\n`);
  return Object.values(results).every((r) => r.ok);
}

/** 見本を 1 回開き、`{ok, reason?, usageLogRequests, usageLogsRefused, cameraFrames, ...}` を返す。 */
async function openExample({
  chrome,
  root,
  out,
  mode,
  example,
  mediapipe,
  tasksVision,
  faceModel,
  model,
}) {
  const runtime = join(root, "bindings", "web");
  const samples = join(root, "assets", "samples");
  const server = createServer((req, res) => {
    const { pathname } = new URL(req.url ?? "/", "http://localhost");
    const prefix = "/node_modules/@ugoe/runtime/";
    let file = null;
    try {
      if (pathname === "/" || pathname === "/index.html") file = join(example, "index.html");
      else if (pathname.startsWith(prefix)) file = within(runtime, pathname.slice(prefix.length));
      else if (pathname.startsWith("/samples/"))
        file = within(samples, pathname.slice("/samples/".length));
    } catch {
      file = null;
    }
    if (!file || !existsSync(file)) {
      res.writeHead(404, { "content-type": "text/plain" }).end("not found");
      return;
    }
    res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    res.end(readFileSync(file));
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const { port } = server.address();

  const profile = mkdtempSync(join(tmpdir(), "ugoe-conformance-example-"));
  const browser = spawn(
    chrome,
    [
      "--headless=new",
      `--user-data-dir=${profile}`,
      "--no-first-run",
      "--no-default-browser-check",
      "--enable-logging=stderr",
      "--use-angle=swiftshader",
      "--enable-unsafe-swiftshader",
      // 偽のカメラ (緑の地に動く図形) を使い、許可を尋ねずに与える
      "--use-fake-device-for-media-stream",
      "--use-fake-ui-for-media-stream",
      "--remote-debugging-port=0",
      ...(process.env.CI ? ["--no-sandbox"] : []),
      "about:blank",
    ],
    { stdio: ["ignore", "ignore", "pipe"] },
  );
  browser.stderr.pipe(createWriteStream(join(out, `chrome-${mode}.log`)));
  const exited = new Promise((r) => browser.on("exit", (code) => r(code)));
  const cdp = browserCdp(profile);

  /** ページが出した要求 (`Network.requestWillBeSent`) のうち、送り先への要求の URL。 */
  const usageLogRequests = [];
  /** 手元のファイルで応えられなかった外への要求 (ネットワークに頼らないよう断る)。 */
  const unexpected = [];
  let result;
  try {
    const sessionId = await attachToPage(cdp);
    const send = (method, params) => cdp.send(method, params, sessionId);
    cdp.on("Network.requestWillBeSent", (params, session) => {
      if (session !== sessionId) return;
      if (new URL(params.request.url).hostname === USAGE_LOG_HOST)
        usageLogRequests.push(params.request.url);
    });
    cdp.on("Fetch.requestPaused", (params, session) => {
      if (session !== sessionId) return;
      const reply = respond(params.request.url, { mediapipe, tasksVision, faceModel, model });
      if (reply) {
        send("Fetch.fulfillRequest", { requestId: params.requestId, ...reply }).catch(() => {});
      } else {
        if (new URL(params.request.url).hostname !== USAGE_LOG_HOST)
          unexpected.push(params.request.url);
        send("Fetch.failRequest", {
          requestId: params.requestId,
          errorReason: "BlockedByClient",
        }).catch(() => {});
      }
    });
    await send("Network.enable");
    // Page.addScriptToEvaluateOnNewDocument は Page を有効にしないと効かない
    await send("Page.enable");
    // 外 (https) への要求だけを止める。ページ (http://127.0.0.1) の要求はそのまま通す
    await send("Fetch.enable", { patterns: [{ urlPattern: "https://*" }] });
    await send("Page.addScriptToEvaluateOnNewDocument", { source: initScript(mode) });
    await send("Page.navigate", { url: `http://127.0.0.1:${port}/?backend=webgl2` });
    const evaluate = async (expression) => {
      const r = await send("Runtime.evaluate", {
        expression,
        returnByValue: true,
        awaitPromise: true,
      });
      if (r.exceptionDetails)
        throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
      return r.result.value;
    };
    result = await check(evaluate, mode, usageLogRequests);
  } catch (e) {
    result = { ok: false, reason: String(e?.message ?? e) };
  } finally {
    cdp.close();
    browser.kill();
    await exited;
    server.close();
    rmSync(profile, { recursive: true, force: true });
  }
  if (unexpected.length > 0)
    console.warn(`example (${mode}): refused external requests: ${unexpected.join(", ")}`);
  return { ...result, usageLogRequests, unexpected };
}

/** 見本を操作して確かめる。 */
async function check(evaluate, mode, usageLogRequests) {
  // 見本が操作の処理をすべて付けた印 (`ready`) を待つ
  await waitFor("the example to be ready", 60_000, () =>
    evaluate("window.ugoeExample?.ready || null"),
  );
  // 「camera (MediaPipe)」: Face Landmarker を作り (createFromOptions)、偽のカメラの映像を調べ始める
  await evaluate('document.getElementById("camera").click()');
  const camera = await waitFor("the camera to start", 120_000, () =>
    evaluate(`(() => {
      const frames = window.ugoeExample.cameraFrames ?? 0;
      const checked = document.getElementById("camera").checked;
      return frames > 0 || !checked ? { frames, checked, errors: window.ugoeExample.errors } : null;
    })()`),
  );
  if (camera.frames === 0)
    return { ok: false, reason: `the camera did not start: ${camera.errors.join("; ")}` };
  const state = () =>
    evaluate(
      "({ refused: window.ugoeExample.usageLogsRefused ?? 0, frames: window.ugoeExample.cameraFrames })",
    );
  if (mode === "unguarded") {
    // 対照: 包みが無ければ送信がネットワークの層に出る
    await waitFor(
      "the usage log request",
      20 * FLUSH_MS,
      async () => usageLogRequests.length > 0 || null,
    );
    const { refused, frames } = await state();
    return {
      ok: refused === 0,
      reason: refused === 0 ? undefined : "the guard was not removed",
      usageLogsRefused: refused,
      cameraFrames: frames,
    };
  }
  // 見本のまま: 送信を断る (送ろうとしたことは断った回数で分かる)。断った後も数回の間隔を待ち、要求が出ないことを見る
  // (包みが効いていなければ、断られずに要求が記録される。そのときは待たずに失敗にする)
  await waitFor("the usage log to be refused", 20 * FLUSH_MS, async () =>
    (await state()).refused > 0 || usageLogRequests.length > 0 ? true : null,
  );
  await new Promise((r) => setTimeout(r, 3 * FLUSH_MS));
  const { refused, frames } = await state();
  const ok = usageLogRequests.length === 0;
  return {
    ok,
    reason: ok
      ? undefined
      : `requests to ${USAGE_LOG_HOST} reached the network: ${usageLogRequests.join(", ")}`,
    usageLogsRefused: refused,
    cameraFrames: frames,
  };
}

/** ページを開く前に動かすスクリプト。 */
function initScript(mode) {
  return `(() => {
  // MediaPipe の送信の間隔 (${MEDIAPIPE_FLUSH_MS} ミリ秒の setInterval) を縮める
  const setIntervalNative = window.setInterval;
  window.setInterval = function (handler, timeout, ...rest) {
    return setIntervalNative.call(this, handler, timeout === ${MEDIAPIPE_FLUSH_MS} ? ${FLUSH_MS} : timeout, ...rest);
  };
  ${
    mode === "unguarded"
      ? // 見本の refuseMediaPipeUsageLog() は、fetch が包んだものなら何もしない。包んだ印を付けて包みを外す
        "window.fetch.refusesMediaPipeUsageLog = true;"
      : ""
  }
})();`;
}

/** CDP の Fetch で止めた外への要求に、手元のファイルで応える。応えられなければ null。 */
function respond(url, { mediapipe, tasksVision, faceModel, model }) {
  let file = null;
  if (url === faceModel) file = model;
  else if (url.startsWith(`${mediapipe}/`))
    file = within(tasksVision, url.slice(mediapipe.length + 1));
  if (!file || !existsSync(file)) return null;
  return {
    responseCode: 200,
    responseHeaders: [
      { name: "Content-Type", value: TYPES[extname(file)] ?? "application/octet-stream" },
      // 見本は MediaPipe を別のオリジン (CDN) から読む
      { name: "Access-Control-Allow-Origin", value: "*" },
    ],
    body: readFileSync(file).toString("base64"),
  };
}

/** Chrome が開いた最初のページに `flatten` で繋ぎ、セッションの ID を返す。 */
async function attachToPage(cdp) {
  const page = await waitFor("the page target", 10_000, async () => {
    const { targetInfos } = await cdp.send("Target.getTargets");
    return targetInfos.find((t) => t.type === "page") ?? null;
  });
  const { sessionId } = await cdp.send("Target.attachToTarget", {
    targetId: page.targetId,
    flatten: true,
  });
  return sessionId;
}

/** `probe` が null 以外を返すまで待ってその値を返す。`timeoutMs` を過ぎたら投げる。 */
async function waitFor(what, timeoutMs, probe) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const value = await probe();
    if (value !== null && value !== undefined) return value;
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 200));
  }
}

/** `base` の中の相対パスをファイルのパスにする (外に出るパスは null)。 */
function within(base, path) {
  const file = normalize(join(base, decodeURIComponent(path)));
  return file.startsWith(base + sep) ? file : null;
}
