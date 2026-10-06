// 監査の結果をまとめる: 同じ種類のもの (検査と要素の種類が同じ) を場面と言語をまたいで 1 つにし、数え、Markdown と JSON にする。
// 前の結果 (summary.json) と比べて、増えたもの・消えたもの・数が変わったものを出す (`--compare`)。

/** 報告に出す検査の順と見出し。 */
export const CHECKS = [
  ["contrast", "文字のコントラスト"],
  ["overflow", "横のはみ出し"],
  ["overlap", "文字の重なり・切れ"],
  ["target-size", "24px 未満の押せるもの"],
  ["name", "名前の無い操作できる部品"],
  ["focus", "Tab の止まりの問題 (印・名前・画面の外)"],
  ["aria", "そのほかの ARIA・言語 (axe-core)"],
  ["structure", "ランドマークと見出しの階層 (axe-core)"],
  ["motion", "reduced motion の下の動き"],
  ["forced", "強制色で見分けられない状態"],
  ["error", "エラー (画面に出たもの・スクリプト・場面の失敗)"],
];

/**
 * 要素の種類: `tag.class "名前"`・`tag@.class "名前"` の形 (page-checks.js の describe) なら `tag.class` (同じ部品の行を 1 つにまとめ、
 * 言語で名前が変わっても同じものにする)。クラスが無ければ名前まで含めた全体。axe の CSS セレクタなどもそのまま。
 */
export function kindOf(target) {
  const m = /^([a-z0-9-]+(?:\[[a-z]+\])?)(@?(?:\.[\w-]+)+) "/.exec(target);
  return m ? `${m[1]}${m[2]}` : target;
}

const keyOf = (f) => `${f.check}\u0000${f.kind}`;

/**
 * 1 つのアプリの場面の列 ({name, findings}) をまとめる。
 * 戻り値: {scenes, counts: {check: {unique, total}}, findings: [{check, kind, targets, detail, scenes}]}
 * (unique は種類の数、total は延べ (場面ごとに数えた数))
 */
export function summarize(scenes) {
  const merged = new Map();
  const counts = Object.fromEntries(CHECKS.map(([c]) => [c, { unique: 0, total: 0 }]));
  for (const scene of scenes) {
    for (const f of scene.findings) {
      counts[f.check] ??= { unique: 0, total: 0 };
      counts[f.check].total++;
      const kind = kindOf(f.target);
      const key = keyOf({ check: f.check, kind });
      let m = merged.get(key);
      if (!m) {
        m = { check: f.check, kind, targets: [], detail: f.detail, scenes: [] };
        merged.set(key, m);
      }
      if (!m.targets.includes(f.target)) m.targets.push(f.target);
      if (!m.scenes.includes(scene.name)) m.scenes.push(scene.name);
    }
  }
  const findings = [...merged.values()];
  for (const f of findings) counts[f.check].unique++;
  const order = Object.fromEntries(CHECKS.map(([c], i) => [c, i]));
  findings.sort(
    (a, b) => (order[a.check] ?? 99) - (order[b.check] ?? 99) || a.kind.localeCompare(b.kind),
  );
  return { scenes: scenes.length, counts, findings };
}

/** アプリごとのまとめ ({app: summarize の結果}) を、検査ごとの数の表 (Markdown) に。数は「種類 (延べ)」。 */
export function countsTable(apps) {
  const names = Object.keys(apps);
  const rows = [`| 検査 | ${names.join(" | ")} |`, `| --- |${names.map(() => " --- |").join("")}`];
  for (const [check, title] of CHECKS) {
    const cells = names.map((a) => {
      const c = apps[a].counts[check] ?? { unique: 0, total: 0 };
      return c.unique === 0 ? "0" : `${c.unique} (${c.total})`;
    });
    rows.push(`| ${title} | ${cells.join(" | ")} |`);
  }
  rows.push(`| 場面の数 | ${names.map((a) => apps[a].scenes).join(" | ")} |`);
  return rows.join("\n");
}

function listWithRest(items, n, sep = ", ") {
  return items.length > n
    ? `${items.slice(0, n).join(sep)} ほか ${items.length - n}`
    : items.join(sep);
}

/** 1 つのアプリの報告 (Markdown)。`extra` は末尾に足す節 (Tab の順など)。 */
export function appReport(app, summary, extra = "") {
  const lines = [
    `# ${app} の UI の監査`,
    "",
    "数は「種類 (延べ)」。種類はクラスの付いた要素なら `tag.class` (同じ部品の行をまとめる)、無ければ名前まで含めた要素。",
    "",
    countsTable({ [app]: summary }),
    "",
  ];
  for (const [check, title] of CHECKS) {
    const items = summary.findings.filter((f) => f.check === check);
    if (items.length === 0) continue;
    lines.push(`## ${title}`, "");
    for (const f of items) {
      const many =
        f.targets.length > 1
          ? ` (${f.targets.length} 個: ${listWithRest(
              f.targets.map((t) => `\`${t}\``),
              3,
            )})`
          : "";
      const head = f.targets.length > 1 ? `\`${f.kind}\`${many}` : `\`${f.targets[0]}\``;
      lines.push(`- ${head} ${f.detail} — ${listWithRest(f.scenes, 3)}`);
    }
    lines.push("");
  }
  if (extra) lines.push(extra);
  return `${lines.join("\n")}\n`;
}

/**
 * 前の結果と比べる。`before`・`after` は summary.json の `apps`。
 * 戻り値: {app: {added, removed, changed: [{finding, before}]}} (両方にあるアプリだけ。changed は要素の数が変わった種類)
 */
export function compare(before, after) {
  const out = {};
  for (const app of Object.keys(after)) {
    if (!before[app]) continue;
    const old = new Map(before[app].findings.map((f) => [keyOf(f), f]));
    const now = new Map(after[app].findings.map((f) => [keyOf(f), f]));
    out[app] = {
      added: [...now.entries()].filter(([k]) => !old.has(k)).map(([, f]) => f),
      removed: [...old.entries()].filter(([k]) => !now.has(k)).map(([, f]) => f),
      changed: [...now.entries()]
        .filter(([k, f]) => old.has(k) && old.get(k).targets.length !== f.targets.length)
        .map(([k, f]) => ({ finding: f, before: old.get(k).targets.length })),
    };
  }
  return out;
}

/** compare の結果を文に。 */
export function formatCompare(diff) {
  const lines = [];
  for (const [app, { added, removed, changed }] of Object.entries(diff)) {
    lines.push(`${app}: +${added.length} -${removed.length} ~${changed.length}`);
    for (const f of added) lines.push(`  + [${f.check}] ${f.kind} ${f.detail}`);
    for (const f of removed) lines.push(`  - [${f.check}] ${f.kind} ${f.detail}`);
    for (const { finding: f, before } of changed)
      lines.push(`  ~ [${f.check}] ${f.kind} ${before} -> ${f.targets.length}`);
  }
  return lines.join("\n");
}
