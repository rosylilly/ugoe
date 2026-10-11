#!/usr/bin/env node
// デスクトップアプリ (エディタ・ビューア・スタジオ) のインストーラを作る、ビルドの入口 (`just app::build`。scripts/app.just)。
// 手順と前提は apps/editor/README.md#各-os-でのビルド。
//
//   node scripts/app-build.mjs [TARGET=editor,viewer,studio] [PLATFORM=windows,macos,linux] [VCAM=1] [DRY_RUN=1]
//                              [tauri build の追加の引数...]
//
// - TARGET: 作るアプリ (カンマ区切り。`all` か省略ですべて)。
// - PLATFORM: 向ける OS (カンマ区切り。省略で今の OS)。今の OS 向けは tauri build、Linux (WSL2 を含む) から
//   Windows 向けは cargo-xwin でクロスビルドする (NSIS だけ。署名なし)。作れない組み合わせは、何も作らずに理由と代わりを示して止まる。
// - VCAM=1: macOS 向けのスタジオに仮想カメラの Camera Extension を入れる (署名の環境変数が要る。docs/release.md#macos-の仮想カメラ)。
// - DRY_RUN=1: 道具を調べ、実行するコマンドを表示するだけ。
// - 署名と公証は環境変数があるときだけ行う (scripts/lib/app-signing.mjs、docs/release.md#デスクトップアプリの署名)。
//   無ければ署名せずに作る。署名したかを最後に出し、GitHub Actions ではジョブの要約 (`$GITHUB_STEP_SUMMARY`) にも書く。
// - それ以外の引数は、すべての tauri build にそのまま渡す (例: `--debug`、`--bundles deb`、`--target aarch64-apple-darwin`)。
//
// できたインストーラは target/dist/<アプリ>/<OS>/ に集め、最後に一覧を出す。
// アプリごとの違い (同梱するもの・重ねる設定) は下の APPS だけに書く。

import { spawnSync } from "node:child_process";
import {
  accessSync,
  appendFileSync,
  constants,
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { delimiter, join } from "node:path";
import {
  notarytoolAuthArgs,
  signingPlan,
  signingSummary,
  withoutEmptySigningEnv,
} from "./lib/app-signing.mjs";
import { exportMbxPath } from "./lib/mbx.mjs";

const root = join(import.meta.dirname, "..");

/**
 * アプリごとの違い。`configs` は `tauri build --config` で重ねる設定 (アプリのディレクトリから見たパス)。
 * `all` はすべての OS、`windows`・`macos`・`linux` はその OS 向けのときだけ。
 */
export const APPS = {
  editor: {
    package: "@ugoe/editor",
    dir: "apps/editor",
    // MCP サーバの ugoe-mcp をサイドカーとして同梱する。beforeBuildCommand の `pnpm sidecar` がビルドのターゲット向けに作る
    // (Linux から Windows 向けは cargo-xwin。docs/editor.md#エージェントをつなぐ)
    configs: { all: ["src-tauri/tauri.bundle.conf.json"] },
  },
  viewer: {
    package: "@ugoe/viewer",
    dir: "apps/viewer",
    configs: {},
  },
  studio: {
    package: "@ugoe/studio",
    dir: "apps/studio",
    // Windows 向けでは、Tauri がターゲットの OS を見て src-tauri/tauri.windows.conf.json を自動で重ねる
    // (仮想カメラの DLL を `pnpm vcam` で作り、NSIS のフック windows/hooks.nsh が入れて登録する。T0807)。
    // クロスビルドでも同じ (`pnpm vcam` は Windows 以外では cargo-xwin を使う)。
    configs: {},
    // VCAM=1 のときだけ重ねる設定と、その前提 (macOS の Camera Extension。`pnpm vcam:macos`、T0809)
    vcam: {
      macos: {
        config: "src-tauri/tauri.macos-vcam.conf.json",
        env: [
          "UGOE_APPLE_TEAM_ID",
          "APPLE_SIGNING_IDENTITY",
          "UGOE_VCAM_PROVISIONING_PROFILE",
          "UGOE_STUDIO_PROVISIONING_PROFILE",
        ],
      },
    },
  },
};

export const PLATFORMS = ["windows", "macos", "linux"];

/** Linux から Windows 向けのクロスビルドのターゲット。 */
export const WINDOWS_CROSS_TARGET = "x86_64-pc-windows-msvc";
/** macOS 向けの既定のターゲット (Apple Silicon と Intel の両方で動く 1 つの .dmg。CI も同じ)。 */
export const MACOS_TARGET = "universal-apple-darwin";

/** 署名の設定を重ねるファイル (`signingPlan` の `config` を書く。リポジトリのルートから)。 */
export const SIGNING_CONFIG = join("target", "app-signing", "windows.conf.json");

/** 集めるインストーラ (tauri のバンドルの種類のディレクトリと拡張子)。 */
const BUNDLE_KINDS = [
  ["deb", ".deb"],
  ["rpm", ".rpm"],
  ["appimage", ".AppImage"],
  ["msi", ".msi"],
  ["nsis", ".exe"],
  ["dmg", ".dmg"],
];

const KEYS = ["TARGET", "PLATFORM", "VCAM", "DRY_RUN"];

/** `process.platform` を PLATFORMS の名前にする (対応しない OS は null)。 */
export function hostPlatform(platform = process.platform) {
  return { linux: "linux", win32: "windows", darwin: "macos" }[platform] ?? null;
}

/** カンマ区切りの値を、重複を除いた名前の列にする。 */
function splitList(value) {
  return [
    ...new Set(
      value
        .split(",")
        .map((s) => s.trim().toLowerCase())
        .filter(Boolean),
    ),
  ];
}

/**
 * 引数を読む。`KEY=値` (大文字小文字は問わない) をこのスクリプトの指定、それ以外を tauri build の引数とする。
 * PowerShell は引数の `a,b` を 2 つの引数に分けるので、`TARGET=editor` の直後に続くアプリや OS の名前も同じ指定の続きとして読む。
 * 引数に無い指定は環境変数 (`DRY_RUN=1 just app::build` など) から読む。失敗したら `{ error }` を返す。
 */
export function parseArgs(argv, env = process.env) {
  const values = {};
  const extra = [];
  let last = null;
  for (const arg of argv) {
    const m = arg.match(/^([A-Za-z][A-Za-z_]*)=(.*)$/s);
    if (m && KEYS.includes(m[1].toUpperCase())) {
      last = m[1].toUpperCase();
      values[last] = [...(values[last] ?? []), ...splitList(m[2])];
      continue;
    }
    if (m && m[1] === m[1].toUpperCase()) {
      return { error: `unknown option ${m[1]}= (expected one of ${KEYS.join(", ")})` };
    }
    const names = splitList(arg);
    const continues =
      (last === "TARGET" && names.every((n) => n === "all" || n in APPS)) ||
      (last === "PLATFORM" && names.every((n) => PLATFORMS.includes(n)));
    if (continues && names.length > 0 && !arg.startsWith("-")) {
      values[last].push(...names);
      continue;
    }
    last = null;
    extra.push(arg);
  }

  // 引数に無い指定は環境変数から読む (`DRY_RUN=1 just app::build` のように前に置いても効くように)
  for (const key of KEYS) {
    if (values[key] === undefined && env[key] !== undefined) values[key] = splitList(env[key]);
  }

  const targets = values.TARGET ?? [];
  const apps = targets.length === 0 || targets.includes("all") ? Object.keys(APPS) : targets;
  const unknownApp = apps.find((a) => !(a in APPS));
  if (unknownApp) {
    return {
      error: `unknown TARGET ${unknownApp} (expected ${Object.keys(APPS).join(", ")} or all)`,
    };
  }
  const platforms = values.PLATFORM ?? [];
  const unknownPlatform = platforms.find((p) => !PLATFORMS.includes(p));
  if (unknownPlatform) {
    return { error: `unknown PLATFORM ${unknownPlatform} (expected ${PLATFORMS.join(", ")})` };
  }
  const flag = (key) => (values[key] ?? []).some((v) => !["", "0", "false", "no"].includes(v));
  return {
    apps: [...new Set(apps)],
    platforms: [...new Set(platforms)],
    vcam: flag("VCAM"),
    dryRun: flag("DRY_RUN"),
    extra,
  };
}

/** 引数の列から `--name 値` または `--name=値` の値を探す。 */
function optionValue(args, ...names) {
  for (let i = 0; i < args.length; i++) {
    for (const name of names) {
      if (args[i] === name) return args[i + 1] ?? null;
      if (args[i].startsWith(`${name}=`)) return args[i].slice(name.length + 1);
    }
  }
  return null;
}

/**
 * 作れない組み合わせの理由と代わりの方法。作れるなら null。
 * `host` は今の OS、`platform` は向ける OS。
 */
export function unsupportedReason(host, platform) {
  if (host === platform) return null;
  if (host === "linux" && platform === "windows") return null;
  const ci =
    "or run the GitHub Actions workflow `Editor build` (.github/workflows/editor-build.yml, workflow_dispatch) " +
    "and download the artifacts";
  switch (platform) {
    case "macos":
      return (
        "macOS apps can only be built on macOS (the Apple SDK, lipo, codesign and the .dmg tools exist only there). " +
        `Run \`just app::build PLATFORM=macos\` on a Mac, ${ci}`
      );
    case "linux":
      return (
        "Linux packages (.deb, .rpm, .AppImage) can only be built on Linux. " +
        (host === "windows"
          ? "Run the same command inside WSL2, "
          : "Run it on a Linux machine, ") +
        ci
      );
    case "windows":
      return (
        "the Windows cross build is only set up from Linux (cargo-xwin + NSIS). " +
        `Run \`just app::build PLATFORM=windows\` on Windows or inside a Linux machine/VM, ${ci}`
      );
    default:
      return `unsupported platform ${platform}`;
  }
}

/**
 * 1 つのアプリと OS の組み合わせのビルドの計画を作る (純粋な関数。テストする)。
 * `extra` は利用者が渡した tauri build の引数。`--target` や `--bundles` があれば、既定のものを足さない。
 */
export function planBuild({ app, platform, host, vcam = false, extra = [], signing = null }) {
  const def = APPS[app];
  const cross = host !== platform;
  const userTarget = optionValue(extra, "--target", "-t");
  const userBundles = optionValue(extra, "--bundles", "-b");
  const args = ["--filter", def.package, "tauri", "build"];
  for (const config of [...(def.configs.all ?? []), ...(def.configs[platform] ?? [])]) {
    args.push("--config", config);
  }
  const vcamDef = def.vcam?.[platform];
  if (vcam && vcamDef) args.push("--config", vcamDef.config);
  // 署名の設定はアプリのディレクトリから見たパスでなく、ルートからの絶対パスで渡す
  if (signing?.config) args.push("--config", join(root, SIGNING_CONFIG));

  let target = userTarget;
  if (cross && platform === "windows") {
    // cargo-xwin が MSVC の CRT と Windows SDK を用意する。MSI (WiX) は Windows でしか作れないので NSIS だけを作る
    args.push("--runner", "cargo-xwin");
    if (!userTarget) {
      target = WINDOWS_CROSS_TARGET;
      args.push("--target", target);
    }
    if (!userBundles) args.push("--bundles", "nsis");
  } else if (platform === "macos" && !userTarget) {
    target = MACOS_TARGET;
    args.push("--target", target);
  }
  args.push(...extra);

  const profile = extra.includes("--debug") || extra.includes("-d") ? "debug" : "release";
  const bundleDir = join("target", ...(target ? [target] : []), profile, "bundle");
  return {
    app,
    platform,
    cross,
    target,
    args,
    bundleDir,
    dist: join("target", "dist", app, platform),
    signing,
  };
}

/**
 * すべての組み合わせの計画。作れない組み合わせや半端な署名の設定があれば `{ errors }` を返す (1 つも作らない)。
 * `env` は署名の環境変数を読む先 (scripts/lib/app-signing.mjs)。
 */
export function planAll({ apps, platforms, host, vcam, extra, env = {} }) {
  const errors = [];
  const signing = {};
  for (const platform of platforms) {
    const reason = unsupportedReason(host, platform);
    if (reason) errors.push(`${platform} from ${host}: ${reason}`);
    signing[platform] = signingPlan(platform, env);
    if (signing[platform].error) errors.push(`signing for ${platform}: ${signing[platform].error}`);
  }
  if (errors.length > 0) return { errors };
  const builds = [];
  for (const platform of platforms) {
    for (const app of apps) {
      builds.push(planBuild({ app, platform, host, vcam, extra, signing: signing[platform] }));
    }
  }
  return { builds };
}

// --- ここから下は環境を調べてビルドする部分 ---

/** `rust-toolchain.toml` で固定された toolchain の版。 */
function pinnedToolchain() {
  const toml = readFileSync(join(root, "rust-toolchain.toml"), "utf8");
  const m = toml.match(/^channel\s*=\s*"([^"]+)"/m);
  return m ? m[1] : null;
}

/** `PATH` からコマンドを探す。 */
function onPath(name) {
  const exts = process.platform === "win32" ? ["", ".exe", ".cmd"] : [""];
  return (process.env.PATH ?? "").split(delimiter).some((dir) =>
    exts.some((ext) => {
      try {
        accessSync(join(dir, name + ext), constants.X_OK);
        return true;
      } catch {
        return false;
      }
    }),
  );
}

/** コマンドの出力 (失敗したら null)。 */
function output(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: root, encoding: "utf8" });
  return r.status === 0 ? r.stdout : null;
}

/** rustup のターゲットが足りなければ、その入れ方を `missing` に足す。 */
function checkRustTargets(triples, missing) {
  const toolchain = pinnedToolchain();
  const listArgs = [
    "target",
    "list",
    "--installed",
    ...(toolchain ? ["--toolchain", toolchain] : []),
  ];
  const installed = output("rustup", listArgs);
  if (installed === null) {
    missing.push(["rustup", "https://rustup.rs/"]);
    return;
  }
  for (const triple of triples) {
    if (!installed.split(/\s+/).includes(triple)) {
      const suffix = toolchain ? ` --toolchain ${toolchain}` : "";
      missing.push([`Rust target ${triple}`, `rustup target add ${triple}${suffix}`]);
    }
  }
}

/** 足りない道具と、その入れ方の一覧 (重複は除く)。 */
function missingTools(builds, vcam) {
  const missing = [];
  const platforms = new Set(builds.map((b) => b.platform));
  if (builds.some((b) => b.cross && b.platform === "windows")) {
    checkRustTargets([builds.find((b) => b.cross).target ?? WINDOWS_CROSS_TARGET], missing);
    if (output("cargo", ["xwin", "--version"]) === null) {
      missing.push(["cargo-xwin", "cargo install cargo-xwin --locked"]);
    }
    // Debian / Ubuntu のパッケージ名。clang は C のコード、lld-link はリンク、
    // llvm-rc と llvm-lib はアイコンなどのリソースと静的ライブラリ、makensis はインストーラに使う
    const commands = [
      ["clang", "clang"],
      ["lld-link", "lld"],
      ["llvm-rc", "llvm"],
      ["llvm-lib", "llvm"],
      ["makensis", "nsis"],
    ];
    for (const [command, pkg] of commands) {
      if (!onPath(command)) missing.push([command, `sudo apt install ${pkg}`]);
    }
  }
  if (builds.some((b) => b.platform === "macos" && b.target === MACOS_TARGET)) {
    checkRustTargets(["aarch64-apple-darwin", "x86_64-apple-darwin"], missing);
  }
  if (platforms.has("linux") && onPath("pkg-config")) {
    if (output("pkg-config", ["--exists", "webkit2gtk-4.1"]) === null) {
      missing.push([
        "WebKitGTK 4.1 and the Tauri build dependencies",
        "sudo apt install build-essential libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf",
      ]);
    }
  }
  if (vcam) {
    for (const build of builds) {
      const vcamDef = APPS[build.app].vcam?.[build.platform];
      for (const name of vcamDef?.env ?? []) {
        if (!process.env[name])
          missing.push([`${name} (VCAM=1)`, "see docs/release.md#macos-の仮想カメラ"]);
      }
    }
  }
  const seen = new Set();
  return missing.filter(([tool]) => !seen.has(tool) && seen.add(tool));
}

/** WSL の中か。 */
function isWsl() {
  if (process.platform !== "linux") return false;
  if (process.env.WSL_DISTRO_NAME) return true;
  try {
    return /microsoft/i.test(readFileSync("/proc/version", "utf8"));
  } catch {
    return false;
  }
}

/** ビルドに渡す環境変数。 */
function buildEnv(build) {
  const env = withoutEmptySigningEnv(process.env);
  // WSL では AppImage を作る linuxdeploy が PATH の Windows のディレクトリ (/mnt/c/...) を読めずに失敗するので外す
  if (build.platform === "linux" && isWsl()) {
    env.PATH = (env.PATH ?? "")
      .split(delimiter)
      .filter((dir) => !dir.startsWith("/mnt/"))
      .join(delimiter);
  }
  return env;
}

/** 表示と、Windows のシェルに渡すための引用。 */
function quote(arg) {
  return /^[\w@%+=:,./-]+$/.test(arg) ? arg : `"${arg.replaceAll('"', '\\"')}"`;
}

/** tauri.conf.json の productName と version。 */
function productOf(app) {
  const conf = JSON.parse(
    readFileSync(join(root, APPS[app].dir, "src-tauri", "tauri.conf.json"), "utf8"),
  );
  return { name: conf.productName, version: conf.version };
}

/**
 * このビルドでできたインストーラを target/dist/<アプリ>/<OS>/ に写す。
 * バンドルのディレクトリはアプリの間で共有なので、製品名と版で始まり、ビルドを始めた後にできたファイルだけを選ぶ。
 */
function collect(build, startedAt) {
  const { name, version } = productOf(build.app);
  const dist = join(root, build.dist);
  rmSync(dist, { recursive: true, force: true });
  mkdirSync(dist, { recursive: true });
  const files = [];
  for (const [kind, ext] of BUNDLE_KINDS) {
    const dir = join(root, build.bundleDir, kind);
    if (!existsSync(dir)) continue;
    for (const file of readdirSync(dir)) {
      // rpm だけは `<名前>-<版>-...`、ほかは `<名前>_<版>_...`
      const ours = file.startsWith(`${name}_${version}`) || file.startsWith(`${name}-${version}`);
      const path = join(dir, file);
      if (!ours || !file.endsWith(ext) || statSync(path).mtimeMs < startedAt) continue;
      copyFileSync(path, join(dist, file));
      files.push(join(build.dist, file));
    }
  }
  return files;
}

/**
 * 署名した .dmg を公証してステープルする (Tauri が公証するのは .app だけ)。
 * `xcrun notarytool` と `xcrun stapler` は macOS にしか無い。
 */
function notarizeDmgs(files) {
  for (const file of files.filter((f) => f.endsWith(".dmg"))) {
    const path = join(root, file);
    console.error(`\n==> notarizing ${file}`);
    const steps = [
      ["notarytool", "submit", path, ...notarytoolAuthArgs(), "--wait"],
      ["stapler", "staple", path],
    ];
    for (const args of steps) {
      const r = spawnSync("xcrun", args, { cwd: root, stdio: "inherit" });
      if (r.error || r.status !== 0) {
        console.error(`error: xcrun ${args[0]} failed for ${file}`);
        return false;
      }
    }
  }
  return true;
}

/** 署名したかを表示し、GitHub Actions ではジョブの要約にも書く。 */
function reportSigning(builds, dryRun = false) {
  const rows = builds.map((b) => ({ app: b.app, platform: b.platform, plan: b.signing }));
  for (const { app, platform, plan } of rows) {
    const state = plan.sign ? (plan.notarize ? "signed and notarized" : "signed") : "not signed";
    console.error(
      `  ${app}/${platform}: ${dryRun ? `would be ${state}` : state}${plan.reason ? ` (${plan.reason})` : ""}`,
    );
  }
  if (process.env.GITHUB_STEP_SUMMARY && !dryRun) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, signingSummary(rows));
  }
}

function formatSize(bytes) {
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

function main(argv) {
  // mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
  exportMbxPath();
  const host = hostPlatform();
  if (!host) {
    console.error(`error: unsupported host OS ${process.platform}`);
    return 1;
  }
  const parsed = parseArgs(argv);
  if (parsed.error) {
    console.error(`error: ${parsed.error}`);
    return 1;
  }
  if (parsed.platforms.length === 0) parsed.platforms = [host];

  const plan = planAll({ ...parsed, host, env: process.env });
  if (plan.errors) {
    console.error(
      "error: some of the requested builds cannot be made on this machine (nothing was built):",
    );
    for (const e of plan.errors) console.error(`  - ${e}`);
    return 1;
  }
  if (parsed.vcam && !plan.builds.some((b) => APPS[b.app].vcam?.[b.platform])) {
    console.error("error: VCAM=1 only applies to TARGET=studio PLATFORM=macos");
    return 1;
  }
  for (const b of plan.builds) {
    if (b.app === "studio" && b.platform === "macos" && !parsed.vcam) {
      console.error(
        "note: the macOS studio is built without the virtual camera (Camera Extension). " +
          "Add VCAM=1 to include it (needs signing; docs/release.md#macos-の仮想カメラ).",
      );
    }
  }

  const missing = missingTools(plan.builds, parsed.vcam);
  if (missing.length > 0) {
    console.error(
      "error: the build needs the following tools (see apps/editor/README.md#各-os-でのビルド):",
    );
    for (const [tool, install] of missing) console.error(`  ${tool}: ${install}`);
    return 1;
  }

  const signingConfig = plan.builds.find((b) => b.signing.config)?.signing.config;
  if (signingConfig && !parsed.dryRun) {
    mkdirSync(join(root, "target", "app-signing"), { recursive: true });
    writeFileSync(join(root, SIGNING_CONFIG), `${JSON.stringify(signingConfig, null, 2)}\n`);
  }

  const results = [];
  for (const build of plan.builds) {
    const label = `${build.app} for ${build.platform}${build.cross ? " (cross build)" : ""}`;
    console.error(`\n==> ${label}: pnpm ${build.args.map(quote).join(" ")}`);
    if (parsed.dryRun) continue;
    const startedAt = Date.now() - 1000;
    // Windows の pnpm は pnpm.cmd なので、シェルを通して実行する
    const win = process.platform === "win32";
    const r = spawnSync("pnpm", win ? build.args.map(quote) : build.args, {
      cwd: root,
      stdio: "inherit",
      env: buildEnv(build),
      shell: win,
    });
    if (r.error) {
      console.error(`error: failed to run pnpm: ${r.error.message}`);
      return 1;
    }
    if (r.status !== 0) {
      console.error(`error: the build of ${label} failed (exit ${r.status})`);
      return r.status ?? 1;
    }
    const files = collect(build, startedAt);
    if (build.signing.notarize && build.platform === "macos" && !notarizeDmgs(files)) return 1;
    results.push([build, files]);
  }

  console.error("\nCode signing (docs/release.md#デスクトップアプリの署名):");
  if (parsed.dryRun) {
    reportSigning(plan.builds, true);
    return 0;
  }
  reportSigning(results.map(([build]) => build));
  console.error("\nInstallers (copied from the tauri bundle directories):");
  for (const [build, files] of results) {
    if (files.length === 0) {
      console.error(
        `  ${build.app}/${build.platform}: no installer was found in ${build.bundleDir}`,
      );
    }
    for (const file of files) {
      console.error(`  ${file}  (${formatSize(statSync(join(root, file)).size)})`);
    }
  }
  return 0;
}

if (import.meta.main) process.exitCode = main(process.argv.slice(2));
