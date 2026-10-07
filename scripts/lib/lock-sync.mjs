// ワークスペースの外のクレート (fuzz/・bindings/godot/) の Cargo.lock が、ルートの Cargo.lock と依存の版をそろえているかの判定。
// fuzz が回すコードや Godot に出すコードと、出荷するコード (ルートのワークスペース) で依存の版が違わないようにする
// (docs/testing.md#fuzz)。scripts/check-lock-sync.mjs と fuzz/run.mjs が使う。

/** Cargo.lock の中身から、crates.io などの外から来るパッケージ (`source` を持つもの) の `{ name, version }` の一覧。 */
export function parseLock(text) {
  const packages = [];
  for (const block of text.split(/^\[\[package\]\]$/m).slice(1)) {
    const field = (key) => new RegExp(`^${key} = "([^"]*)"$`, "m").exec(block)?.[1];
    const name = field("name");
    const version = field("version");
    // source の無いもの (ワークスペースのメンバーと path の依存) は比べない
    if (name !== undefined && version !== undefined && field("source") !== undefined) {
      packages.push({ name, version });
    }
  }
  return packages;
}

/**
 * semver で互換な版の系列 (cargo が 1 つの lock に 1 つしか入れない単位)。
 * 1.2.3 → "1"、0.33.11 → "0.33"、0.0.4 → "0.0.4"。
 */
export function compatLine(version) {
  const [major, minor, patch] = version.split(/[.+-]/);
  if (major !== "0") return major;
  if (minor !== "0") return `0.${minor}`;
  return `0.0.${patch}`;
}

/**
 * `other` (外のクレートの lock) のパッケージのうち、ルートの lock に同じ名前で互換な系列の別の版があるもの。
 * ルートに無いパッケージや、互換でない系列 (godot-rust の glam 0.32 とルートの glam 0.33 など) は比べない。
 * 返す要素は `{ name, version, rootVersion }`。
 */
export function lockDrift(rootText, otherText) {
  const rootVersions = new Map();
  for (const { name, version } of parseLock(rootText)) {
    rootVersions.set(`${name}@${compatLine(version)}`, version);
  }
  const drift = [];
  for (const { name, version } of parseLock(otherText)) {
    const rootVersion = rootVersions.get(`${name}@${compatLine(version)}`);
    if (rootVersion !== undefined && rootVersion !== version) {
      drift.push({ name, version, rootVersion });
    }
  }
  return drift.sort((a, b) => a.name.localeCompare(b.name));
}

/** ずれを伝える短い文 (複数行)。`lock` と `manifest` はリポジトリのルートからの相対。 */
export function lockDriftMessage(lock, manifest, drift) {
  return [
    `error: ${lock} の依存の版がルートの Cargo.lock と違う`,
    ...drift.map((d) => `  ${d.name} ${d.version} (ルートは ${d.rootVersion})`),
    "直し方: ルートの版に合わせる。",
    ...drift.map(
      (d) =>
        `  cargo update --manifest-path ${manifest} -p ${d.name}@${d.version} --precise ${d.rootVersion}`,
    ),
    "  合わせられない (外のクレートの依存が新しい版を求める) ときは、ルートで `cargo update -p <名前>` して両方を新しい版にする。",
  ].join("\n");
}
