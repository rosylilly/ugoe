#!/usr/bin/env node
// ugoe の Unity パッケージ (bindings/unity) の準備と検査。Unity は要らない。
//
//   node bindings/unity/build.mjs              # ホスト向けの ugoe-ffi をビルドして Runtime/Plugins/ に置き、
//                                                      # 見本のモデルを Samples~/Basic/Models/ に写す (.meta も作る)
//   node bindings/unity/build.mjs --release    # リリースビルド
//   node bindings/unity/build.mjs --check      # ビルドして、.meta の検査・Runtime/Core の dotnet test・
//                                                      # Unity に依存する C# の型の検査 (tests/UnityCompile) を行う (dotnet が要る)
//   node bindings/unity/build.mjs --metas      # コミットするファイルに足りない .meta を作る (GUID はパスから決める)
//   node bindings/unity/build.mjs --plugins <dir>
//       # CI の成果物 (<dir>/<プラットフォーム>/<ライブラリ>。下の PLUGINS の from) を Runtime/Plugins/ に並べる。ビルドはしない
//
// ライブラリは target/ (リポジトリのルートと共有。CARGO_TARGET_DIR で変えられる) から写す。
// パスは UPM パッケージ (com.ugoe.unity/) の中。Runtime/Plugins/ と Samples~/Basic/Models/ はコミットしない (.gitignore)。

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
// UPM パッケージのルート (package.json のあるところ)
const pkg = join(here, "com.ugoe.unity");
const root = join(here, "..", "..");
const args = process.argv.slice(2);
const release = args.includes("--release");
const check = args.includes("--check");
const metasOnly = args.includes("--metas");
const pluginsFrom = args[args.indexOf("--plugins") + 1];
const usePlugins = args.includes("--plugins");

// ネイティブライブラリの置き場所と、Unity のプラグインの設定 (PluginImporter の platformData)。
// from: CI の成果物での場所 (--plugins)。
const PLUGINS = [
  {
    path: "Linux/x86_64/libugoe_ffi.so",
    from: "linux-x86_64/libugoe_ffi.so",
    host: "linux",
    editor: { OS: "Linux", CPU: "x86_64" },
    platforms: [["Standalone", "Linux64", { CPU: "x86_64" }]],
  },
  {
    path: "Windows/x86_64/ugoe_ffi.dll",
    from: "windows-x86_64/ugoe_ffi.dll",
    host: "win32",
    editor: { OS: "Windows", CPU: "x86_64" },
    platforms: [["Standalone", "Win64", { CPU: "x86_64" }]],
  },
  {
    path: "macOS/libugoe_ffi.dylib",
    from: "macos-universal/libugoe_ffi.dylib",
    host: "darwin",
    editor: { OS: "OSX", CPU: "AnyCPU" },
    platforms: [["Standalone", "OSXUniversal", { CPU: "AnyCPU" }]],
  },
  ...[
    ["arm64-v8a", "ARM64"],
    ["armeabi-v7a", "ARMv7"],
    ["x86_64", "X86_64"],
  ].map(([abi, cpu]) => ({
    path: `Android/${abi}/libugoe_ffi.so`,
    from: `android-${abi}/libugoe_ffi.so`,
    platforms: [["Android", "Android", { CPU: cpu }]],
  })),
  {
    path: "iOS/libugoe_ffi.a",
    from: "ios-arm64/libugoe_ffi.a",
    platforms: [["iPhone", "iOS", { AddToEmbeddedBinaries: "false", CPU: "AnyCPU" }]],
  },
];

const SAMPLE_MODELS = [
  "avatar.ugoe",
  "avatar-lit.ugoe",
  ...[
    "dress-up.ugoe",
    "long-hair.ugoemod",
    "twin-tails.ugoemod",
    "sailor.ugoemod",
    "cat-ears.ugoemod",
  ].map((n) => join("dress-up", n)),
];

// --- .meta ---

/** パッケージの中のパスから決まる GUID (作り直しても変わらないように)。 */
function guidFor(rel) {
  return createHash("md5")
    .update(`ugoe-unity/${rel.replaceAll("\\", "/")}`)
    .digest("hex");
}

const TAIL = "  userData: \n  assetBundleName: \n  assetBundleVariant: \n";

/** ファイルの種類ごとの .meta の中身。 */
function metaBody(file, isDir) {
  if (isDir) return `folderAsset: yes\nDefaultImporter:\n  externalObjects: {}\n${TAIL}`;
  const ext = file.slice(file.lastIndexOf(".") + 1);
  switch (ext) {
    case "cs":
      return `MonoImporter:\n  externalObjects: {}\n  serializedVersion: 2\n  defaultReferences: []\n  executionOrder: 0\n  icon: {instanceID: 0}\n${TAIL}`;
    case "asmdef":
      return `AssemblyDefinitionImporter:\n  externalObjects: {}\n${TAIL}`;
    case "shader":
      return `ShaderImporter:\n  externalObjects: {}\n  defaultTextures: []\n  nonModifiableTextures: []\n${TAIL}`;
    case "hlsl":
      return `ShaderIncludeImporter:\n  externalObjects: {}\n${TAIL}`;
    case "json":
      return basename(file) === "package.json"
        ? `PackageManifestImporter:\n  externalObjects: {}\n${TAIL}`
        : `TextScriptImporter:\n  externalObjects: {}\n${TAIL}`;
    case "md":
    case "txt":
      return `TextScriptImporter:\n  externalObjects: {}\n${TAIL}`;
    case "ugoe":
    case "ugoemod": {
      // Editor/UgoeImporters.cs の ScriptedImporter で取り込む。
      const importer = guidFor("Editor/UgoeImporters.cs");
      return `ScriptedImporter:\n  internalIDToNameTable: []\n  externalObjects: {}\n  serializedVersion: 2\n${TAIL}  script: {fileID: 11500000, guid: ${importer}, type: 3}\n`;
    }
    default:
      return `DefaultImporter:\n  externalObjects: {}\n${TAIL}`;
  }
}

function pluginMeta(plugin) {
  const settings = (obj, indent) =>
    Object.keys(obj).length === 0
      ? " {}\n"
      : `\n${Object.entries(obj)
          .map(([k, v]) => `${indent}${k}: ${v}`)
          .join("\n")}\n`;
  const entry = (key, value, enabled, s) =>
    `  - first:\n      ${key}: ${value}\n    second:\n      enabled: ${enabled ? 1 : 0}\n      settings:${settings(s, "        ")}`;
  let data = entry("Any", "", false, {});
  data += plugin.editor
    ? entry("Editor", "Editor", true, { ...plugin.editor, DefaultValueInitialized: "true" })
    : entry("Editor", "Editor", false, { DefaultValueInitialized: "true" });
  for (const [group, platform, s] of plugin.platforms) data += entry(group, platform, true, s);
  return (
    `PluginImporter:\n  externalObjects: {}\n  serializedVersion: 2\n  iconMap: {}\n  executionOrder: {}\n` +
    `  defineConstraints: []\n  isPreloaded: 0\n  isOverridable: 0\n  isExplicitlyReferenced: 0\n  validateReferences: 1\n` +
    `  platformData:\n${data}${TAIL}`
  );
}

function writeMeta(abs, body) {
  const rel = relative(pkg, abs);
  writeFileSync(`${abs}.meta`, `fileFormatVersion: 2\nguid: ${guidFor(rel)}\n${body}`);
}

/** Unity が取り込むファイル (名前が . で始まるものと ~ で終わるフォルダは Unity が無視する)。 */
function* assets(dir) {
  for (const name of readdirSync(dir).sort()) {
    if (name.startsWith(".") || name.endsWith("~") || name.endsWith(".meta")) continue;
    const abs = join(dir, name);
    yield abs;
    if (statSync(abs).isDirectory()) yield* assets(abs);
  }
}

/**
 * Samples~ の中の見本も、利用者のプロジェクトに写されたときに取り込まれるので .meta が要る
 * (見本のフォルダ自体は写し先で作られるので要らない)。
 */
function* allAssets() {
  yield* assets(pkg);
  const samples = join(pkg, "Samples~");
  for (const name of readdirSync(samples)) {
    const abs = join(samples, name);
    if (statSync(abs).isDirectory()) yield* assets(abs);
  }
}

function makeMetas() {
  let made = 0;
  for (const abs of allAssets()) {
    if (existsSync(`${abs}.meta`)) continue;
    writeMeta(abs, metaBody(abs, statSync(abs).isDirectory()));
    made++;
  }
  return made;
}

function checkMetas() {
  const problems = [];
  const guids = new Map();
  for (const abs of allAssets()) {
    const rel = relative(pkg, abs);
    if (!existsSync(`${abs}.meta`)) {
      problems.push(`missing ${rel}.meta (run: node bindings/unity/build.mjs --metas)`);
      continue;
    }
    const guid = /^guid: ([0-9a-f]{32})$/m.exec(readFileSync(`${abs}.meta`, "utf8"))?.[1];
    if (!guid) problems.push(`${rel}.meta has no guid`);
    else if (guids.has(guid)) problems.push(`${rel}.meta has the same guid as ${guids.get(guid)}`);
    else guids.set(guid, rel);
  }
  // 対になるファイルの無い .meta (消したファイルの残り)
  const orphans = (dir) => {
    for (const name of readdirSync(dir)) {
      const abs = join(dir, name);
      if (name.endsWith(".meta") && !existsSync(abs.slice(0, -5)))
        problems.push(`orphan ${relative(pkg, abs)}`);
      else if (!name.endsWith("~") && !name.startsWith(".") && statSync(abs).isDirectory())
        orphans(abs);
    }
  };
  orphans(pkg);
  for (const name of readdirSync(join(pkg, "Samples~"))) {
    const abs = join(pkg, "Samples~", name);
    if (name.endsWith(".meta")) problems.push(`unneeded ${relative(pkg, abs)}`);
    else if (statSync(abs).isDirectory()) orphans(abs);
  }
  if (problems.length > 0) {
    for (const p of problems) console.error(p);
    throw new Error(`${problems.length} .meta problem(s)`);
  }
  console.log(`.meta: ${guids.size} assets ok`);
}

// --- ネイティブライブラリ ---

function placePlugin(plugin, source) {
  const dest = join(pkg, "Runtime", "Plugins", plugin.path);
  mkdirSync(dirname(dest), { recursive: true });
  copyFileSync(source, dest);
  // フォルダの .meta (Runtime/Plugins から下)
  let dir = dirname(dest);
  while (dir !== join(pkg, "Runtime")) {
    if (!existsSync(`${dir}.meta`)) writeMeta(dir, metaBody(dir, true));
    dir = dirname(dir);
  }
  writeMeta(dest, pluginMeta(plugin));
  console.log(`placed Runtime/Plugins/${plugin.path}`);
}

function buildHost() {
  const targetDir = process.env.CARGO_TARGET_DIR ?? join(root, "target");
  const cargoArgs = ["build", "--locked", "-p", "ugoe-ffi"];
  if (release) cargoArgs.push("--release");
  execFileSync("cargo", cargoArgs, {
    cwd: root,
    stdio: "inherit",
    env: { CARGO_BUILD_JOBS: "8", ...process.env, CARGO_TARGET_DIR: targetDir },
  });
  const plugin = PLUGINS.find((p) => p.host === process.platform);
  if (!plugin) throw new Error(`unsupported host platform: ${process.platform}`);
  const out = join(targetDir, release ? "release" : "debug");
  placePlugin(plugin, join(out, basename(plugin.path)));
  return out;
}

function copySamples() {
  const models = join(pkg, "Samples~", "Basic", "Models");
  mkdirSync(models, { recursive: true });
  if (!existsSync(`${models}.meta`)) writeMeta(models, metaBody(models, true));
  for (const model of SAMPLE_MODELS) {
    const dest = join(models, basename(model));
    copyFileSync(join(root, "assets", "samples", model), dest);
    writeMeta(dest, metaBody(dest, false));
  }
  console.log(`copied ${SAMPLE_MODELS.length} sample models into Samples~/Basic/Models`);
}

function dotnet(...dotnetArgs) {
  try {
    execFileSync("dotnet", ["--version"], { stdio: "ignore" });
  } catch {
    console.error(
      "dotnet (the .NET SDK 8 or later) is required for --check.\n" +
        "Install it from https://dotnet.microsoft.com/download (or `sudo apt-get install dotnet-sdk-8.0` on Ubuntu).",
    );
    process.exit(1);
  }
  execFileSync("dotnet", dotnetArgs, { cwd: here, stdio: "inherit" });
}

if (metasOnly) {
  console.log(`made ${makeMetas()} .meta file(s)`);
} else if (usePlugins) {
  if (!pluginsFrom || pluginsFrom.startsWith("--")) throw new Error("--plugins needs a directory");
  let placed = 0;
  for (const plugin of PLUGINS) {
    const source = join(pluginsFrom, plugin.from);
    if (existsSync(source)) {
      placePlugin(plugin, source);
      placed++;
    } else {
      console.warn(`missing ${plugin.from}; skipped`);
    }
  }
  if (placed === 0) throw new Error(`no native libraries found in ${pluginsFrom}`);
} else {
  const out = buildHost();
  copySamples();
  if (check) {
    checkMetas();
    dotnet("test", "tests/Ugoe.Tests.csproj", `-p:UgoeFfiDir=${out}`);
    dotnet("build", "tests/UnityCompile/Ugoe.UnityCompile.csproj");
  }
}
