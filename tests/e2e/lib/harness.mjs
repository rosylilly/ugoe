// テストの筋書き (specs/*.test.mjs) から使う道具: アプリの起動・手順・失敗したときの記録 (tests/e2e/README.md)。
//
// run.mjs が環境変数で渡すもの:
//   E2E_DRIVER  tauri-driver の URL
//   E2E_APP     起動するアプリの実行ファイル
//   E2E_OUT     このアプリの結果を置くディレクトリ (スクリーンショット・ログ)
//   E2E_WORK    テストが作ってよいファイルの置き場 (見本の写し・保存先など)
//   E2E_DISPLAY 画面全体のスクリーンショットを撮る X のディスプレイ (無ければ撮らない)

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { after, before, it } from "node:test";
import { Session } from "./webdriver.mjs";

export const root = resolve(import.meta.dirname, "..", "..", "..");

function env(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is not set (run the tests with \`just e2e\`)`);
  return value;
}

export const out = () => env("E2E_OUT");
export const work = () => env("E2E_WORK");

/**
 * ページの console と例外を `window.__E2E_LOG__` に、CSP が止めたものを `window.__E2E_CSP__` に溜める (起動の後に入れる)。
 *
 * CSP が止めたものは 3 つの口から集める。指示・止めたもの・場所・文が同じものは 1 件にまとめる。
 * - `securitypolicyviolation` のイベント (入れた後のもの)。
 * - `ReportingObserver` の `csp-violation` の報告。`buffered` で、入れる前のもの (読み込みの途中) も受け取る。
 * - 文に "Content Security Policy" を含む例外・console の出力。WebKitGTK は wasm のコンパイル (`'wasm-unsafe-eval'`) を止めても
 *   イベントを出さず、`CompileError` を投げるだけなので、これが無いと見逃す。
 * 入れる前に起きてイベントも報告も出ないもの (起動の直後の wasm) は、画面に出たエラーの文から拾う (`checkNoCspViolations`)。
 * 見逃すもの: それも画面に出ないもの・アプリが握りつぶした例外・Worker の中の違反 (Worker に届く)。
 */
const CAPTURE_LOG = `
  if (window.__E2E_LOG__) return;
  const csp = (window.__E2E_CSP__ = []);
  const violation = (v) => {
    const entry = {
      directive: v.effectiveDirective ?? v.violatedDirective ?? null,
      blocked: v.blockedURI ?? v.blockedURL ?? null,
      source: v.sourceFile ?? null,
      line: v.lineNumber ?? null,
      sample: v.sample || null,
    };
    const key = JSON.stringify(entry);
    if (!csp.some((e) => JSON.stringify(e) === key)) csp.push(entry);
  };
  document.addEventListener("securitypolicyviolation", violation, true);
  if (typeof ReportingObserver === "function") {
    new ReportingObserver((reports) => reports.forEach((r) => violation(r.body)), {
      types: ["csp-violation"],
      buffered: true,
    }).observe();
  }
  const log = (window.__E2E_LOG__ = []);
  const push = (level, args) => {
    const text = args.map((a) => { try { return a instanceof Error ? a.stack ?? String(a) : typeof a === "string" ? a : JSON.stringify(a); } catch { return String(a); } }).join(" ");
    log.push({ t: Math.round(performance.now()), level, text });
    if (text.includes("Content Security Policy")) violation({ sample: text.slice(0, 500) });
  };
  for (const level of ["log", "info", "warn", "error", "debug"]) {
    const original = console[level].bind(console);
    console[level] = (...args) => { push(level, args); original(...args); };
  }
  window.addEventListener("error", (e) => push("uncaught", [e.error ?? e.message]));
  window.addEventListener("unhandledrejection", (e) => push("unhandledrejection", [e.reason]));
`;

/**
 * アプリを 1 つ起動し、筋書きの間使う。`describe` の中で呼ぶ。
 * 終わったら、すべてのウィンドウのページのログを `E2E_OUT/<名前>.console.json` に書いてからアプリを閉じる。
 * @param {{ args?: string[], name?: string }} options
 */
export function launch({ args = [], name = "app" } = {}) {
  /** @type {{ session: Session, main: string, unresponsive?: string }} */
  const app = {};
  before(async () => {
    app.session = await Session.create(env("E2E_DRIVER"), { application: env("E2E_APP"), args });
    app.main = await app.session.windowHandle();
    await captureLogs(app.session);
  });
  after(async () => {
    if (!app.session) return;
    if (app.unresponsive) app.session.timeout = 5_000;
    await writeLogs(app.session, join(out(), `${name}.console.json`)).catch((e) =>
      console.error(`could not read the page logs: ${e.message}`),
    );
    await app.session.close();
  });
  return app;
}

/** 今開いているすべてのウィンドウに、ページのログの取り込みを入れる。 */
export async function captureLogs(session) {
  const current = await session.windowHandle();
  for (const handle of await session.windowHandles()) {
    await session.switchToWindow(handle);
    await session.execute(CAPTURE_LOG);
  }
  await session.switchToWindow(current);
}

async function writeLogs(session, path) {
  const current = await session.windowHandle().catch(() => null);
  const logs = {};
  for (const handle of await session.windowHandles()) {
    await session.switchToWindow(handle);
    const url = await session.url().catch(() => "?");
    logs[`${handle} ${url}`] = await session.execute(
      "return { console: window.__E2E_LOG__ ?? null, csp: window.__E2E_CSP__ ?? null }",
    );
  }
  if (current) await session.switchToWindow(current);
  mkdirSync(out(), { recursive: true });
  writeFileSync(path, `${JSON.stringify(logs, null, 2)}\n`);
}

/**
 * 筋書きの 1 つの手順。失敗したら、各ウィンドウのスクリーンショット・画面全体のスクリーンショット・ページのログを
 * `E2E_OUT/failures/<手順>/` に残してから失敗にする。
 */
export function step(app, name, fn) {
  it(name, async () => {
    // 前の手順でアプリが応答しなくなっていたら、待たずに失敗にする (要求ごとにタイムアウトまで待つと何分もかかる)
    if (app.unresponsive) throw new Error(`the app stopped responding in "${app.unresponsive}"`);
    try {
      await fn();
    } catch (e) {
      const timedOut = e?.name === "TimeoutError";
      if (timedOut) app.unresponsive = name;
      await recordFailure(app.session, name, timedOut).catch((r) =>
        console.error(`could not record the failure: ${r.message}`),
      );
      if (timedOut) {
        throw new Error(
          "a WebDriver request timed out: the page stopped responding (its main thread is blocked or its event queue is flooded)",
          { cause: e },
        );
      }
      throw e;
    }
  });
}

const slug = (s) =>
  s
    .replace(/[^\w.-]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 80) || "step";

async function recordFailure(session, name, unresponsive) {
  const dir = join(out(), "failures", slug(name));
  mkdirSync(dir, { recursive: true });
  screen(join(dir, "screen.png"));
  // 応答しないページには、画面全体のスクリーンショットだけを残す
  if (!session || unresponsive) return;
  const current = await session.windowHandle().catch(() => null);
  for (const [i, handle] of (await session.windowHandles()).entries()) {
    try {
      await session.switchToWindow(handle);
      // 隠れたウィンドウのスクリーンショットは WebKitWebDriver が応答しない
      if (await session.execute("return document.hidden")) {
        writeFileSync(join(dir, `window-${i}.hidden.txt`), "the window was hidden\n");
        continue;
      }
      writeFileSync(join(dir, `window-${i}.png`), await session.screenshot());
    } catch (e) {
      writeFileSync(join(dir, `window-${i}.error.txt`), String(e.stack ?? e));
    }
  }
  if (current) await session.switchToWindow(current).catch(() => {});
  await writeLogs(session, join(dir, "console.json"));
}

/** X の画面全体を PNG で撮る (xwd と ImageMagick があれば。無ければ何もしない)。 */
export function screen(path) {
  const display = process.env.E2E_DISPLAY;
  if (!display) return;
  const xwd = spawnSync("xwd", ["-root", "-silent", "-display", display], {
    maxBuffer: 256 << 20,
  });
  if (xwd.status !== 0) return;
  spawnSync("convert", ["xwd:-", `png:${path}`], { input: xwd.stdout });
}

/** 名前の付いたスクリーンショットを残す (成功したときも見返せるように)。 */
export async function shot(session, name) {
  mkdirSync(join(out(), "shots"), { recursive: true });
  writeFileSync(join(out(), "shots", `${slug(name)}.png`), await session.screenshot());
}

/** `label` の付いたウィンドウ (Tauri のウィンドウのラベル) に切り替える。見つけたハンドルを返す。 */
export async function switchToTauriWindow(session, label) {
  for (const handle of await session.windowHandles()) {
    await session.switchToWindow(handle);
    const found = await session
      .execute("return window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label ?? null")
      .catch(() => null);
    if (found === label) return handle;
  }
  throw new Error(`no window labelled "${label}"`);
}

/** バックエンドの Tauri のコマンドを、ページの中から呼ぶ (画面の操作で代えられない準備だけに使う)。 */
export function invoke(session, cmd, args = {}) {
  return session.run((cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args), cmd, args);
}

/** 今のウィンドウで CSP が止めたもの (`window.__E2E_CSP__`) を返して空にする。わざと止めさせたものを確かめた後に使う。 */
export function takeCspViolations(session) {
  return session.execute("return (window.__E2E_CSP__ ?? []).splice(0)");
}

/**
 * 筋書きの最後の手順: すべてのウィンドウで、CSP が止めたものが 0 件であることを確かめる (tests/e2e/README.md#csp-の違反)。
 * CSP が要るもの (wasm・Worker・blob の画像・IPC) を止めても、画面の一部が静かに動かなくなるだけで、ほかの手順では気づけないため。
 */
export function checkNoCspViolations(app) {
  step(app, "has no CSP violations", async () => {
    const s = app.session;
    // 読み込み直したページ (エディタの言語の手順) には集める仕掛けが無いので入れ直す。前の違反は buffered の報告で届く
    await captureLogs(s);
    // 違反のイベントと報告は非同期に届くので、少し待ってから読む
    await new Promise((r) => setTimeout(r, 500));
    const current = await s.windowHandle();
    const found = {};
    for (const handle of await s.windowHandles()) {
      await s.switchToWindow(handle);
      const label = await s
        .execute("return window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label ?? null")
        .catch(() => null);
      // 画面に出た CSP のエラー (集める仕掛けを入れる前に止まり、イベントも出なかったもの。エディタのビューポートの wasm など) も数える
      const violations = await s.execute(`
        const shown = (document.body?.innerText ?? "").split("\\n")
          .filter((line) => line.includes("Content Security Policy"))
          .map((line) => ({ shown: line.trim().slice(0, 500) }));
        return [...window.__E2E_CSP__, ...shown];
      `);
      if (violations.length > 0) found[label ?? handle] = violations;
    }
    await s.switchToWindow(current);
    assert.deepEqual(found, {}, `CSP blocked something:\n${JSON.stringify(found, null, 2)}`);
  });
}
