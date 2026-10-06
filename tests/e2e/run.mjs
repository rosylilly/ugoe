#!/usr/bin/env node
// 本物の Tauri のアプリ (エディタ・ビューア・スタジオ) を起動して操作する e2e テストを実行する (`just e2e`。tests/e2e/README.md)。
//
//   node tests/e2e/run.mjs [--only editor,viewer,studio] [--no-build] [--headed] [--out DIR] [--manual] [--update-images]
//
// `--manual` は筋書きの代わりに specs/<アプリ>.manual.mjs を流し、マニュアルの画像を <out>/<アプリ>/manual/ に撮る。
// `--update-images` は、撮った画像の色を減らして docs/manual/images に写す (`--manual` を含む。README の「マニュアルの画像」)。
//
// 1. アプリをビルドする: `tauri build --debug --no-bundle` (フロントエンドを埋め込んだデバッグビルド。Vite は要らない)。
//    できた実行ファイルを <out>/bin/ に写す (後の `cargo build` で target/debug のものが置き換わっても使えるように)。
// 2. 画面を用意する: 既定では Xvfb を起動する (利用者の画面にウィンドウを出さない)。`--headed` なら今の DISPLAY を使う。
// 3. アプリごとに、テスト用の XDG のディレクトリ (設定・データ・キャッシュ) と D-Bus のセッションで tauri-driver を起動し、
//    specs/<アプリ>.test.mjs を node:test で流す。利用者の設定・プロファイル・WebView のデータ・鍵の保管庫には触らない。
//    エディタの D-Bus のセッションでは、使い捨ての鍵の保管庫 (gnome-keyring-daemon) を動かす (API キーの保存の筋書き。T0050)。
//    エディタはこの計算機の外への HTTP(S) を、手元の止める口 (プロキシ) に向ける。外へ出ようとしたら止め、その手順を失敗にする
//    (README の「外に出さない」)。
//
// 結果は <out> (既定 target/e2e) に残る: <アプリ>/driver.log (アプリと WebKitWebDriver の出力)、
// <アプリ>/*.console.json (ページの console)、<アプリ>/failures/<手順>/ (失敗したときのスクリーンショットとログ)、
// <アプリ>/shots/ (途中のスクリーンショット)。
//
// 要るもの (Linux): tauri-driver (`cargo install tauri-driver --locked`。TAURI_DRIVER で指定できる)、
// WebKitWebDriver (Debian・Ubuntu の webkit2gtk-driver。WEBKIT_WEBDRIVER で指定できる)、Xvfb (--headed でなければ)、
// dbus-run-session、アプリのビルドに要るもの (apps/editor/README.md)。あれば gnome-keyring-daemon (無ければ、保管庫の要る筋書きを飛ばす)。
// Windows と macOS では流さない (macOS の WKWebView には WebDriver が無い。docs/testing.md#実機の-e2e-テスト)。

import { spawn, spawnSync } from "node:child_process";
import {
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { createServer as createHttpServer } from "node:http";
import { createServer } from "node:net";
import { homedir, tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { exportMbxPath } from "../../scripts/lib/mbx.mjs";

// mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
exportMbxPath();

const here = import.meta.dirname;
const root = resolve(here, "..", "..");
const target = process.env.CARGO_TARGET_DIR ?? join(root, "target");
/** マニュアルの画像の置き場 (`--update-images` が写す先)。 */
const manualImages = join(root, "docs", "manual", "images");

/**
 * アプリの名前と、pnpm のパッケージ・実行ファイル。`keyring`: D-Bus のセッションで鍵の保管庫を動かす (API キーの保存)。
 * `sidecars`: 実行ファイルの隣に置くワークスペースのバイナリ (エディタは同梱の ugoe-mcp を隣に探す。「Connect agents」の登録の
 * コマンドと Claude Desktop の .mcpb)。`offline`: 外への HTTP(S) を止める口に向ける (README の「外に出さない」)。
 */
const APPS = {
  editor: {
    package: "@ugoe/editor",
    binary: "ugoe-editor",
    keyring: true,
    sidecars: ["ugoe-mcp"],
    offline: true,
  },
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
const updateImages = args.includes("--update-images");
const manual = updateImages || args.includes("--manual");
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
  // 無くても流せる (保管庫の要る筋書きを飛ばす)。GNOME_KEYRING_DAEMON を空にすると、あっても使わない
  keyring: process.env.GNOME_KEYRING_DAEMON ?? which("gnome-keyring-daemon"),
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
/** 保管庫が無いので飛ばす筋書きの理由 (保管庫があれば null)。 */
const keyringSkip = tools.keyring
  ? null
  : "no gnome-keyring-daemon: sudo apt-get install gnome-keyring (or set GNOME_KEYRING_DAEMON)";
if (keyringSkip && only.some((app) => APPS[app].keyring)) {
  console.warn(`skipping the scenarios that need a keyring (${keyringSkip})`);
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
    for (const sidecar of APPS[app].sidecars ?? []) {
      const built = spawnSync("cargo", ["build", "-p", sidecar], { cwd: root, stdio: "inherit" });
      if (built.status !== 0) {
        console.error(`building ${sidecar} failed`);
        process.exit(1);
      }
    }
    console.log(`built ${app} in ${Math.round((Date.now() - started) / 1000)}s`);
    for (const name of [binary, ...(APPS[app].sidecars ?? [])]) {
      // mbx が置き場から戻した成果物は読むだけの権限なので、写しも上書きできない。消してから写す
      rmSync(join(bin, name), { force: true });
      copyFileSync(join(target, "debug", name), join(bin, name));
    }
  } else {
    for (const name of [binary, ...(APPS[app].sidecars ?? [])]) {
      if (!existsSync(join(bin, name))) {
        console.error(`${join(bin, name)} does not exist: run without --no-build first`);
        process.exit(2);
      }
    }
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
/** 鍵の保管庫に使わせている XDG_RUNTIME_DIR (終わったら消す)。 */
let runtimeDir = null;
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
/**
 * 鍵の保管庫に使わせた XDG_RUNTIME_DIR を消す。消せたら true。D-Bus のセッションで起動されたもの (xdg-document-portal は
 * FUSE で `doc` をつなぐ) がまだ止まっていなければ消せない。`warn` なら消せなかったことを出して諦める (/tmp の下に残るだけ)。
 */
function removeRuntimeDir(warn) {
  if (runtimeDir === null) return true;
  try {
    rmSync(runtimeDir, { recursive: true, force: true });
  } catch (e) {
    if (!warn) return false;
    console.warn(`could not remove ${runtimeDir}: ${e.message}`);
  }
  runtimeDir = null;
  return true;
}
/**
 * マニュアルの画像を撮るときの、決まった短い場所 (README の「マニュアルの画像」)。画面に出るパス (エージェントの作業の場所・
 * 同梱の ugoe-mcp・保存した .mcpb) に、流した機械の場所 (ユーザー名・worktree・ビルドのキャッシュ) を写さないため。
 */
const MANUAL_ROOT = "/tmp/ugoe-manual";
/** 決まった場所を使っている印 (`--manual` を同じ機械で同時に流さない)。 */
const manualLock = join(MANUAL_ROOT, "lock");
let manualLocked = false;
if (manual) {
  mkdirSync(MANUAL_ROOT, { recursive: true });
  if (existsSync(manualLock)) {
    const pid = Number(readFileSync(manualLock, "utf8"));
    let alive = false;
    try {
      process.kill(pid, 0);
      alive = Number.isInteger(pid) && pid > 0;
    } catch {
      // 前の実行が残した印
    }
    if (alive) {
      console.error(
        `another --manual run (pid ${pid}) is using ${MANUAL_ROOT}; wait for it to finish`,
      );
      process.exit(2);
    }
    rmSync(manualLock, { force: true });
  }
  writeFileSync(manualLock, String(process.pid), { flag: "wx" });
  manualLocked = true;
}
process.on("exit", () => {
  if (manualLocked) rmSync(manualLock, { force: true });
  testProcess?.kill();
  stopDriver();
  removeRuntimeDir(true);
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

/**
 * 外への HTTP(S) を止める口 (README の「外に出さない」)。プロキシとして待ち受け、来た要求 (HTTPS の CONNECT と HTTP) を
 * 403 で断って `blocked` に覚える。アプリは HTTPS_PROXY などでここに向け、NO_PROXY でループバック (偽の AI のプロバイダ・
 * MCP のブリッジ・WebDriver) だけを直に通す。
 */
async function startNetworkGuard() {
  const blocked = [];
  const server = createHttpServer((req, res) => {
    blocked.push(`${req.method} ${req.url}`);
    res.writeHead(403, { "content-type": "text/plain" });
    res.end("blocked by the e2e network guard\n");
  });
  server.on("connect", (req, socket) => {
    blocked.push(`CONNECT ${req.url}`);
    socket.end("HTTP/1.1 403 Forbidden\r\ncontent-length: 0\r\n\r\n");
  });
  await new Promise((done, fail) => {
    server.once("error", fail);
    server.listen(0, "127.0.0.1", done);
  });
  return {
    url: `http://127.0.0.1:${server.address().port}`,
    blocked,
    close: () => new Promise((done) => server.close(() => done())),
  };
}

/** 使い捨ての鍵の保管庫のパスワード (保管庫はテストの間だけのもので、秘密ではない)。 */
const KEYRING_PASSWORD = "ugoe-e2e";

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
    // エディタのファイルのダイアログの差し替え (デバッグビルドのバックエンドが、OS のダイアログの代わりにここの答えを読む。
    // apps/editor/src-tauri/src/dialog.rs、T1616)
    UGOE_EDITOR_DIALOG_ANSWERS: join(dir, "dialog-answers"),
  };
  for (const sub of Object.values(appEnv).filter((v) => v.startsWith(xdg))) {
    mkdirSync(sub, { recursive: true });
  }
  mkdirSync(appEnv.UGOE_EDITOR_DIALOG_ANSWERS, { recursive: true });
  // マニュアルの画像は普通の Linux の画面で撮る (WSL の中で流しても、エディタの「Connect agents」に WSL の案内を出さない)
  if (manual) appEnv.WSL_DISTRO_NAME = undefined;
  const guard = APPS[app].offline ? await startNetworkGuard() : null;
  if (guard) {
    // ureq (AI のプロバイダ。crates/ugoe-ai) はこれらを読む。ループバックは止めずに通す
    for (const name of ["ALL_PROXY", "HTTPS_PROXY", "HTTP_PROXY"]) {
      appEnv[name] = guard.url;
      appEnv[name.toLowerCase()] = guard.url;
    }
    appEnv.NO_PROXY = appEnv.no_proxy = "127.0.0.1,localhost,::1";
  }
  // マニュアルの画像は、作業の場所と実行ファイルを決まった短い場所に置いて撮る (画面に出るパスがどの機械でも同じになる)。
  // 実行ファイルは写す (エディタは自分の実行ファイルのシンボリックリンクを辿った場所の隣に ugoe-mcp を探し、その場所を画面に出す)
  const work = manual ? join(MANUAL_ROOT, "work") : join(dir, "work");
  let appBin = bin;
  if (manual) {
    rmSync(work, { recursive: true, force: true });
    appBin = join(MANUAL_ROOT, "bin");
    rmSync(appBin, { recursive: true, force: true });
    mkdirSync(appBin, { recursive: true });
    for (const name of [APPS[app].binary, ...(APPS[app].sidecars ?? [])]) {
      copyFileSync(join(bin, name), join(appBin, name));
    }
  }
  mkdirSync(work, { recursive: true });

  const port = await freePort();
  const nativePort = await freePort();
  const log = createWriteStream(join(dir, "driver.log"));
  const driverCommand = [
    tools.tauriDriver,
    "--port",
    String(port),
    "--native-port",
    String(nativePort),
    "--native-driver",
    tools.webkitDriver,
  ];
  const keyring = Boolean(APPS[app].keyring && tools.keyring);
  if (keyring) {
    // 使い捨ての鍵の保管庫 (README の「鍵の保管庫」)。XDG_RUNTIME_DIR を新しいディレクトリにする: gnome-keyring は
    // そこの keyring/control で動いている保管庫を探すので、利用者の保管庫 (/run/user/<uid>/keyring) を見つけて使わないように。
    // 制御のソケットのパスは 108 バイトまでなので、短い一時ディレクトリに置く (target/ の下は長すぎることがある)
    runtimeDir = mkdtempSync(join(tmpdir(), "ugoe-e2e-"));
    appEnv.XDG_RUNTIME_DIR = runtimeDir;
  }
  // dbus-run-session: 新しい D-Bus のセッションで動かす (利用者の鍵の保管庫やアクセシビリティのバスに触らない)
  const driver = spawn(
    tools.dbus,
    keyring
      ? [
          "--",
          "sh",
          "-c",
          // 保管庫 (データは XDG_DATA_HOME/keyrings。毎回消す) を決まったパスワードで作って解錠し、Secret Service だけを動かす。
          // gnome-keyring は空のパスワードではログインの保管庫を作らない。D-Bus のセッションが終わると止まる
          'printf %s "$2" | "$1" --unlock --components=secrets --control-directory="$XDG_RUNTIME_DIR/keyring" >/dev/null || exit 3; shift 2; exec "$@"',
          "sh",
          tools.keyring,
          KEYRING_PASSWORD,
          ...driverCommand,
        ]
      : ["--", ...driverCommand],
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
      [
        "--test",
        "--test-concurrency=1",
        join(here, "specs", manual ? `${app}.manual.mjs` : `${app}.test.mjs`),
      ],
      {
        cwd: root,
        stdio: "inherit",
        env: {
          ...process.env,
          E2E_DRIVER: url,
          E2E_APP: join(appBin, APPS[app].binary),
          E2E_OUT: dir,
          E2E_WORK: work,
          // アプリの設定の場所 (起動の前に置くもの。スタジオの最近のプロファイルなど)
          E2E_CONFIG_HOME: appEnv.XDG_CONFIG_HOME,
          // エディタのファイルのダイアログの答えを置く場所
          E2E_DIALOG_ANSWERS: appEnv.UGOE_EDITOR_DIALOG_ANSWERS,
          E2E_DISPLAY: display,
          // 鍵の保管庫が動いているか (`1`)、飛ばす理由 (README の「鍵の保管庫」)
          ...(keyring ? { E2E_KEYRING: "1" } : { E2E_KEYRING_SKIP: keyringSkip ?? "no keyring" }),
          // 利用者の画面に出しているか (マニュアルの画像の筋書きが、OS 全体に効くホットキーを外す)
          ...(headed ? { E2E_HEADED: "1" } : {}),
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
  if (guard) {
    await guard.close();
    if (guard.blocked.length > 0) {
      console.error(
        `${app} tried to reach the network (blocked):\n  ${guard.blocked.join("\n  ")}`,
      );
      status = 1;
    }
  }
  // D-Bus のセッションで起動されたもの (FUSE の `doc`) が止まるまで、少し待ちながら消し直す
  for (let attempt = 0; attempt < 40 && !removeRuntimeDir(attempt === 39); attempt++) {
    await new Promise((r) => setTimeout(r, 250));
  }
  const seconds = Math.round((Date.now() - started) / 1000);
  results.push({ app, ok: status === 0, seconds });
}

if (updateImages) {
  console.log("\n== manual images");
  for (const { app, ok } of results) {
    if (!ok) {
      console.log(`  ${app}: not updated (the run failed)`);
      continue;
    }
    for (const name of readdirSync(join(out, app, "manual")).filter((f) => f.endsWith(".png"))) {
      const dest = join(manualImages, name);
      // 256 色に減らす (前の画像と同じく、リポジトリを重くしない。色の多いアバターの絵も読める)
      const result = spawnSync("convert", [
        join(out, app, "manual", name),
        "-strip",
        "-dither",
        "None",
        "-colors",
        "256",
        `png8:${dest}`,
      ]);
      if (result.status !== 0) {
        console.error(`  could not write ${dest}: ${result.stderr}`);
        process.exitCode = 1;
        continue;
      }
      console.log(`  ${app}: ${name} (${Math.round(statSync(dest).size / 1024)} KB)`);
    }
  }
}

console.log("\n== e2e results");
for (const { app, ok, seconds } of results) {
  console.log(`  ${ok ? "ok  " : "FAIL"} ${app} (${seconds}s)`);
}
console.log(`  results: ${out}`);
process.exit(results.every((r) => r.ok) && !process.exitCode ? 0 : 1);
