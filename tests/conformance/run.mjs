#!/usr/bin/env node
// 配布物の適合テストを実行する (`just conformance`。tests/conformance/README.md)。
//
//   node tests/conformance/run.mjs [--only c,wasm,wasm-webgpu,unity,godot] [--cases A,B] [--out DIR]
//                                  [--no-web-build] [--no-godot-build]
//
// 1. 参照を作る: `ugoe-conformance reference` (計画と参照の評価の結果) と `ugoe render` (参照の画像)
// 2. C ABI: tests/conformance/c/runner.c を libugoe_ffi.a にリンクして実行し、評価の結果を比べる
// 3. wasm: npm パッケージ (bindings/web/dist) をヘッドレスの Chrome の WebGL2 (wasm) と WebGPU (wasm-webgpu) で動かし、
//    評価の結果と画像を比べる
// 4. Unity: UPM パッケージの Unity に依存しない C# の層 (Runtime/Core) を dotnet で動かし、評価の結果を比べる
// 5. Godot: GDExtension で描いた画像を比べる (bindings/godot/example/tests/conformance.gd)
//
// 要るもの: C コンパイラ (`cc`。CC で変えられる)、Chrome (CHROME で指定。既定は google-chrome など)、
// wasm-bindgen-cli (パッケージのビルド)、参照の画像を描く GPU (無ければ Mesa の lavapipe / llvmpipe)、
// .NET SDK 9 以降 (unity)、Godot 4.4 以降 (GODOT で指定。既定は godot) と、画面が無ければ xvfb-run (godot)。
// `--only` を付けないときは、unity と godot の道具が無ければそのバインディングを飛ばす (ほかは止まる)。
// `--only` で名前を挙げたバインディングは、道具が無ければ入れ方を示して止まる。

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
import { tmpdir } from "node:os";
import { extname, join, normalize, resolve } from "node:path";

const here = import.meta.dirname;
const root = resolve(here, "..", "..");
const target = process.env.CARGO_TARGET_DIR ?? join(root, "target");
const exe = (name) => join(target, "debug", process.platform === "win32" ? `${name}.exe` : name);

const BINDINGS = ["c", "wasm", "wasm-webgpu", "unity", "godot"];
/** `--only` を付けないとき、道具が無ければ飛ばすバインディング。 */
const OPTIONAL = new Set(["unity", "godot"]);
/** 画像を比べるバインディング。 */
const IMAGES = new Set(["wasm", "wasm-webgpu", "godot"]);

const args = process.argv.slice(2);
function option(name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
}
if (args.includes("--backend")) {
  console.error(
    "--backend was removed: use `--only wasm` (WebGL2) or `--only wasm-webgpu` (WebGPU)",
  );
  process.exit(2);
}
const explicit = args.includes("--only");
const only = new Set(option("--only", BINDINGS.join(",")).split(","));
const out = resolve(option("--out", join(target, "conformance")));
const cases = option("--cases", null);
const webBuild = !args.includes("--no-web-build");
const godotBuild = !args.includes("--no-godot-build");
for (const name of only) {
  if (!BINDINGS.includes(name)) {
    console.error(`unknown binding \`${name}\` (expected ${BINDINGS.join(", ")})`);
    process.exit(2);
  }
}

/** コマンドを実行し、失敗したらその終了コードで終える。 */
function run(cmd, cmdArgs, options = {}) {
  const r = spawnSync(cmd, cmdArgs, { cwd: root, stdio: "inherit", ...options });
  if (r.error) {
    console.error(`error: failed to run ${cmd}: ${r.error.message}`);
    process.exit(2);
  }
  if (r.status !== 0) process.exit(r.status ?? 2);
}

/** 比べ合わせを実行し、通ったかを返す。`mode` は "frames"・"images" (評価の結果と画像)・"images-only"。 */
function compare(binding, mode) {
  const flags = { frames: [], images: ["--images"], "images-only": ["--images-only"] }[mode];
  const r = spawnSync(
    exe("ugoe-conformance"),
    ["compare", "--out", out, "--binding", binding, ...flags],
    { cwd: root, stdio: "inherit" },
  );
  return r.status === 0;
}

// --- 0. 要るものを調べる (GPU はビルドの後) ---
const cc = process.env.CC ?? "cc";
const godot = process.env.GODOT ?? "godot";
const headless = !process.env.DISPLAY && !process.env.WAYLAND_DISPLAY;
const chrome =
  only.has("wasm") || only.has("wasm-webgpu") ? (process.env.CHROME ?? findChrome()) : null;
const skipped = {};
{
  /** バインディングごとの足りない道具 (と入れ方)。 */
  const missing = new Map();
  const need = (binding, message) => {
    if (only.has(binding)) missing.set(binding, [...(missing.get(binding) ?? []), message]);
  };
  if (!works(cc, ["--version"])) {
    need(
      "c",
      `C compiler \`${cc}\` was not found: install one (e.g. \`sudo apt-get install build-essential\`) or set CC`,
    );
  }
  for (const binding of ["wasm", "wasm-webgpu"]) {
    if (!(chrome && works(chrome, ["--version"]))) {
      need(
        binding,
        `Chrome ${chrome ? `\`${chrome}\` does not run` : "was not found"}: install Google Chrome or Chromium, or set CHROME to its path`,
      );
    }
  }
  if ((only.has("wasm") || only.has("wasm-webgpu")) && webBuild) {
    const want = capture("node", [join(root, "scripts", "wasm.mjs"), "version"]);
    const have = capture("wasm-bindgen", ["--version"])?.split(/\s+/)[1];
    if (!want || have !== want) {
      for (const binding of ["wasm", "wasm-webgpu"]) {
        need(
          binding,
          `wasm-bindgen-cli ${want ?? ""} is required (found: ${have ?? "none"}):\n` +
            '    cargo install wasm-bindgen-cli --version "$(just wasm-bindgen-version)" --locked',
        );
      }
    }
  }
  const sdks = capture("dotnet", ["--list-sdks"]) ?? "";
  if (!sdks.split("\n").some((line) => Number.parseInt(line, 10) >= 9)) {
    need(
      "unity",
      `the .NET SDK 9 or later was not found (dotnet --list-sdks: ${sdks.replaceAll("\n", ", ") || "none"}):\n` +
        "    install it from https://dotnet.microsoft.com/download (e.g. `sudo apt-get install dotnet-sdk-9.0`)",
    );
  }
  if (!works(godot, ["--version"])) {
    need(
      "godot",
      `Godot \`${godot}\` was not found: install Godot 4.4 or later from https://godotengine.org/download ` +
        "and put it on PATH, or set GODOT to its path",
    );
  } else if (headless && !works("sh", ["-c", "command -v xvfb-run"])) {
    need("godot", "Godot needs a display: install xvfb-run (e.g. `sudo apt-get install xvfb`)");
  }
  const stop = [];
  for (const [binding, messages] of missing) {
    if (!explicit && OPTIONAL.has(binding)) {
      skipped[binding] = messages;
      only.delete(binding);
    } else {
      stop.push(...messages.map((m) => `${binding}: ${m}`));
    }
  }
  if (stop.length > 0) {
    console.error(
      `error: missing tools for \`just conformance\`:\n${[...new Set(stop)].map((m) => `  - ${m}`).join("\n")}`,
    );
    console.error(
      "  (`just conformance --only c` needs only the C compiler; choose the bindings with --only)",
    );
    process.exit(2);
  }
  for (const [binding, messages] of Object.entries(skipped)) {
    console.warn(
      `skipping ${binding} (missing tools):\n${messages.map((m) => `  - ${m}`).join("\n")}`,
    );
  }
}

// --- 1. 参照 ---
run("cargo", ["build", "-q", "-p", "ugoe-cli", "-p", "ugoe-conformance", "-p", "ugoe-ffi"]);
const images = [...only].some((b) => IMAGES.has(b));
if (images) {
  // 参照の画像を描く GPU があるか (小さなフィクスチャを描いてみる)。
  const probe = mkdtempSync(join(tmpdir(), "ugoe-conformance-gpu-"));
  const r = spawnSync(
    exe("ugoe"),
    ["render", "fixture:single_quad", "-o", join(probe, "gpu.png")],
    {
      cwd: root,
      encoding: "utf8",
    },
  );
  rmSync(probe, { recursive: true, force: true });
  if (r.status !== 0) {
    console.error(
      `error: the reference images need a GPU, and \`ugoe render\` failed:\n${r.stderr.trim()}\n` +
        "  Without a GPU, install Mesa's software rasterizers and select one:\n" +
        "    sudo apt-get install mesa-vulkan-drivers libvulkan1   # lavapipe, then WGPU_BACKEND=vulkan\n" +
        "    sudo apt-get install libegl1 libegl-mesa0 libgl1-mesa-dri   # llvmpipe, then WGPU_BACKEND=gl\n" +
        "  (`just conformance --only c` does not need a GPU)",
    );
    process.exit(2);
  }
}
rmSync(out, { recursive: true, force: true });
run(exe("ugoe-conformance"), ["reference", "--out", out, ...(cases ? ["--only", cases] : [])]);
const plan = JSON.parse(readFileSync(join(out, "plan.json"), "utf8"));
if (images) {
  for (const c of plan.cases.filter((c) => c.image)) {
    const png = join(out, "reference", `${c.name}.png`);
    const r = spawnSync(exe("ugoe"), ["render", ...c.render_args, "-o", png], {
      cwd: root,
      encoding: "utf8",
    });
    if (r.status !== 0) {
      console.error(`ugoe render failed for ${c.name}:\n${r.stderr}`);
      process.exit(2);
    }
  }
  console.log(`reference images: ${plan.cases.filter((c) => c.image).length}`);
}

const results = {};

// --- 2. C ABI ---
if (only.has("c")) {
  const runner = join(out, process.platform === "win32" ? "runner.exe" : "runner");
  const windows = process.platform === "win32";
  const lib = join(target, "debug", windows ? "ugoe_ffi.lib" : "libugoe_ffi.a");
  const system = { darwin: ["-framework", "CoreFoundation"], linux: ["-lm", "-lpthread", "-ldl"] };
  run(cc, [
    "-std=c11",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-pedantic",
    "-I",
    join(root, "crates", "ugoe-ffi", "include"),
    join(here, "c", "runner.c"),
    lib,
    ...(system[process.platform] ?? []),
    "-o",
    runner,
  ]);
  mkdirSync(join(out, "c"), { recursive: true });
  const r = spawnSync(runner, [join(out, "plan.txt"), join(out, "c")], { stdio: "inherit" });
  results.c = r.status === 0 && compare("c", "frames");
}

// --- 3. wasm (WebGL2 と WebGPU) ---
if (only.has("wasm") || only.has("wasm-webgpu")) {
  if (webBuild) run("pnpm", ["--filter", "@ugoe/runtime", "build"]);
  const dist = join(root, "bindings", "web", "dist");
  if (!existsSync(join(dist, "index.js"))) {
    console.error("bindings/web/dist is missing; run `just web-runtime`");
    process.exit(2);
  }
  for (const [binding, backend] of [
    ["wasm", "webgl2"],
    ["wasm-webgpu", "webgpu"],
  ]) {
    if (!only.has(binding)) continue;
    mkdirSync(join(out, binding), { recursive: true });
    results[binding] = (await runWeb(dist, binding, backend)) && compare(binding, "images");
  }
}

// --- 4. Unity (Runtime/Core を dotnet で) ---
if (only.has("unity")) {
  const r = spawnSync(
    "dotnet",
    [
      "run",
      "--project",
      join(here, "unity"),
      `-p:UgoeFfiDir=${join(target, "debug")}`,
      "--",
      join(out, "plan.json"),
      join(out, "unity"),
    ],
    { cwd: root, stdio: "inherit" },
  );
  results.unity = r.status === 0 && compare("unity", "frames");
}

// --- 5. Godot ---
if (only.has("godot")) {
  const example = join(root, "bindings", "godot", "example");
  // 拡張をビルドして見本のプロジェクトに置き、プロジェクトを読み込む (build.mjs --conformance からは済んでいる)。
  if (godotBuild) run("node", [join(root, "bindings", "godot", "build.mjs"), "--import"]);
  const godotArgs = [
    "--path",
    example,
    "--rendering-driver",
    "opengl3",
    "--script",
    "res://tests/conformance.gd",
    "--",
    join(out, "plan.json"),
    join(out, "godot"),
  ];
  const [cmd, cmdArgs] = headless
    ? ["xvfb-run", ["-a", "-s", "-screen 0 1280x1024x24", godot, ...godotArgs]]
    : [godot, godotArgs];
  const r = spawnSync(cmd, cmdArgs, { cwd: root, stdio: "inherit", timeout: 20 * 60 * 1000 });
  if (r.error) console.error(`godot: ${r.error.message}`);
  results.godot = r.status === 0 && compare("godot", "images-only");
}

console.log(`results: ${out}`);
for (const [binding, ok] of Object.entries(results))
  console.log(`${binding}: ${ok ? "ok" : "FAILED"}`);
for (const binding of Object.keys(skipped)) console.log(`${binding}: skipped (missing tools)`);
process.exit(Object.values(results).every(Boolean) ? 0 : 1);

/** ヘッドレスの Chrome で web/index.html を開き、ページが /done を送るまで待つ。 */
async function runWeb(dist, binding, backend) {
  const types = {
    ".html": "text/html",
    ".js": "text/javascript",
    ".json": "application/json",
    ".wasm": "application/wasm",
  };
  let finish;
  const done = new Promise((r) => {
    finish = r;
  });
  // ページからの要求が IDLE_MS のあいだ無ければ、止まったとみなす (WebGPU のアダプタを待ち続けるなど)。
  const IDLE_MS = 120 * 1000;
  let current = "(before the first case)";
  let idle;
  let rearm;
  const timeout = new Promise((r) => {
    rearm = () => {
      clearTimeout(idle);
      idle = setTimeout(() => r({ timeout: true }), IDLE_MS);
    };
  });
  rearm();
  const server = createServer((req, res) => {
    rearm();
    const url = new URL(req.url, "http://localhost");
    const send = (status, body, type = "text/plain") => {
      res.writeHead(status, { "content-type": type });
      res.end(body);
    };
    if (req.method === "POST") {
      const chunks = [];
      req.on("data", (c) => chunks.push(c));
      req.on("end", () => {
        const body = Buffer.concat(chunks);
        if (url.pathname === "/start") {
          current = body.toString("utf8");
        } else if (url.pathname === "/log") {
          console.error(`[${binding}] ${body}`);
        } else if (url.pathname === "/done") {
          finish(JSON.parse(body.toString("utf8")));
        } else if (/^\/result\/[\w-]+\.(json|png)$/.test(url.pathname)) {
          writeFileSync(join(out, binding, url.pathname.slice("/result/".length)), body);
        } else {
          return send(404, "not found");
        }
        send(200, "ok");
      });
      return;
    }
    // GET: ページ・パッケージ・計画・計画にあるモデルとモジュールだけを配る。
    let file;
    const m = url.pathname.match(/^\/file\/(\d+)\/(model|\d+)$/);
    if (url.pathname === "/" || url.pathname === "/index.html")
      file = join(here, "web", "index.html");
    else if (url.pathname === "/runner.js") file = join(here, "web", "runner.js");
    else if (url.pathname === "/plan.json") file = join(out, "plan.json");
    else if (url.pathname.startsWith("/pkg/")) {
      file = normalize(join(dist, url.pathname.slice("/pkg/".length)));
      if (!file.startsWith(dist)) return send(403, "forbidden");
    } else if (m) {
      const c = plan.cases[Number(m[1])];
      file = m[2] === "model" ? c?.model : c?.modules[Number(m[2])];
    }
    if (!file || !existsSync(file)) return send(404, "not found");
    send(200, readFileSync(file), types[extname(file)] ?? "application/octet-stream");
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const { port } = server.address();

  const profile = mkdtempSync(join(tmpdir(), "ugoe-conformance-chrome-"));
  const flags = [
    "--headless=new",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--enable-logging=stderr",
    "--no-default-browser-check",
    // GPU の無い環境でも同じ結果になるよう、ソフトウェアの実装 (SwiftShader) で描く。
    // WebGPU に ANGLE の SwiftShader の指定を重ねると、アダプタの要求が返らなくなる (Chrome 14x、WSL2 で確かめた)
    ...(backend === "webgpu"
      ? ["--enable-unsafe-webgpu", "--enable-features=Vulkan", "--use-webgpu-adapter=swiftshader"]
      : ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"]),
    ...(process.env.CI ? ["--no-sandbox"] : []),
    `http://127.0.0.1:${port}/?backend=${backend}`,
  ];
  // Chrome のログ (ページのコンソールを含む) は <出力>/<バインディング>/chrome.log に書く。
  const browser = spawn(chrome, flags, { stdio: ["ignore", "ignore", "pipe"] });
  browser.stderr.pipe(createWriteStream(join(out, binding, "chrome.log")));
  const exited = new Promise((r) => browser.on("exit", (code) => r({ exited: code })));
  const result = await Promise.race([done, exited, timeout]);
  clearTimeout(idle);
  browser.kill();
  await exited;
  server.close();
  rmSync(profile, { recursive: true, force: true });
  if (result.timeout || "exited" in result || result.error) {
    console.error(
      result.error ??
        (result.timeout
          ? `${binding}: no progress for ${IDLE_MS / 1000}s in ${current} (see chrome.log)`
          : `${binding}: Chrome exited (${result.exited}); see chrome.log`),
    );
    return false;
  }
  console.log(
    `${binding} (${backend}): ran ${result.total - result.failed} of ${result.total} cases`,
  );
  return result.failed === 0;
}

/** 見つかった Chrome の名前。無ければ null。 */
function findChrome() {
  for (const name of ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser"]) {
    if (works(name, ["--version"])) return name;
  }
  return null;
}

/** コマンドが実行でき、終了コード 0 で終わるか。 */
function works(cmd, cmdArgs) {
  const r = spawnSync(cmd, cmdArgs, { stdio: "ignore" });
  return !r.error && r.status === 0;
}

/** コマンドの標準出力 (前後の空白を除く)。実行できなければ null。 */
function capture(cmd, cmdArgs) {
  const r = spawnSync(cmd, cmdArgs, { cwd: root, encoding: "utf8" });
  return !r.error && r.status === 0 ? r.stdout.trim() : null;
}
