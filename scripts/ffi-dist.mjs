#!/usr/bin/env node
// C ABI の配布物 (ugoe-ffi の共有ライブラリ・静的ライブラリとヘッダ) を作る。`just ffi-dist` と `just ffi-dist-check` の実体
// (docs/release.md#c-abi-の配布物)。
//
//   node scripts/ffi-dist.mjs pack [--out <dir>] [--tag <tag>]
//       手元の OS の分のアーカイブ (ugoe-ffi-<版>-<プラットフォーム>.tar.gz か .zip) と .sha256 を <dir> (既定 target/ffi-dist) に作る。
//       ヘッダが cbindgen の生成物と同じか (`just ffi-check` と同じ検査) を確かめてから詰める。
//       --tag を渡すと、タグがルートの Cargo.toml の版 (v<版>) と揃っているかも確かめる。
//   node scripts/ffi-dist.mjs verify [<archive>]
//       アーカイブ (既定は target/ffi-dist の手元の OS のもの) を一時ディレクトリに展開し、中のファイルの一覧を確かめ、中のヘッダとライブラリと C の見本だけで
//       見本をビルドして動かす (静的ライブラリと共有ライブラリの両方)。Linux と macOS では、共有ライブラリの soname・インストール名が
//       中のヘッダの ABI の版と合っているかと、リンクした見本がその名前を記録したかも確かめる。
//   node scripts/ffi-dist.mjs build-plugin --platform <name> --out <dir>
//       Unity のプラグインのライブラリ (`BUILDS` の `plugin`。Android・iOS を含む) だけを作り、<dir> に置く (unity.yml の native)。
//       soname・版の付いた名前は付けない (Unity はファイル名で読み、Android の動的リンカは soname で読み込み済みのライブラリを見分けるため)。
//
// ビルドの手順 (ターゲットごとの cargo・macOS の lipo とインストール名・できたファイルの名前) は `pack` と `build-plugin` で共有する
// (docs/release.md#ワークフロー)。ワークフロー (ffi-release.yml と unity.yml) は分けたまま。
//
// Linux と macOS の共有ライブラリは、名前に ABI の版 (`ugoe_ffi::ABI_VERSION`。ヘッダの `UGOE_ABI_VERSION`) を入れた実体
// (`libugoe_ffi.so.<ABI の版>`・`libugoe_ffi.<ABI の版>.dylib`) と、リンク用のシンボリックリンク (`libugoe_ffi.so`・`libugoe_ffi.dylib`)
// で配る。soname・インストール名も版の付いた名前にするので、リンクしたプログラムは版の付いた名前を探し、ABI の版が違うライブラリを
// 読まない (docs/release.md#c-abi-の配布物)。版は lib.rs から読み、手で書いた数を持たない。
//
// どのプラットフォームも `--target` を付けて作る (`BUILDS` の `targets`)。rustup でターゲットを入れておく。macOS では arm64 と x86_64 を
// 作って lipo でまとめる (universal):
//   rustup target add aarch64-apple-darwin x86_64-apple-darwin

import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative } from "node:path";
import { workspaceVersion } from "./crates.mjs";
import { checkHeader, compileSample, run, runSample, STATIC_SYSTEM_LIBS } from "./ffi.mjs";
import { readChecksum, sha256File, writeChecksum } from "./lib/checksum.mjs";
import { exportMbxPath } from "./lib/mbx.mjs";

const root = join(import.meta.dirname, "..");
const crate = join(root, "crates", "ugoe-ffi");

/**
 * 配布するプラットフォーム。`lib` はアーカイブの `lib/` に入れるファイル (共有ライブラリ・静的ライブラリ。Windows は
 * DLL のインポートライブラリ `.dll.lib` も)。`format` はアーカイブの形。
 * `versioned` は、`lib` の最初のもの (共有ライブラリ) を ABI の版の付いた名前の実体へのシンボリックリンクにするときの、その実体の名前
 * (soname・インストール名もこれにする)。Windows の DLL には該当するものが無いので持たない。
 */
export const PLATFORMS = {
  "linux-x86_64": {
    lib: ["libugoe_ffi.so", "libugoe_ffi.a"],
    versioned: (abi) => `libugoe_ffi.so.${abi}`,
    format: "tar.gz",
  },
  "linux-aarch64": {
    lib: ["libugoe_ffi.so", "libugoe_ffi.a"],
    versioned: (abi) => `libugoe_ffi.so.${abi}`,
    format: "tar.gz",
  },
  "windows-x86_64": { lib: ["ugoe_ffi.dll", "ugoe_ffi.dll.lib", "ugoe_ffi.lib"], format: "zip" },
  "macos-universal": {
    lib: ["libugoe_ffi.dylib", "libugoe_ffi.a"],
    versioned: (abi) => `libugoe_ffi.${abi}.dylib`,
    format: "tar.gz",
  },
};

/** lib.rs の `pub const ABI_VERSION: u32 = <版>;` から ABI の版。見つからなければ null。 */
export function abiVersionFromRust(source) {
  const m = /^pub const ABI_VERSION: u32 = (\d+);$/m.exec(source);
  return m ? Number(m[1]) : null;
}

/** ヘッダの `#define UGOE_ABI_VERSION <版>` から ABI の版。見つからなければ null。 */
export function abiVersionFromHeader(source) {
  const m = /^#define UGOE_ABI_VERSION (\d+)$/m.exec(source);
  return m ? Number(m[1]) : null;
}

/** 共有ライブラリの soname (Linux)・インストール名 (macOS)。持たないプラットフォーム (Windows) なら null。 */
export function sharedLibraryId(platform, abi) {
  const { versioned } = PLATFORMS[platform];
  if (!versioned) return null;
  return platform === "macos-universal" ? `@rpath/${versioned(abi)}` : versioned(abi);
}

/**
 * ugoe-ffi をビルドするプラットフォーム (C ABI の配布物の `PLATFORMS` と、Unity のプラグインの並び (unity.yml の native) を合わせたもの)。
 * - `targets`: cargo のターゲット。2 つなら lipo で 1 つの universal にまとめる (macOS)。
 * - `android`: cargo-ndk の ABI (Android の NDK でビルドする)。
 * - `plugin`: Unity のプラグインに置くライブラリ (`build-plugin`。bindings/unity/build.mjs の PLUGINS の from のファイル名)。
 *   C ABI のアーカイブにしか無いもの (linux-aarch64) は持たない。
 */
export const BUILDS = {
  "linux-x86_64": { targets: ["x86_64-unknown-linux-gnu"], plugin: "libugoe_ffi.so" },
  "linux-aarch64": { targets: ["aarch64-unknown-linux-gnu"] },
  "windows-x86_64": { targets: ["x86_64-pc-windows-msvc"], plugin: "ugoe_ffi.dll" },
  "macos-universal": {
    targets: ["aarch64-apple-darwin", "x86_64-apple-darwin"],
    plugin: "libugoe_ffi.dylib",
  },
  "android-arm64-v8a": {
    targets: ["aarch64-linux-android"],
    android: "arm64-v8a",
    plugin: "libugoe_ffi.so",
  },
  "android-armeabi-v7a": {
    targets: ["armv7-linux-androideabi"],
    android: "armeabi-v7a",
    plugin: "libugoe_ffi.so",
  },
  "android-x86_64": {
    targets: ["x86_64-linux-android"],
    android: "x86_64",
    plugin: "libugoe_ffi.so",
  },
  // iOS はアプリに静的にリンクする (Unity の iOS のプラグインは静的ライブラリ)
  "ios-arm64": { targets: ["aarch64-apple-ios"], plugin: "libugoe_ffi.a" },
};

/** Android の最低の API レベル (cargo-ndk の `--platform`)。 */
const ANDROID_API = 21;

/**
 * `platform` の ugoe-ffi をリリースビルドするコマンド (`[コマンド, 引数]` の並び。ターゲットごとに 1 つ)。
 * - `soname` を渡すと、共有ライブラリの soname をそれにする (Linux の C ABI の配布物だけ。Unity のプラグインには付けない。
 *   docs/release.md#c-abi-の配布物)。リンクの引数は共有ライブラリにだけ効く (静的ライブラリは変わらない)。
 * - `hostOs` は手元の `process.platform`。Windows の外で Windows (MSVC) 向けに作るときは cargo-xwin を使う (手元での確かめ用)。
 */
export function cargoCommands(platform, { soname = null, hostOs = process.platform } = {}) {
  const { targets, android } = BUILDS[platform];
  const common = ["--locked", "--release", "-p", "ugoe-ffi"];
  return targets.map((target) => {
    if (android) {
      return [
        "cargo",
        ["ndk", "--target", android, "--platform", String(ANDROID_API), "build", ...common],
      ];
    }
    if (soname) {
      return [
        "cargo",
        [
          "rustc",
          ...common,
          "--lib",
          "--target",
          target,
          "--",
          "-C",
          `link-arg=-Wl,-soname,${soname}`,
        ],
      ];
    }
    const xwin = target.endsWith("-windows-msvc") && hostOs !== "win32";
    return ["cargo", [...(xwin ? ["xwin", "build"] : ["build"]), ...common, "--target", target]];
  });
}

/** cargo の成果物の置き場 (CARGO_TARGET_DIR か、リポジトリの target/)。 */
function targetDir() {
  return process.env.CARGO_TARGET_DIR ?? join(root, "target");
}

/** `platform` のビルドでできたファイル `file` (ターゲットごと)。 */
export function builtFiles(platform, file, dir = targetDir()) {
  return BUILDS[platform].targets.map((t) => join(dir, t, "release", file));
}

/** Unity のプラグインの macOS のインストール名。Unity はファイル名で読むので、版の無いファイル名だけにする。 */
export const PLUGIN_INSTALL_NAME = "@rpath/libugoe_ffi.dylib";

/** `platform` の ugoe-ffi をリリースビルドする (`cargoCommands`)。 */
function compile(platform, options) {
  for (const [cmd, args] of cargoCommands(platform, options)) run(cmd, args);
}

/**
 * ビルドしたライブラリ `file` を `dest` に置く。ターゲットが 2 つ (macOS) なら lipo で universal にまとめる。
 * `installName` を渡すと、共有ライブラリ (macOS) のインストール名をそれにする。macOS の universal とインストール名を扱うのはここだけ。
 */
function place(platform, file, dest, installName = null) {
  const [first, ...more] = builtFiles(platform, file);
  if (more.length === 0) copyFileSync(first, dest);
  else run("lipo", ["-create", "-output", dest, first, ...more]);
  if (installName) run("install_name_tool", ["-id", installName, dest]);
}

/**
 * Unity のプラグインのライブラリ (`BUILDS` の `plugin`) を作り、`out` に置く (unity.yml の native。soname・版の付いた名前は付けない)。
 * Android は cargo-ndk と NDK (ANDROID_NDK_HOME)、ほかの OS 向けは rustup のターゲットを先に用意しておく。
 */
export function buildPlugin(platform, out) {
  const { plugin } = BUILDS[platform];
  compile(platform);
  mkdirSync(out, { recursive: true });
  const dest = join(out, plugin);
  place(platform, plugin, dest, plugin.endsWith(".dylib") ? PLUGIN_INSTALL_NAME : null);
  return dest;
}

/** Node の `process.platform`・`process.arch` から配布のプラットフォームの名前。配らない組み合わせなら null。 */
export function platformOf(platform, arch) {
  if (platform === "linux" && arch === "x64") return "linux-x86_64";
  if (platform === "linux" && arch === "arm64") return "linux-aarch64";
  if (platform === "win32" && arch === "x64") return "windows-x86_64";
  if (platform === "darwin") return "macos-universal";
  return null;
}

/** アーカイブの中の最上位のディレクトリの名前 (拡張子を除いたアーカイブの名前)。 */
export function stem(version, platform) {
  return `ugoe-ffi-${version}-${platform}`;
}

/** アーカイブのファイル名。 */
export function archiveName(version, platform) {
  return `${stem(version, platform)}.${PLATFORMS[platform].format}`;
}

/** アーカイブのファイル名から版とプラットフォーム。形が違えば null。 */
export function parseArchiveName(name) {
  const m = /^ugoe-ffi-(.+)-((?:linux|windows|macos)-[a-z0-9_]+)\.(tar\.gz|zip)$/.exec(name);
  if (!m || !(m[2] in PLATFORMS) || PLATFORMS[m[2]].format !== m[3]) return null;
  return { version: m[1], platform: m[2] };
}

/**
 * アーカイブの中のファイル (最上位のディレクトリからの相対パス、`/` 区切り、並べ替え済み)。シンボリックリンクも含む。
 * `abi` は ABI の版 (版の付いた共有ライブラリの名前に使う)。
 */
export function expectedFiles(platform, abi) {
  const { lib, versioned } = PLATFORMS[platform];
  return [
    "LICENSE",
    "README.md",
    "THIRD-PARTY-NOTICES.txt",
    "examples/sample.c",
    "include/ugoe.h",
    ...lib.map((f) => `lib/${f}`),
    ...(versioned ? [`lib/${versioned(abi)}`] : []),
  ].sort();
}

/** lib.rs の `ABI_VERSION` とヘッダの `UGOE_ABI_VERSION` を読み、一致すればその版を返す。食い違えば終える。 */
function abiVersion() {
  const rust = abiVersionFromRust(readFileSync(join(crate, "src", "lib.rs"), "utf8"));
  const header = abiVersionFromHeader(readFileSync(join(crate, "include", "ugoe.h"), "utf8"));
  if (rust === null) fail("ABI_VERSION not found in crates/ugoe-ffi/src/lib.rs");
  if (rust !== header) {
    fail(`ABI_VERSION in lib.rs (${rust}) and UGOE_ABI_VERSION in ugoe.h (${header}) differ`);
  }
  return rust;
}

/** タグ (`v<版>`) が版と揃っていなければ、その問題の文。揃っていれば null。 */
export function tagProblem(tag, version) {
  return tag === `v${version}`
    ? null
    : `tag ${tag} does not match the workspace version ${version} (expected v${version})`;
}

/**
 * tar の実行ファイル。Windows では Windows に入っている bsdtar (zip を作れて展開できる) を使う (Git for Windows の GNU tar が
 * PATH の先にあることがあり、それは zip を扱えず、`C:` を含むパスをリモートのホストとみなす)。
 */
function tar() {
  return process.platform === "win32"
    ? join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe")
    : "tar";
}

/** ディレクトリの中のファイルを、`dir` からの相対パス (`/` 区切り) で並べ替えて返す。 */
function listFiles(dir) {
  const out = [];
  const walk = (d) => {
    for (const name of readdirSync(d)) {
      const path = join(d, name);
      // シンボリックリンクはたどらず、1 つのファイルとして数える
      if (lstatSync(path).isDirectory()) walk(path);
      else out.push(relative(dir, path).split("\\").join("/"));
    }
  };
  walk(dir);
  return out.sort();
}

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

/**
 * `platform` の ugoe-ffi をリリースビルドし、`lib` に配るライブラリを置く。`abi` は ABI の版。共有ライブラリは版の付いた名前の実体と、
 * そこへのシンボリックリンクにする (`PLATFORMS` の `versioned`)。soname・インストール名もその名前にする。
 * Linux の soname は配布物のビルドだけで付け、ふだんのビルド (unity.yml のプラグイン・テスト) には付けない (docs/release.md#c-abi-の配布物)。
 */
function build(platform, lib, abi) {
  mkdirSync(lib, { recursive: true });
  const { versioned, lib: files } = PLATFORMS[platform];
  const id = sharedLibraryId(platform, abi);
  compile(platform, { soname: platform === "macos-universal" ? null : id });
  const [shared, ...rest] = files;
  if (versioned) {
    // macOS は読み込むときに実行ファイルの rpath から探せるように、インストール名を `@rpath/` と版の付いたファイル名にする
    place(platform, shared, join(lib, versioned(abi)), platform === "macos-universal" ? id : null);
    symlinkSync(versioned(abi), join(lib, shared));
  } else {
    place(platform, shared, join(lib, shared));
  }
  for (const file of rest) place(platform, file, join(lib, file));
}

function pack(args) {
  let out = join(root, "target", "ffi-dist");
  let tag = null;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--out" && args[i + 1]) out = args[++i];
    else if (args[i] === "--tag" && args[i + 1]) tag = args[++i];
    else fail(`unknown argument: ${args[i]}`);
  }
  const platform = platformOf(process.platform, process.arch);
  if (!platform) fail(`no C ABI archive for ${process.platform}-${process.arch}`);
  const version = workspaceVersion(readFileSync(join(root, "Cargo.toml"), "utf8"));
  if (tag) {
    const problem = tagProblem(tag, version);
    if (problem) fail(problem);
  }

  // 配るヘッダは `just ffi-check` が見るものと同じ (cbindgen の生成物と食い違っていれば止まる)
  checkHeader();
  const abi = abiVersion();

  const name = stem(version, platform);
  const stage = join(out, name);
  rmSync(stage, { recursive: true, force: true });
  build(platform, join(stage, "lib"), abi);
  mkdirSync(join(stage, "include"), { recursive: true });
  copyFileSync(join(crate, "include", "ugoe.h"), join(stage, "include", "ugoe.h"));
  mkdirSync(join(stage, "examples"), { recursive: true });
  copyFileSync(join(crate, "examples", "c", "sample.c"), join(stage, "examples", "sample.c"));
  copyFileSync(join(root, "LICENSE"), join(stage, "LICENSE"));
  // 入っている Rust のクレートのライセンスの表記 (scripts/dist-notices.mjs が生成する。docs/release.md#mit-の配布物)
  copyFileSync(join(crate, "THIRD-PARTY-NOTICES.txt"), join(stage, "THIRD-PARTY-NOTICES.txt"));
  const readme = readFileSync(join(crate, "archive", "README.md"), "utf8");
  writeFileSync(join(stage, "README.md"), readme.replaceAll("@VERSION@", version));

  const files = listFiles(stage);
  const expected = expectedFiles(platform, abi);
  if (files.join("\n") !== expected.join("\n")) {
    fail(
      `unexpected files in ${stage}:\n  got:      ${files.join(", ")}\n  expected: ${expected.join(", ")}`,
    );
  }

  const archive = join(out, archiveName(version, platform));
  rmSync(archive, { force: true });
  // Windows の tar (bsdtar) は `-a` で拡張子から zip を選ぶ
  if (PLATFORMS[platform].format === "zip")
    run(tar(), ["-a", "-c", "-f", archive, "-C", out, name]);
  else run(tar(), ["-c", "-z", "-f", archive, "-C", out, name]);
  const sha = writeChecksum(archive);
  console.log(`wrote ${archive} (sha256 ${sha})`);
  for (const f of files) console.log(`  ${name}/${f}`);
}

function buildPluginCommand(args) {
  let platform = null;
  let out = null;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--platform" && args[i + 1]) platform = args[++i];
    else if (args[i] === "--out" && args[i + 1]) out = args[++i];
    else fail(`unknown argument: ${args[i]}`);
  }
  if (!platform || !out)
    fail("usage: node scripts/ffi-dist.mjs build-plugin --platform <name> --out <dir>");
  const plugins = Object.keys(BUILDS).filter((p) => BUILDS[p].plugin);
  if (!plugins.includes(platform)) {
    fail(`no Unity plugin for ${platform} (one of: ${plugins.join(", ")})`);
  }
  console.log(`wrote ${buildPlugin(platform, out)}`);
}

/** `pack` が既定の置き場 (target/ffi-dist) に作る、手元の OS のアーカイブ。 */
function defaultArchive() {
  const platform = platformOf(process.platform, process.arch);
  if (!platform) fail(`no C ABI archive for ${process.platform}-${process.arch}`);
  const version = workspaceVersion(readFileSync(join(root, "Cargo.toml"), "utf8"));
  return join(root, "target", "ffi-dist", archiveName(version, platform));
}

function verify(args) {
  if (args.length > 1) fail("usage: node scripts/ffi-dist.mjs verify [<archive>]");
  const archive = args[0] ?? defaultArchive();
  const parsed = parseArchiveName(basename(archive));
  if (!parsed) fail(`not a C ABI archive name: ${basename(archive)}`);
  const { version, platform } = parsed;
  if (platform !== platformOf(process.platform, process.arch)) {
    fail(`${basename(archive)} is for ${platform}; it can only be verified on that platform`);
  }

  const sums = `${archive}.sha256`;
  if (existsSync(sums)) {
    const want = readChecksum(sums);
    const got = sha256File(archive);
    if (want !== got) fail(`sha256 mismatch for ${archive}: ${got} (expected ${want})`);
    console.log("sha256 ok");
  }

  const tmp = mkdtempSync(join(tmpdir(), "ugoe-ffi-dist-"));
  try {
    run(tar(), ["-x", "-f", archive, "-C", tmp]);
    const top = readdirSync(tmp);
    if (top.join() !== stem(version, platform))
      fail(`expected a single directory ${stem(version, platform)}, got ${top.join(", ")}`);
    const dir = join(tmp, top[0]);
    // 版の付いた名前は、アーカイブの中のヘッダの ABI の版から作る (利用者がビルドに使うヘッダと食い違わないこと)
    const abi = abiVersionFromHeader(readFileSync(join(dir, "include", "ugoe.h"), "utf8"));
    if (abi === null) fail("UGOE_ABI_VERSION not found in include/ugoe.h");
    const files = listFiles(dir);
    const expected = expectedFiles(platform, abi);
    if (files.join("\n") !== expected.join("\n")) {
      fail(
        `unexpected files in ${basename(archive)}:\n  got:      ${files.join(", ")}\n  expected: ${expected.join(", ")}`,
      );
    }
    console.log(`files ok (${files.length})`);
    checkSharedLibraryName(platform, join(dir, "lib"), abi);

    // アーカイブの中のヘッダ・ライブラリ・見本だけでビルドする (リポジトリのヘッダやビルドの成果物を見ない)
    const include = join(dir, "include");
    const source = join(dir, "examples", "sample.c");
    const lib = join(dir, "lib");
    const windows = platform === "windows-x86_64";
    const ext = windows ? ".exe" : "";

    const staticLib = join(lib, windows ? "ugoe_ffi.lib" : "libugoe_ffi.a");
    const staticExe = join(tmp, `sample-static${ext}`);
    compileSample({
      source,
      include,
      link: [staticLib, ...(STATIC_SYSTEM_LIBS[process.platform] ?? [])],
      exe: staticExe,
    });
    runSample(staticExe);
    console.log("static library ok");

    const sharedExe = join(tmp, `sample-shared${ext}`);
    if (windows) {
      // インポートライブラリにリンクし、DLL を実行ファイルの隣に置く (Windows は実行ファイルのディレクトリから DLL を探す)
      compileSample({
        source,
        include,
        link: [join(lib, "ugoe_ffi.dll.lib")],
        exe: sharedExe,
      });
      copyFileSync(join(lib, "ugoe_ffi.dll"), join(dirname(sharedExe), "ugoe_ffi.dll"));
    } else {
      // アーカイブの README と同じく `-lugoe_ffi` (版の無い名前のシンボリックリンク) でリンクし、rpath で lib/ から読む。
      // 実行ファイルには soname・インストール名 (版の付いた名前) が残るので、読むのはシンボリックリンクでなく実体
      const link = ["-L", lib, "-lugoe_ffi", ...(platform === "macos-universal" ? [] : ["-lm"])];
      compileSample({ source, include, link: [...link, `-Wl,-rpath,${lib}`], exe: sharedExe });
      checkLinkedName(platform, sharedExe, abi);
    }
    runSample(sharedExe);
    console.log("shared library ok");
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

/** コマンドを実行し、標準出力を返す。失敗したら終える。 */
function capture(cmd, args) {
  const r = spawnSync(cmd, args, { encoding: "utf8" });
  if (r.error) fail(`failed to run ${cmd}: ${r.error.message}`);
  if (r.status !== 0) fail(`${cmd} ${args.join(" ")} failed:\n${r.stderr}`);
  return r.stdout;
}

/** `readelf -d` の出力から、`tag` (`SONAME`・`NEEDED`) の名前の一覧。 */
export function readelfNames(output, tag) {
  const re = new RegExp(`\\(${tag}\\)\\s+\\S.*\\[(.+)\\]`, "g");
  return [...output.matchAll(re)].map((m) => m[1]);
}

/**
 * 展開した `lib/` の共有ライブラリが、版の付いた名前の実体とそこへのシンボリックリンクで、soname (`readelf -d`)・
 * インストール名 (`otool -D`) が版の付いた名前かを確かめる。持たないプラットフォーム (Windows) では何もしない。
 */
function checkSharedLibraryName(platform, lib, abi) {
  const { lib: files, versioned } = PLATFORMS[platform];
  if (!versioned) return;
  const link = join(lib, files[0]);
  const real = join(lib, versioned(abi));
  // tar の展開でシンボリックリンクが保たれたか (実体の写しになっていないか)
  if (!lstatSync(link).isSymbolicLink()) fail(`${files[0]} is not a symbolic link`);
  if (readlinkSync(link) !== versioned(abi))
    fail(`${files[0]} points to ${readlinkSync(link)} (expected ${versioned(abi)})`);
  if (!lstatSync(real).isFile()) fail(`${versioned(abi)} is not a regular file`);
  const want = sharedLibraryId(platform, abi);
  const got =
    platform === "macos-universal"
      ? capture("otool", ["-D", real]).trim().split("\n").slice(1)
      : readelfNames(capture("readelf", ["-d", real]), "SONAME");
  if (got.join() !== want)
    fail(`${versioned(abi)} is named ${got.join(", ") || "(none)"} (expected ${want})`);
  console.log(`shared library name ok (${want})`);
}

/** 共有ライブラリにリンクした実行ファイルが、版の付いた名前 (soname・インストール名) を記録したかを確かめる。 */
function checkLinkedName(platform, exe, abi) {
  const want = sharedLibraryId(platform, abi);
  const names =
    platform === "macos-universal"
      ? capture("otool", ["-L", exe])
          .split("\n")
          .slice(1)
          .map((l) => l.trim().split(" ")[0])
      : readelfNames(capture("readelf", ["-d", exe]), "NEEDED");
  if (!names.includes(want))
    fail(`${basename(exe)} does not depend on ${want}: ${names.join(", ")}`);
}

if (import.meta.main) {
  // mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
  exportMbxPath();
  const [command, ...args] = process.argv.slice(2);
  if (command === "pack") pack(args);
  else if (command === "verify") verify(args);
  else if (command === "build-plugin") buildPluginCommand(args);
  else {
    console.error(
      "usage: node scripts/ffi-dist.mjs pack [--out <dir>] [--tag <tag>] | verify [<archive>]" +
        " | build-plugin --platform <name> --out <dir>",
    );
    process.exit(2);
  }
}
