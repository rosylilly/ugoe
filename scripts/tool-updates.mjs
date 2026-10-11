#!/usr/bin/env node
// 版を手で固定した道具の一覧と、上流の新しい版に気づく検査 (T1004。`just tool-updates`。docs/release.md#固定した道具の版)。
//
// Dependabot は scripts/ の定数・ワークフローの `tool: <名前>@<版>`・.tool-versions などに書いた版を見ないので、
// ここに道具ごとに「版を書いた場所 (pin)」と「同じ版を写した場所 (mirrors)」と「上流 (GitHub の Release か crates.io)」を
// まとめる。版を上げるときに直す場所は、この一覧の pin と mirrors ですべて。
//
//   node scripts/tool-updates.mjs           上流の最新の版と比べて表を出す (ネットワークを使う。`just ci` には入れない)
//   node scripts/tool-updates.mjs --list    一覧と、写した場所の版が揃っているかだけ (ネットワークを使わない)
//
// 比べるだけで版は書き換えない。新しい版があっても終了コードは 0 (知らせるだけ)。GitHub Actions では表をジョブの要約
// (`$GITHUB_STEP_SUMMARY`) にも書く。上流を読めなかった道具は表に書いて警告にし、これも失敗にしない (リリースの前の検査を
// 上流の一時的な障害で止めないため)。写した場所の版が揃っていないときだけ 1 で終わる (単体テストでも確かめる)。
// GitHub の API は GH_TOKEN か GITHUB_TOKEN があれば使う (無くても 1 時間に 60 回まで読める)。

import { appendFileSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const root = join(import.meta.dirname, "..");

/** GitHub の Release のタグの既定の形 (`v1.2.3` か `1.2.3`)。`-rc1` などの付いた先行版は合わない。 */
const PLAIN_TAG = /^v?(\d+(?:\.\d+)*)$/;

/** crates.io の API に送る User-Agent (crates.io の方針で、連絡先の分かるものにする)。 */
const USER_AGENT = "ugoe-tool-updates (https://github.com/rosylilly/ugoe)";

/**
 * ワークフローの `extractions/setup-just` に渡す just の版 (`.tool-versions` の写し)。すべての setup-just が `with:` の
 * `just-version` を持つことは単体テストが確かめる。
 */
export const SETUP_JUST_VERSION =
  /uses: extractions\/setup-just@\S+\n\s*with:\n\s*just-version: ([^\s#]+)/g;

/** setup-just を使うワークフロー (足したらここにも足す。漏れは単体テストが見つける)。 */
export const SETUP_JUST_WORKFLOWS = [
  ".github/workflows/ci.yml",
  ".github/workflows/e2e.yml",
  ".github/workflows/editor-build.yml",
  ".github/workflows/fuzz.yml",
  ".github/workflows/nostd-build-std.yml",
  ".github/workflows/perf.yml",
  ".github/workflows/release-checks.yml",
  ".github/workflows/release-notes.yml",
  ".github/workflows/unity.yml",
  ".github/workflows/web-runtime-release.yml",
];

/**
 * 版を固定した道具の一覧。
 * - `pin`: 版を書いた元の場所 (リポジトリのルートからのファイルと、版を 1 つ目の組で取り出す正規表現)。
 * - `mirrors`: 同じ版を写した場所 (正規表現は `g` 付き。合うものがすべて pin と同じ版で、1 つ以上合わなければならない)。
 * - `upstream`: `{ github: "<owner>/<repo>" }` (Release のタグ) か `{ crate: "<名前>" }` (crates.io)。
 * - `tag`: Release のタグ (と pin の値) から比べる版を取り出す正規表現 (既定は PLAIN_TAG)。
 * - `bump`: 上げるときにほかにすること (docs/release.md#固定した道具の版 の表と同じ内容を短く)。
 */
export const PINNED_TOOLS = [
  {
    name: "gitleaks",
    pin: { file: "scripts/secrets.mjs", pattern: /GITLEAKS_VERSION = "([^"]+)"/ },
    mirrors: [{ file: "scripts/secrets.mjs", pattern: /releases\/download\/v([^/]+)\/gitleaks_/g }],
    upstream: { github: "gitleaks/gitleaks" },
    bump: "copy the sha256 of GITLEAKS_ASSETS from the release's checksums.txt; run `just secrets`",
  },
  {
    name: "actionlint",
    pin: { file: "scripts/actionlint.mjs", pattern: /ACTIONLINT_VERSION = "([^"]+)"/ },
    mirrors: [{ file: "scripts/actionlint.mjs", pattern: /actionlint_([\d.]+)_/g }],
    upstream: { github: "rhysd/actionlint" },
    bump: "copy the sha256 of ACTIONLINT_ASSETS from the release's checksums.txt; run `just actionlint`",
  },
  {
    name: "shellcheck",
    pin: { file: "scripts/actionlint.mjs", pattern: /SHELLCHECK_VERSION = "([^"]+)"/ },
    mirrors: [{ file: "scripts/actionlint.mjs", pattern: /shellcheck-v([\d.]+)[./]/g }],
    upstream: { github: "koalaman/shellcheck" },
    bump: "copy the sha256 of SHELLCHECK_ASSETS from the release assets' digest; run `just actionlint`",
  },
  {
    name: "cargo-about",
    pin: { file: "scripts/lib/rust-notices.mjs", pattern: /CARGO_ABOUT_VERSION = "([^"]+)"/ },
    mirrors: [],
    upstream: { crate: "cargo-about" },
    bump: "`just cargo-about`, then `just notices` and read the diff",
  },
  {
    name: "git-cliff",
    pin: { file: "scripts/changelog.mjs", pattern: /GIT_CLIFF_VERSION = "([^"]+)"/ },
    mirrors: [
      { file: "docs/release.md", pattern: /`GIT_CLIFF_VERSION` \(今は ([^)]+)\)/g },
      { file: "docs/release.md", pattern: /cargo install git-cliff --version (\S+) --locked/g },
    ],
    upstream: { crate: "git-cliff" },
    bump: "compare `just changelog` with the previous version",
  },
  {
    name: "cbindgen",
    pin: { file: "scripts/ffi.mjs", pattern: /CBINDGEN_VERSION = "([^"]+)"/ },
    mirrors: [
      { file: ".github/workflows/ci.yml", pattern: /tool: cbindgen@(\S+)/g },
      { file: ".github/workflows/release-checks.yml", pattern: /tool: cbindgen@(\S+)/g },
      { file: ".github/workflows/ffi-release.yml", pattern: /tool: cbindgen@(\S+)/g },
      { file: "justfile", pattern: /cargo install cbindgen --version (\S+) --locked/g },
      {
        file: "crates/ugoe-ffi/README.md",
        pattern: /cargo install cbindgen --version (\S+) --locked/g,
      },
    ],
    upstream: { crate: "cbindgen" },
    bump: "`just ffi-header` and commit the regenerated header; `just ffi-check`",
  },
  {
    name: "cargo-ndk",
    pin: { file: ".github/workflows/unity.yml", pattern: /tool: cargo-ndk@(\S+)/ },
    mirrors: [],
    upstream: { crate: "cargo-ndk" },
    bump: "run unity.yml by hand",
  },
  {
    name: "tauri-driver",
    pin: { file: ".github/workflows/e2e.yml", pattern: /TAURI_DRIVER_VERSION: (\S+)/ },
    mirrors: [
      { file: "docs/testing.md", pattern: /crates\/tauri-driver\) \*\*([^*]+)\*\*/g },
      { file: "docs/testing.md", pattern: /cargo install tauri-driver --version (\S+) --locked/g },
    ],
    upstream: { crate: "tauri-driver" },
    bump: "run `just e2e` (and e2e.yml by hand)",
  },
  {
    name: "artifact-signing-cli",
    pin: {
      file: ".github/workflows/editor-build.yml",
      pattern: /cargo install artifact-signing-cli --version (\S+) --locked/,
    },
    mirrors: [],
    upstream: { crate: "artifact-signing-cli" },
    bump: "build a signed Windows installer with editor-build.yml",
  },
  {
    name: "Godot",
    pin: { file: ".github/workflows/release-checks.yml", pattern: /GODOT_VERSION: (\S+)/ },
    mirrors: [{ file: "bindings/godot/README.md", pattern: /版を固定した Godot \((\S+?)\)/g }],
    upstream: { github: "godotengine/godot" },
    tag: /^(\d+(?:\.\d+)*)-stable$/,
    bump: "copy GODOT_SHA512 from the release's SHA512-SUMS.txt; run the godot job; recheck api-4-x and compatibility_minimum",
  },
  {
    name: "Rust",
    pin: { file: "rust-toolchain.toml", pattern: /^channel = "([^"]+)"$/m },
    mirrors: [],
    upstream: { github: "rust-lang/rust" },
    bump: "fix new clippy lints; `just ci`, `just wasm-check`, `just nostd-check`",
  },
  {
    name: "Node.js",
    pin: { file: ".tool-versions", pattern: /^nodejs (\S+)$/m },
    mirrors: [],
    upstream: { github: "nodejs/node" },
    bump: "`just ci`",
  },
  {
    name: "pnpm",
    pin: { file: "package.json", pattern: /"packageManager": "pnpm@([^"]+)"/ },
    mirrors: [],
    upstream: { github: "pnpm/pnpm" },
    bump: "`corepack use pnpm@<version>`; `pnpm install` and commit the lockfile if it changes",
  },
  {
    name: "just",
    pin: { file: ".tool-versions", pattern: /^just (\S+)$/m },
    mirrors: SETUP_JUST_WORKFLOWS.map((file) => ({ file, pattern: SETUP_JUST_VERSION })),
    upstream: { github: "casey/just" },
    bump: "change `just-version` of every extractions/setup-just in the workflows too; `just ci`",
  },
];

/** 版の文字列を数の並びにする (`v` は外す)。`tag` に合わなければ null。 */
export function parseVersion(text, tag = PLAIN_TAG) {
  const m = tag.exec(text);
  if (!m) return null;
  return m[1].split(".").map(Number);
}

/** 2 つの版 (数の並び) を比べる。足りない桁は 0 と見る (`4.7` と `4.7.0` は同じ)。 */
export function compareVersions(a, b) {
  const n = Math.max(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const d = (a[i] ?? 0) - (b[i] ?? 0);
    if (d !== 0) return Math.sign(d);
  }
  return 0;
}

/**
 * 上流の版の候補 (文字列) から、最も新しい版と、固定した版と同じメジャー版の中で最も新しい版を選ぶ。
 * 形の合わないもの (先行版など) は除く。Node.js のように古い系列の修正が後から出るものがあるので、並びの順は使わない。
 */
export function pickLatest(candidates, pinned, tag = PLAIN_TAG) {
  let latest = null;
  let sameMajor = null;
  for (const text of candidates) {
    const v = parseVersion(text, tag);
    if (!v) continue;
    if (!latest || compareVersions(v, latest.version) > 0) latest = { text, version: v };
    if (v[0] === pinned[0] && (!sameMajor || compareVersions(v, sameMajor.version) > 0)) {
      sameMajor = { text, version: v };
    }
  }
  return { latest: latest?.text ?? null, sameMajor: sameMajor?.text ?? null };
}

/** `pattern` の 1 つ目の組を、`text` の中のすべての一致について返す (`g` 付きの正規表現)。 */
function allMatches(text, pattern) {
  return [...text.matchAll(pattern)].map((m) => m[1]);
}

/**
 * 一覧を読み、道具ごとに固定した版と、写した場所の食い違いを返す。
 * `read` はリポジトリのルートからのパスを受け取って中身を返す (テストで差し替える)。
 * 返す `problems` は、版を読めなかった・写した場所が無い・版が違う場所の説明。
 */
export function readPins(
  tools = PINNED_TOOLS,
  read = (file) => readFileSync(join(root, file), "utf8"),
) {
  const pins = [];
  const problems = [];
  for (const tool of tools) {
    const m = tool.pin.pattern.exec(read(tool.pin.file));
    if (!m) {
      problems.push(`${tool.name}: no version found in ${tool.pin.file}`);
      continue;
    }
    const pinned = m[1];
    if (!parseVersion(pinned, tool.tag)) {
      problems.push(`${tool.name}: unexpected version "${pinned}" in ${tool.pin.file}`);
      continue;
    }
    for (const mirror of tool.mirrors) {
      const found = allMatches(read(mirror.file), mirror.pattern);
      if (found.length === 0) {
        problems.push(
          `${tool.name}: no copy of the version found in ${mirror.file} (${mirror.pattern})`,
        );
      }
      for (const v of found) {
        if (v !== pinned) {
          problems.push(
            `${tool.name}: ${mirror.file} has ${v}, but ${tool.pin.file} pins ${pinned}`,
          );
        }
      }
    }
    pins.push({ tool, pinned });
  }
  return { pins, problems };
}

/** GitHub の Release のタグ (下書きと先行版を除く。新しい順の 100 件)。 */
async function githubReleaseTags(repo, fetchImpl, token) {
  const headers = { Accept: "application/vnd.github+json", "User-Agent": USER_AGENT };
  if (token) headers.Authorization = `Bearer ${token}`;
  const res = await fetchImpl(`https://api.github.com/repos/${repo}/releases?per_page=100`, {
    headers,
  });
  if (!res.ok) throw new Error(`GitHub API ${repo}: HTTP ${res.status}`);
  const releases = await res.json();
  return releases.filter((r) => !r.draft && !r.prerelease).map((r) => r.tag_name);
}

/** crates.io の版 (取り下げ (yanked) を除く)。 */
async function crateVersions(name, fetchImpl) {
  const res = await fetchImpl(`https://crates.io/api/v1/crates/${name}`, {
    headers: { Accept: "application/json", "User-Agent": USER_AGENT },
  });
  if (!res.ok) throw new Error(`crates.io ${name}: HTTP ${res.status}`);
  const body = await res.json();
  return body.versions.filter((v) => !v.yanked).map((v) => v.num);
}

/**
 * 1 つの道具の上流の最新の版を調べ、固定した版と比べる。
 * 返す `status` は `update` (新しい版がある)・`current`・`error` (上流を読めなかった)。
 */
export async function checkTool({ tool, pinned }, { fetchImpl = fetch, token } = {}) {
  const tag = tool.tag ?? PLAIN_TAG;
  const pinnedVersion = parseVersion(pinned, tag);
  try {
    const candidates = tool.upstream.github
      ? await githubReleaseTags(tool.upstream.github, fetchImpl, token)
      : await crateVersions(tool.upstream.crate, fetchImpl);
    const { latest, sameMajor } = pickLatest(candidates, pinnedVersion, tag);
    if (!latest) throw new Error("no released version found upstream");
    const newer = compareVersions(parseVersion(latest, tag), pinnedVersion) > 0;
    return { tool, pinned, latest, sameMajor, status: newer ? "update" : "current" };
  } catch (e) {
    return { tool, pinned, latest: null, sameMajor: null, status: "error", error: e.message };
  }
}

/** 上流の場所 (表に出す)。 */
export function upstreamLabel(upstream) {
  return upstream.github ? `github.com/${upstream.github}` : `crates.io/crates/${upstream.crate}`;
}

/** `--list` の表 (Markdown)。 */
export function listTable(pins) {
  const lines = [
    "| tool | pinned | pinned in | copies | upstream |",
    "| --- | --- | --- | --- | --- |",
  ];
  for (const { tool, pinned } of pins) {
    const copies = [...new Set(tool.mirrors.map((m) => m.file))].join(", ") || "-";
    lines.push(
      `| ${tool.name} | ${pinned} | ${tool.pin.file} | ${copies} | ${upstreamLabel(tool.upstream)} |`,
    );
  }
  return `${lines.join("\n")}\n`;
}

/** 比べた結果の表 (Markdown。ジョブの要約にも書く)。新しい版のあるものを先に並べる。 */
export function resultTable(results) {
  const order = { update: 0, error: 1, current: 2 };
  const sorted = [...results].sort((a, b) => order[a.status] - order[b.status]);
  const updates = results.filter((r) => r.status === "update").length;
  const lines = [
    "### Pinned tool updates",
    "",
    updates === 0
      ? "All pinned tools are at the latest upstream version."
      : `${updates} pinned tool(s) have a newer upstream version. See docs/release.md#固定した道具の版 for how to bump.`,
    "",
    "| tool | pinned | latest | latest in the pinned major | status | pinned in | when bumping |",
    "| --- | --- | --- | --- | --- | --- | --- |",
  ];
  for (const r of sorted) {
    const status = r.status === "error" ? `error: ${r.error}` : r.status;
    const sameMajor = r.sameMajor && r.sameMajor !== r.latest ? r.sameMajor : "-";
    lines.push(
      `| ${r.tool.name} | ${r.pinned} | ${r.latest ?? "?"} | ${sameMajor} | ${status} | ${r.tool.pin.file} | ${r.tool.bump} |`,
    );
  }
  return `${lines.join("\n")}\n`;
}

async function main(args) {
  const list = args.includes("--list");
  const unknown = args.filter((a) => a !== "--list");
  if (unknown.length > 0) {
    console.error("usage: node scripts/tool-updates.mjs [--list]");
    process.exit(2);
  }
  const { pins, problems } = readPins();
  for (const p of problems) console.error(`error: ${p}`);
  if (list) {
    process.stdout.write(listTable(pins));
    process.exit(problems.length > 0 ? 1 : 0);
  }
  const token = process.env.GH_TOKEN || process.env.GITHUB_TOKEN;
  const results = await Promise.all(pins.map((pin) => checkTool(pin, { token })));
  const table = resultTable(results);
  process.stdout.write(table);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, table);
  const annotate = Boolean(process.env.GITHUB_ACTIONS);
  for (const r of results) {
    if (r.status === "update" && annotate) {
      console.log(
        `::notice title=${r.tool.name} ${r.latest}::${r.tool.name} ${r.pinned} is pinned; ${r.latest} is out`,
      );
    }
    if (r.status === "error") {
      const msg = `could not read the upstream of ${r.tool.name}: ${r.error}`;
      console.log(annotate ? `::warning::${msg}` : `warning: ${msg}`);
    }
  }
  process.exit(problems.length > 0 ? 1 : 0);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  await main(process.argv.slice(2));
}
