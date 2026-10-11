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
//       見本をビルドして動かす (静的ライブラリと共有ライブラリの両方)。
//
// macOS では arm64 と x86_64 を作って lipo でまとめる (universal)。rustup で 2 つのターゲットを入れておく:
//   rustup target add aarch64-apple-darwin x86_64-apple-darwin

import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative } from "node:path";
import { workspaceVersion } from "./crates.mjs";
import { checkHeader, compileSample, run, runSample, STATIC_SYSTEM_LIBS } from "./ffi.mjs";
import { exportMbxPath } from "./lib/mbx.mjs";

const root = join(import.meta.dirname, "..");
const crate = join(root, "crates", "ugoe-ffi");

/**
 * 配布するプラットフォーム。`lib` はアーカイブの `lib/` に入れるファイル (共有ライブラリ・静的ライブラリ。Windows は
 * DLL のインポートライブラリ `.dll.lib` も)。`format` はアーカイブの形。
 */
export const PLATFORMS = {
  "linux-x86_64": { lib: ["libugoe_ffi.so", "libugoe_ffi.a"], format: "tar.gz" },
  "linux-aarch64": { lib: ["libugoe_ffi.so", "libugoe_ffi.a"], format: "tar.gz" },
  "windows-x86_64": { lib: ["ugoe_ffi.dll", "ugoe_ffi.dll.lib", "ugoe_ffi.lib"], format: "zip" },
  "macos-universal": { lib: ["libugoe_ffi.dylib", "libugoe_ffi.a"], format: "tar.gz" },
};

/** macOS の universal に入れるアーキテクチャのターゲット。 */
const MACOS_TARGETS = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

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

/** アーカイブの中のファイル (最上位のディレクトリからの相対パス、`/` 区切り、並べ替え済み)。 */
export function expectedFiles(platform) {
  return [
    "LICENSE",
    "README.md",
    "THIRD-PARTY-NOTICES.txt",
    "examples/sample.c",
    "include/ugoe.h",
    ...PLATFORMS[platform].lib.map((f) => `lib/${f}`),
  ].sort();
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
      if (statSync(path).isDirectory()) walk(path);
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

/** 手元の OS 向けに ugoe-ffi をリリースビルドし、`lib` に配るライブラリを置く。 */
function build(platform, lib) {
  mkdirSync(lib, { recursive: true });
  if (platform === "macos-universal") {
    for (const target of MACOS_TARGETS) {
      run("cargo", ["build", "--locked", "--release", "-p", "ugoe-ffi", "--target", target]);
    }
    for (const file of PLATFORMS[platform].lib) {
      run("lipo", [
        "-create",
        "-output",
        join(lib, file),
        ...MACOS_TARGETS.map((t) => join(root, "target", t, "release", file)),
      ]);
    }
    // 読み込むときに実行ファイルの rpath から探せるように、インストール名をファイル名だけにする (unity.yml と同じ)
    run("install_name_tool", ["-id", "@rpath/libugoe_ffi.dylib", join(lib, "libugoe_ffi.dylib")]);
    return;
  }
  run("cargo", ["build", "--locked", "--release", "-p", "ugoe-ffi"]);
  for (const file of PLATFORMS[platform].lib) {
    copyFileSync(join(root, "target", "release", file), join(lib, file));
  }
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

  const name = stem(version, platform);
  const stage = join(out, name);
  rmSync(stage, { recursive: true, force: true });
  build(platform, join(stage, "lib"));
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
  const expected = expectedFiles(platform);
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
  const sha = createHash("sha256").update(readFileSync(archive)).digest("hex");
  writeFileSync(`${archive}.sha256`, `${sha}  ${basename(archive)}\n`);
  console.log(`wrote ${archive} (sha256 ${sha})`);
  for (const f of files) console.log(`  ${name}/${f}`);
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
    const want = readFileSync(sums, "utf8").split(/\s+/)[0];
    const got = createHash("sha256").update(readFileSync(archive)).digest("hex");
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
    const files = listFiles(dir);
    const expected = expectedFiles(platform);
    if (files.join("\n") !== expected.join("\n")) {
      fail(
        `unexpected files in ${basename(archive)}:\n  got:      ${files.join(", ")}\n  expected: ${expected.join(", ")}`,
      );
    }
    console.log(`files ok (${files.length})`);

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
      // rpath で lib/ から読む。Linux の共有ライブラリには soname が無いので `-l` で名前を書く (パスで渡すとパスが残る)
      const link =
        platform === "macos-universal"
          ? [join(lib, "libugoe_ffi.dylib")]
          : ["-L", lib, "-l:libugoe_ffi.so", "-lm"];
      compileSample({ source, include, link: [...link, `-Wl,-rpath,${lib}`], exe: sharedExe });
    }
    runSample(sharedExe);
    console.log("shared library ok");
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

if (import.meta.main) {
  // mbx (ビルドのキャッシュ。docs/workflow/build-cache.md) の shim が入っていれば、justfile と同じく cargo を mbx に通す
  exportMbxPath();
  const [command, ...args] = process.argv.slice(2);
  if (command === "pack") pack(args);
  else if (command === "verify") verify(args);
  else {
    console.error(
      "usage: node scripts/ffi-dist.mjs pack [--out <dir>] [--tag <tag>] | verify [<archive>]",
    );
    process.exit(2);
  }
}
