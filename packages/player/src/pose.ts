// ポーズ (排他的なパーツの切り替え。docs/spec/model.md#ポーズ と docs/spec/motion.md#ポーズ) のグループの読み取りと、
// グループごとに表示する要素の選択 (T0735)。WebPuppet の `pose_json` (着せ替えで付けたモジュールのグループも入る) を読む。

/** ポーズのグループ 1 つ。 */
export interface PoseGroupInfo {
  /** 要素のパーツの ID (グループの順。先頭が最初に表示される)。モジュールのパーツは `<モジュール>/<パーツ>`。 */
  parts: string[];
  /** 着せ替えで付けたモジュールのグループなら、そのモジュールの名前 (先頭の要素の接頭辞)。本体のグループは `null`。 */
  module: string | null;
}

/**
 * `pose_json()` (モデルの `pose` の形: `{fade_duration, groups: [{entries: [{part, links}]}]}`) を読む。要素の無いグループと、
 * `part` が文字列でない要素は飛ばす。`null`・`undefined` (モデルが無い) と、形の違うものは空。
 */
export function parsePose(json: string | null | undefined): PoseGroupInfo[] {
  if (!json) return [];
  const value: unknown = JSON.parse(json);
  const groups = (value as { groups?: unknown } | null)?.groups;
  if (!Array.isArray(groups)) return [];
  return groups.flatMap((g: unknown) => {
    const entries = (g as { entries?: unknown } | null)?.entries;
    if (!Array.isArray(entries)) return [];
    const parts = entries.flatMap((e: unknown) => {
      const part = (e as { part?: unknown } | null)?.part;
      return typeof part === "string" ? [part] : [];
    });
    const first = parts[0];
    if (first === undefined) return [];
    const slash = first.indexOf("/");
    return [{ parts, module: slash < 0 ? null : first.slice(0, slash) }];
  });
}

/**
 * ポーズを読み直したとき (着せ替えの付け外し) の、グループごとに表示している要素。前と要素の並びが同じグループは前の選択を
 * 保ち、ほかは先頭の要素にする (Puppet がポーズを作り直すときの引き継ぎと同じ規則。`PosePlayer::carry_over`)。
 * `shown` は `previousGroups` と同じ順。
 */
export function carryPoseShown(
  groups: readonly PoseGroupInfo[],
  previousGroups: readonly PoseGroupInfo[],
  shown: readonly string[],
): string[] {
  return groups.map((group) => {
    const i = previousGroups.findIndex((p) => sameParts(p.parts, group.parts));
    const kept = i < 0 ? undefined : shown[i];
    return kept !== undefined && group.parts.includes(kept) ? kept : (group.parts[0] ?? "");
  });
}

function sameParts(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((p, i) => p === b[i]);
}

/** 要素の表示名 (モジュールのグループでは `<モジュール>/` を除く)。 */
export function poseEntryLabel(group: PoseGroupInfo, part: string): string {
  const prefix = group.module === null ? null : `${group.module}/`;
  return prefix !== null && part.startsWith(prefix) ? part.slice(prefix.length) : part;
}

/** グループの番号 (1 から。本体のグループと、モジュールごとのグループをそれぞれ数える)。見出しに使う。 */
export function poseGroupNumbers(groups: readonly PoseGroupInfo[]): number[] {
  const counts = new Map<string | null, number>();
  return groups.map(({ module }) => {
    const n = (counts.get(module) ?? 0) + 1;
    counts.set(module, n);
    return n;
  });
}

/**
 * 表示中の要素 (`pose_visible_json()`: `pose_json()` のグループの並びでパーツか `null`。T0478) に `shown` を合わせる
 * (モーションがパーツの不透明度のカーブで要素を切り替えたとき、欄を追従させる。T0736)。グループは要素のパーツで
 * 突き合わせる (`parsePose` が要素の無いグループを飛ばしても並びがずれないように)。表示中の要素が見つからないグループと、
 * 読めないもの (`null`・形の違うもの) は前の選択のまま。何も変わらなければ `shown` そのものを返す。
 */
export function followPoseShown(
  groups: readonly PoseGroupInfo[],
  visibleJson: string | null | undefined,
  shown: readonly string[],
): readonly string[] {
  if (!visibleJson) return shown;
  const value: unknown = JSON.parse(visibleJson);
  if (!Array.isArray(value)) return shown;
  const visible = new Set(value.filter((p): p is string => typeof p === "string"));
  let changed = false;
  const next = groups.map((group, i) => {
    const current = shown[i] ?? group.parts[0] ?? "";
    const part = group.parts.find((p) => visible.has(p));
    if (part === undefined || part === current) return current;
    changed = true;
    return part;
  });
  return changed || next.length !== shown.length ? next : shown;
}
