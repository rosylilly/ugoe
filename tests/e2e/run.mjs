#!/usr/bin/env node
// 本物の Tauri のアプリ (エディタ・ビューア・スタジオ) を起動して操作する e2e テストを実行する (`just e2e`。tests/e2e/README.md)。
//
//   node tests/e2e/run.mjs [--only editor,viewer,studio] [--no-build] [--headed] [--out DIR]
//
// 1. アプリをビルドする: `tauri build --debug --no-bundle` (フロントエンドを埋め込んだデバッグビルド。Vite は要らない)。
//    できた実行ファイルを <out>/bin/ に写す (後の `cargo build` で target/debug のものが置き換わっても使えるように)。
// 2. 画面を用意する: 既定では Xvfb を起動する (利用者の画面にウィンドウを出さない)。`--headed` なら今の DISPLAY を使う。
// 3. アプリごとに、テスト用の XDG のディレクトリ (設定・データ・キャッシュ) と D-Bus のセッションで tauri-driver を起動し、
//    specs/<アプリ>.test.mjs を node:test で流す。利用者の設定・プロファイル・WebView のデータ・鍵の保管庫には触らない。
//
// 結果は <out> (既定 target/e2e) に残る: <アプリ>/driver.log (アプリと WebKitWebDriver の出力)、
// <アプリ>/*.console.json (ページの console)、<アプリ>/failures/<手順>/ (失敗したときのスクリーンショットとログ)、
// <アプリ>/shots/ (途中のスクリーンショット)。
//
// 要るもの (Linux): tauri-driver (`cargo install tauri-driver --locked`。TAURI_DRIVER で指定できる)、
// WebKitWebDriver (Debian・Ubuntu の webkit2gtk-driver。WEBKIT_WEBDRIVER で指定できる)、Xvfb (--headed でなければ)、
// dbus-run-session、アプリのビルドに要るもの (apps/editor/README.md)。
// Windows と macOS では流さない (macOS の WKWebView には WebDriver が無い。docs/testing.md#実機の-e2e-テスト)。

import { spawn, spawnSync } from "node:child_process";
import {
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { createServer } from "node:net";
import { homedir } from "node:os";
import { delimiter, join, resolve } from "node:path";

const here = import.meta.dirname;
const root = resolve(here, "..", "..");
const target = process.env.CARGO_TARGET_DIR ?? join(root, "target");

/** アプリの名前と、pnpm のパッケージ・実行ファイル。 */
const APPS = {
  editor: { package: "@ugoe/editor", binary: "ugoe-editor" },
  viewer: { package: "@ugoe/viewer", binary: "ugoe-viewer" },
  studio: { package: "@ugoe/studio", binary: "ugoe-studio" },
};

const args = process.argv.slice(2);
function option(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
}
const only = option("--only", Object.keys(APPS).join(",")).split(",");
for (const app of only) {
  if (!(app in APPS)) {
    console.error(`unknown app: ${app} (choose from ${Object.keys(APPS).join(", ")})`);
    process.exit(2);
  }
}
const build = !args.includes("--no-build");
const headed = args.includes("--headed");
const out = resolve(option("--out", join(target, "e2e")));

if (process.platform !== "linux") {
  console.error("the e2e tests run on Linux only (see docs/testing.md#実機の-e2e-テスト)");
  process.exit(2);
}

// --- 道具 ---

/** PATH (と追加の候補) から実行ファイルを探す。 */
function which(name, extra = []) {
  for (const dir of [...(process.env.PATH ?? "").split(delimiter), ...extra]) {
    const path = join(dir, name);
    if (dir && existsSync(path)) return path;
  }
  return null;
}

const tools = {
  tauriDriver:
    process.env.TAURI_DRIVER ?? which("tauri-driver", [join(homedir(), ".cargo", "bin")]),
  webkitDriver: process.env.WEBKIT_WEBDRIVER ?? which("WebKitWebDriver"),
  xvfb: headed ? "-" : which("Xvfb"),
  dbus: which("dbus-run-session"),
};
const missing = [
  [tools.tauriDriver, "tauri-driver: cargo install tauri-driver --locked (or set TAURI_DRIVER)"],
  [
    tools.webkitDriver,
    "WebKitWebDriver: sudo apt-get install webkit2gtk-driver (or set WEBKIT_WEBDRIVER)",
  ],
  [tools.xvfb, "Xvfb: sudo apt-get install xvfb (or pass --headed to use the current display)"],
  [tools.dbus, "dbus-run-session: sudo apt-get install dbus"],
].filter(([found]) => !found);
if (missing.length > 0) {
  console.error("missing tools:");
  for (const [, hint] of missing) console.error(`  ${hint}`);
  process.exit(2);
}
if (headed && !process.env.DISPLAY) {
  console.error("--headed needs DISPLAY");
  process.exit(2);
}

// --- ビルド ---

const bin = join(out, "bin");
mkdirSync(bin, { recursive: true });
for (const app of only) {
  const { package: pkg, binary } = APPS[app];
  if (build) {
    console.log(`\n== build ${app} (tauri build --debug --no-bundle)`);
    const started = Date.now();
    const result = spawnSync(
      "pnpm",
      ["--filter", pkg, "tauri", "build", "--debug", "--no-bundle", "--ci"],
      { cwd: root, stdio: "inherit" },
    );
    if (result.status !== 0) {
      console.error(`building ${app} failed`);
      process.exit(1);
    }
    console.log(`built ${app} in ${Math.round((Date.now() - started) / 1000)}s`);
    copyFileSync(join(target, "debug", binary), join(bin, binary));
  } else if (!existsSync(join(bin, binary))) {
    console.error(`${join(bin, binary)} does not exist: run without --no-build first`);
    process.exit(2);
  }
}

// --- 画面 ---

/**
 * X の画面の番号 `number` が使われていそうか。ロックファイル (/tmp/.X<n>-lock)・ファイルのソケット (/tmp/.X11-unix/X<n>)・
 * 抽象ソケット (@/tmp/.X11-unix/X<n>。/proc/net/unix に出る) のどれかがあれば使われている。
 * 抽象ソケットはネットワークの名前空間で共有され、/tmp を別に持つサンドボックスや WSLg の画面からはロックファイルが見えないことがある。
 */
function displayInUse(number, abstractSockets) {
  return (
    existsSync(`/tmp/.X${number}-lock`) ||
    existsSync(`/tmp/.X11-unix/X${number}`) ||
    abstractSockets.has(`@/tmp/.X11-unix/X${number}`)
  );
}

/** /proc/net/unix に出ている抽象ソケットの名前 (`@` 始まり)。読めなければ空。 */
function abstractUnixSockets() {
  try {
    return new Set(
      readFileSync("/proc/net/unix", "utf8")
        .split("\n")
        .map((line) => line.trim().split(/\s+/)[7])
        .filter((name) => name?.startsWith("@")),
    );
  } catch {
    return new Set();
  }
}

/**
 * Xvfb を番号 `number` で起動する。準備ができたら (-displayfd に番号が書かれたら) プロセスを、
 * 起動できなければ (番号を同時に選んだ別の Xvfb に取られたなど) Xvfb の標準エラーを持つ Error を返す。
 */
function startXvfb(number) {
  const child = spawn(
    tools.xvfb,
    [
      `:${number}`,
      "-displayfd",
      "3",
      "-screen",
      "0",
      "1920x1080x24",
      "-nolisten",
      "tcp",
      "-nolisten",
      "unix",
    ],
    { stdio: ["ignore", "ignore", "pipe", "pipe"] },
  );
  let stderr = "";
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  return new Promise((resolveChild, reject) => {
    let text = "";
    child.stdio[3].on("data", (chunk) => {
      text += chunk;
      if (text.includes("\n")) resolveChild(child);
    });
    child.on("error", reject);
    child.on("exit", (code) =>
      reject(new Error(`Xvfb :${number} exited (${code}):\n${stderr.slice(-2000)}`)),
    );
  });
}

/** @type {import("node:child_process").ChildProcess | null} */
let xvfb = null;
let display = process.env.DISPLAY;
if (!headed) {
  // 番号は Xvfb に選ばせず (-displayfd だけだと、WSLg のように今の画面がロックファイルを作らない環境で :0 を選び、
  // 今の画面とぶつかる)、使われていない番号を探す。同じ機械で並行に流すと (並列のエージェントなど) 2 つが同じ番号を選びうるので、
  // 始める番号をプロセスごとにずらし、起動できなければ次の番号で試し直す (先にロックファイルとソケットを取った方が使い、もう一方は起動に失敗して終わる)。
  // -nolisten unix: /tmp/.X11-unix にソケットを作らない (WSLg では書けない)。Linux のクライアントは抽象ソケットでつなぐ。
  let number = 99 + (process.pid % 400);
  let lastError = null;
  for (let attempt = 0; attempt < 20 && xvfb === null; attempt++, number++) {
    const sockets = abstractUnixSockets();
    while (displayInUse(number, sockets)) number++;
    try {
      xvfb = await startXvfb(number);
      display = `:${number}`;
    } catch (e) {
      lastError = e;
    }
  }
  if (xvfb === null) {
    console.error(`could not start Xvfb: ${lastError?.message}`);
    process.exit(1);
  }
  console.log(`\nXvfb on ${display}`);
}
/** 動いている tauri-driver (dbus-run-session ごとのプロセスグループ)。 */
let driverGroup = null;
/** 動いている筋書き (node --test)。 */
let testProcess = null;
function stopDriver() {
  if (driverGroup === null) return;
  try {
    // dbus-run-session の下の tauri-driver・WebKitWebDriver・アプリをまとめて止める
    process.kill(-driverGroup, "SIGTERM");
  } catch {
    // もう終わっている
  }
  driverGroup = null;
}
process.on("exit", () => {
  testProcess?.kill();
  stopDriver();
  xvfb?.kill();
});
process.on("SIGINT", () => process.exit(130));
process.on("SIGTERM", () => process.exit(143));

// --- テスト ---

/** 空いている TCP のポートを 1 つ返す。 */
function freePort() {
  return new Promise((resolvePort) => {
    const server = createServer();
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolvePort(port));
    });
  });
}

/** ドライバが要求を受けられるようになるまで待つ。 */
async function waitForDriver(url, driver) {
  const end = Date.now() + 20_000;
  while (Date.now() < end) {
    if (driver.exitCode !== null) throw new Error(`tauri-driver exited (${driver.exitCode})`);
    try {
      const res = await fetch(`${url}/status`);
      if (res.ok) return;
    } catch {
      // まだ
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error("tauri-driver did not start");
}

const results = [];
for (const app of only) {
  console.log(`\n== ${app}`);
  const dir = join(out, app);
  rmSync(dir, { recursive: true, force: true });
  const xdg = join(dir, "xdg");
  const appEnv = {
    // 利用者の設定・データ (プロファイル・API の鍵・WebView の localStorage) に触らない
    XDG_CONFIG_HOME: join(xdg, "config"),
    XDG_DATA_HOME: join(xdg, "data"),
    XDG_CACHE_HOME: join(xdg, "cache"),
    XDG_STATE_HOME: join(xdg, "state"),
    DISPLAY: display,
    // Wayland に出ないように (WSLg など)。GTK は DISPLAY の X に描く
    WAYLAND_DISPLAY: "",
    GDK_BACKEND: "x11",
    // 画面なしの環境の GPU はソフトウェア (llvmpipe)。WebKit の合成も同じ
    LIBGL_ALWAYS_SOFTWARE: process.env.LIBGL_ALWAYS_SOFTWARE ?? "1",
    // 言語を固定する (OS の言語に合わせるところを、どの環境でも同じにする)
    LANG: "en_US.UTF-8",
    LC_ALL: "en_US.UTF-8",
    LANGUAGE: "en",
  };
  for (const sub of Object.values(appEnv).filter((v) => v.startsWith(xdg))) {
    mkdirSync(sub, { recursive: true });
  }
  const work = join(dir, "work");
  mkdirSync(work, { recursive: true });

  const port = await freePort();
  const nativePort = await freePort();
  const log = createWriteStream(join(dir, "driver.log"));
  // dbus-run-session: 新しい D-Bus のセッションで動かす (利用者の鍵の保管庫やアクセシビリティのバスに触らない)
  const driver = spawn(
    tools.dbus,
    [
      "--",
      tools.tauriDriver,
      "--port",
      String(port),
      "--native-port",
      String(nativePort),
      "--native-driver",
      tools.webkitDriver,
    ],
    { env: { ...process.env, ...appEnv }, stdio: ["ignore", "pipe", "pipe"], detached: true },
  );
  driverGroup = driver.pid;
  driver.stdout.pipe(log);
  driver.stderr.pipe(log);
  const url = `http://127.0.0.1:${port}`;
  const started = Date.now();
  let status;
  try {
    await waitForDriver(url, driver);
    // 非同期で待つ (止める合図を受けたときに、後片付けできるように)
    const test = spawn(
      process.execPath,
      ["--test", "--test-concurrency=1", join(here, "specs", `${app}.test.mjs`)],
      {
        cwd: root,
        stdio: "inherit",
        env: {
          ...process.env,
          E2E_DRIVER: url,
          E2E_APP: join(bin, APPS[app].binary),
          E2E_OUT: dir,
          E2E_WORK: work,
          E2E_DISPLAY: display,
        },
      },
    );
    testProcess = test;
    status = await new Promise((done) => test.on("exit", (code) => done(code)));
  } catch (e) {
    console.error(e.message);
    status = 1;
  } finally {
    stopDriver();
  }
  const seconds = Math.round((Date.now() - started) / 1000);
  results.push({ app, ok: status === 0, seconds });
}

console.log("\n== e2e results");
for (const { app, ok, seconds } of results) {
  console.log(`  ${ok ? "ok  " : "FAIL"} ${app} (${seconds}s)`);
}
console.log(`  results: ${out}`);
process.exit(results.every((r) => r.ok) ? 0 : 1);
