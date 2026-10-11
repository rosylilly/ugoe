#!/usr/bin/env node
// 秘密の値 (API キー・トークン・秘密鍵など) がリポジトリに入っていないことを gitleaks で検査する
// (T0098。`just secrets`。docs/release.md#自動の検査)。
//
// gitleaks は版と sha256 を固定し (下の GITLEAKS_VERSION と GITLEAKS_ASSETS)、GitHub の Release から
// `<メインの作業ツリー>/target/tools/gitleaks-<版>/` に取得する (scripts/lib/release-tool.mjs。worktree でも共有する。リポジトリには置かない)。
// 設定は scripts/gitleaks.toml (既定の規則と、秘密の値ではないものを外す allowlist)。
//
// 見るものは 2 つ:
// - 作業ツリー: Git が見るファイル (追うものと、無視されていない新しいもの) を `gitleaks dir` で。
//   `.gitignore` で無視するもの (target/・node_modules/・.worktrees/ など) は設定に足して外す。
// - コミット: 既定は履歴のすべて (`--log-opts=--all`)。`--since <rev>` を渡すと `<rev>..HEAD` だけ (CI の push・PR の差分)。
//   浅い clone (CI の actions/checkout) なら、範囲のコミットがそろうまで `git fetch --deepen` で深くする。
//   `<rev>` が無い・すべて 0 (新しいブランチ)・取れない (履歴の書き換え) なら、履歴のすべてを見る。
//
// 見つけた値はログに出さない (`--redact`。CI のログは公開すると見える)。

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { ensureReleaseTool } from "./lib/release-tool.mjs";

const root = join(import.meta.dirname, "..");

/** 固定した gitleaks の版。上げるときは GITLEAKS_ASSETS の sha256 も Release の checksums.txt から写す。 */
export const GITLEAKS_VERSION = "8.30.1";

/**
 * 実行する機械 (`${process.platform}-${process.arch}`) ごとの Release のファイルと sha256
 * (https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_checksums.txt)。
 */
export const GITLEAKS_ASSETS = {
  "linux-x64": {
    file: "gitleaks_8.30.1_linux_x64.tar.gz",
    sha256: "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb",
  },
  "linux-arm64": {
    file: "gitleaks_8.30.1_linux_arm64.tar.gz",
    sha256: "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080",
  },
  "darwin-x64": {
    file: "gitleaks_8.30.1_darwin_x64.tar.gz",
    sha256: "dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709",
  },
  "darwin-arm64": {
    file: "gitleaks_8.30.1_darwin_arm64.tar.gz",
    sha256: "b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5",
  },
  "win32-x64": {
    file: "gitleaks_8.30.1_windows_x64.zip",
    sha256: "d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e",
  },
};

/** 設定 (リポジトリのルートから)。 */
export const CONFIG = "scripts/gitleaks.toml";

/** 浅い clone を 1 回に深くするコミットの数と、その回数の上限 (超えたら履歴のすべてを取る)。 */
const DEEPEN_STEP = 50;
const MAX_DEEPEN = 20;

/** 実行する機械の Release のファイル。対応していなければ null。 */
export function assetFor(platform = process.platform, arch = process.arch) {
  return GITLEAKS_ASSETS[`${platform}-${arch}`] ?? null;
}

/** Release のファイルの URL。 */
export function assetUrl(asset, version = GITLEAKS_VERSION) {
  return `https://github.com/gitleaks/gitleaks/releases/download/v${version}/${asset.file}`;
}

/** 正規表現の特別な文字を逃がす。 */
function escapeRegex(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * `git ls-files --others --ignored --exclude-standard --directory` の行 (無視するファイルと、`/` で終わるディレクトリ) から、
 * gitleaks の allowlist の `paths` の正規表現を作る。区切りは `/` と `\` (Windows) のどちらにも合わせる。
 */
export function ignoredPathPatterns(entries) {
  const patterns = [];
  for (const entry of entries) {
    if (entry === "") continue;
    const dir = entry.endsWith("/");
    const parts = (dir ? entry.slice(0, -1) : entry).split("/").map(escapeRegex);
    patterns.push(`^${parts.join("[\\\\/]")}${dir ? "[\\\\/]" : "$"}`);
  }
  return patterns;
}

/** TOML の複数行のリテラル文字列 (`'''…'''`)。正規表現の `\` をそのまま書ける。 */
function tomlLiteral(text) {
  if (text.includes("'''")) throw new Error(`cannot write ${text} as a TOML literal string`);
  return `'''${text}'''`;
}

/** 設定 (scripts/gitleaks.toml の中身) に、Git が無視するパスを外す allowlist を足したもの。 */
export function configWithIgnored(base, ignored) {
  const patterns = [...ignoredPathPatterns(ignored), "^\\.git([\\\\/]|$)"];
  return [
    base.trimEnd(),
    "",
    "# scripts/secrets.mjs が足したもの: Git が無視するファイル (作業ツリーの検査で見ない)",
    "[[allowlists]]",
    'description = "ignored by git"',
    "paths = [",
    ...patterns.map((p) => `  ${tomlLiteral(p)},`),
    "]",
    "",
  ].join("\n");
}

/** すべて 0 のコミットの ID (push の前のコミットが無い = 新しいブランチ)。 */
export function isNullRev(rev) {
  return /^0+$/.test(rev);
}

/** 省略の無いコミットの ID (40 桁か 64 桁の 16 進)。 */
export function isCommitId(rev) {
  return /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(rev);
}

/** コマンドの引数。`--since <rev>` (無い・空・すべて 0 なら null)。 */
export function parseArgs(argv) {
  let since = null;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--since") {
      if (i + 1 >= argv.length) throw new Error("--since needs a revision");
      since = argv[++i];
    } else if (arg.startsWith("--since=")) {
      since = arg.slice("--since=".length);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (since !== null && (since.trim() === "" || isNullRev(since.trim()))) since = null;
  return { since: since?.trim() ?? null };
}

function git(args, options = {}) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8", ...options }).trim();
}

function tryGit(args) {
  const r = spawnSync("git", args, { cwd: root, encoding: "utf8" });
  return r.status === 0 ? r.stdout.trim() : null;
}

/** 固定した版の gitleaks を (無ければ取得して sha256 を確かめてから) 用意し、実行ファイルのパスを返す。 */
async function ensureGitleaks() {
  const asset = assetFor();
  if (!asset) {
    throw new Error(
      `gitleaks ${GITLEAKS_VERSION} is not set up for ${process.platform}-${process.arch} (scripts/secrets.mjs の GITLEAKS_ASSETS)`,
    );
  }
  return ensureReleaseTool({
    root,
    name: "gitleaks",
    version: GITLEAKS_VERSION,
    asset: { ...asset, exe: process.platform === "win32" ? "gitleaks.exe" : "gitleaks" },
    url: assetUrl(asset),
  });
}

/** 浅い clone のときに、範囲 `<since>..HEAD` のコミットがすべてそろうまで深くする。そろえば true。 */
function deepenFor(since) {
  const head = git(["rev-parse", "HEAD"]);
  const hasSince = () => tryGit(["rev-parse", "--verify", "--quiet", `${since}^{commit}`]) !== null;
  // コミットの ID (push の前のコミット) なら、直に取れるかを先に確かめる。取れなければ (履歴の書き換えで消えた) 深くしても無駄
  if (isCommitId(since) && !hasSince()) {
    const r = spawnSync("git", ["fetch", "--no-tags", "--quiet", "--depth=1", "origin", since], {
      cwd: root,
      stdio: "inherit",
    });
    if (r.status !== 0) return false;
  }
  for (let i = 0; ; i++) {
    if (tryGit(["rev-parse", "--is-shallow-repository"]) !== "true") {
      return hasSince();
    }
    if (hasSince()) {
      // 範囲に浅い境目 (親を取っていないコミット) が無ければ、範囲のコミットはすべてある
      const shallowFile = git(["rev-parse", "--git-path", "shallow"]);
      const boundary = new Set(
        existsSync(resolve(root, shallowFile))
          ? readFileSync(resolve(root, shallowFile), "utf8").split(/\s+/).filter(Boolean)
          : [],
      );
      const range = git(["rev-list", `${since}..HEAD`])
        .split(/\s+/)
        .filter(Boolean);
      if (!range.some((c) => boundary.has(c))) return true;
    }
    if (i >= MAX_DEEPEN) return false;
    const r = spawnSync(
      "git",
      ["fetch", "--no-tags", "--quiet", `--deepen=${DEEPEN_STEP}`, "origin", head],
      {
        cwd: root,
        stdio: "inherit",
      },
    );
    if (r.status !== 0) return false;
  }
}

/** 履歴のすべてを見られるようにする (浅い clone なら残りを取る)。 */
function unshallow() {
  if (tryGit(["rev-parse", "--is-shallow-repository"]) !== "true") return;
  console.log("fetching the whole history");
  const r = spawnSync("git", ["fetch", "--no-tags", "--quiet", "--unshallow", "origin"], {
    cwd: root,
    stdio: "inherit",
  });
  if (r.status !== 0) throw new Error("failed to fetch the whole history");
}

/** gitleaks を実行する。見つかれば false。 */
function runGitleaks(exe, args) {
  const r = spawnSync(exe, [...args, "--no-banner", "--redact", "--verbose"], {
    cwd: root,
    stdio: "inherit",
  });
  if (r.error) throw r.error;
  return r.status === 0;
}

async function main(argv) {
  const { since } = parseArgs(argv);
  const exe = await ensureGitleaks();

  // 作業ツリー: 設定に、Git が無視するパスを足した一時的な設定で見る
  const ignored = git([
    "ls-files",
    "-z",
    "--others",
    "--ignored",
    "--exclude-standard",
    "--directory",
  ]).split("\0");
  const config = join(dirname(exe), `config-${process.pid}.toml`);
  writeFileSync(config, configWithIgnored(readFileSync(join(root, CONFIG), "utf8"), ignored));
  let ok;
  try {
    console.log("== working tree");
    ok = runGitleaks(exe, ["dir", ".", "--config", config]);
  } finally {
    rmSync(config, { force: true });
  }

  // コミット
  let logOpts = "--all";
  if (since !== null) {
    if (deepenFor(since)) {
      logOpts = `${since}..HEAD`;
    } else {
      console.log(`could not get the commits ${since}..HEAD; scanning the whole history`);
      unshallow();
    }
  }
  console.log(`== commits (${logOpts})`);
  ok =
    runGitleaks(exe, ["git", ".", "--config", join(root, CONFIG), `--log-opts=${logOpts}`]) && ok;

  if (!ok) {
    console.error(
      "error: gitleaks found possible secrets. Remove them (and rotate them if they are real), or if they are not secrets, add them to scripts/gitleaks.toml with the reason (docs/release.md#自動の検査)",
    );
    process.exit(1);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main(process.argv.slice(2)).catch((err) => {
    console.error(`error: ${err.message}`);
    process.exit(1);
  });
}
