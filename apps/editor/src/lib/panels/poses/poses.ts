// ポーズの編集 (docs/editor.md#ポーズの編集) の補助。DOM や IPC に依存しない純粋な関数だけを置く (poses.test.ts で検査する)。
// 編集はポーズを丸ごと置き換える `set_pose` を送るだけで、値の検査 (参照するパーツ・同じパーツの重複・
// フェードの時間) は適用後の検証が、モジュールのプロジェクトの本体の部分の検査は editor-core が行う。
import type { Command, ModuleStatus, Pose, PoseGroup } from "../../ipc";

/** ポーズを丸ごと置き換えるコマンド。 */
export function setPoseCommand(pose: Pose): Command {
  return { op: "set_pose", pose };
}

/** ポーズの要素とリンクに使っているパーツ (同じパーツは 1 度しか使えない)。 */
export function usedParts(pose: Pose): Set<string> {
  const used = new Set<string>();
  for (const group of pose.groups) {
    for (const entry of group.entries) {
      used.add(entry.part);
      for (const link of entry.links) used.add(link);
    }
  }
  return used;
}

/** モジュールのプロジェクトで、`index` 番目のグループが本体のもの (読み取り専用) か。 */
export function isBaseGroup(module: ModuleStatus | null, index: number): boolean {
  return module !== null && index < module.base_pose_groups;
}

/** パーツが本体のもの (モジュールのプロジェクトでは、ポーズに入れたり外したりできない) か。 */
export function isBasePart(module: ModuleStatus | null, part: string): boolean {
  return module?.base_parts.includes(part) ?? false;
}

/**
 * 本体のグループの要素 `part` の `links` にモジュールのパーツを加えられるか (スロットの `pose_parts` にある要素だけ。
 * docs/spec/modules.md#ポーズ)。本体のプロジェクトではいつも加えられる。
 */
export function canLinkTo(module: ModuleStatus | null, part: string): boolean {
  return module === null || module.slot.pose_parts.includes(part);
}

/** 要素やリンクに選べるパーツ: ポーズでまだ使っていないもの (モジュールのプロジェクトではモジュールのパーツだけ)。 */
export function freeParts<T extends { id: string }>(
  parts: readonly T[],
  pose: Pose,
  module: ModuleStatus | null,
): T[] {
  const used = usedParts(pose);
  return parts.filter((p) => !used.has(p.id) && !isBasePart(module, p.id));
}

function mapGroup(pose: Pose, index: number, f: (group: PoseGroup) => PoseGroup | null): Pose {
  const groups = pose.groups.flatMap((g, i) => {
    if (i !== index) return [g];
    const next = f(g);
    return next && next.entries.length > 0 ? [next] : [];
  });
  return { ...pose, groups };
}

/** パーツ `part` を要素 1 つにした新しいグループを末尾に足す。 */
export function addGroup(pose: Pose, part: string): Pose {
  return { ...pose, groups: [...pose.groups, { entries: [{ part, links: [] }] }] };
}

/** グループを消す。 */
export function removeGroup(pose: Pose, index: number): Pose {
  return mapGroup(pose, index, () => null);
}

/** グループの末尾に要素を足す。 */
export function addEntry(pose: Pose, group: number, part: string): Pose {
  return mapGroup(pose, group, (g) => ({ entries: [...g.entries, { part, links: [] }] }));
}

/** 要素を消す (空になったグループも消す)。 */
export function removeEntry(pose: Pose, group: number, entry: number): Pose {
  return mapGroup(pose, group, (g) => ({ entries: g.entries.filter((_, i) => i !== entry) }));
}

/** 要素を `delta` だけ動かす (先頭の要素が最初に表示される)。動かせなければそのまま。 */
export function moveEntry(pose: Pose, group: number, entry: number, delta: -1 | 1): Pose {
  return mapGroup(pose, group, (g) => {
    const to = entry + delta;
    const a = g.entries[entry];
    const b = g.entries[to];
    if (!a || !b) return g;
    const entries = [...g.entries];
    entries[entry] = b;
    entries[to] = a;
    return { entries };
  });
}

/**
 * 要素を `from` から `to` の位置へ動かす (ドラッグでの並べ替え。`to` は動かす前の並びでの落とした先の要素の番号で、
 * その要素の前に入れる。`to` が要素の数なら末尾)。動かせなければそのまま。
 */
export function moveEntryTo(pose: Pose, group: number, from: number, to: number): Pose {
  return mapGroup(pose, group, (g) => {
    const moved = g.entries[from];
    if (!moved || to < 0 || to > g.entries.length || to === from || to === from + 1) return g;
    const entries = g.entries.filter((_, i) => i !== from);
    entries.splice(to > from ? to - 1 : to, 0, moved);
    return { entries };
  });
}

/**
 * ビューポートでポーズを見ているとき (`shown` は表示する要素のパーツ。ViewportController.posePreview)、
 * グループで表示される要素の番号 (挙げたパーツがなければ最初の要素)。
 */
export function shownEntry(group: PoseGroup, shown: readonly string[]): number {
  const i = group.entries.findIndex((e) => shown.includes(e.part));
  return i < 0 ? 0 : i;
}

/** グループ `group` で `part` を表示するよう、表示する要素のパーツの一覧を作り直す (同じグループのほかの要素を外す)。 */
export function showEntry(shown: readonly string[], group: PoseGroup, part: string): string[] {
  const inGroup = new Set(group.entries.map((e) => e.part));
  return [...shown.filter((p) => !inGroup.has(p)), part];
}

/**
 * 要素の `links` を `picked` にする。モジュールのプロジェクトでは、選べるのはモジュールのパーツだけなので、
 * 本体のリンク (本体のプロジェクトで決めたもの) はそのまま残す。
 */
export function setLinks(
  pose: Pose,
  group: number,
  entry: number,
  picked: readonly string[],
  module: ModuleStatus | null,
): Pose {
  return mapGroup(pose, group, (g) => ({
    entries: g.entries.map((e, i) =>
      i === entry
        ? { ...e, links: [...e.links.filter((l) => isBasePart(module, l)), ...picked] }
        : e,
    ),
  }));
}

/** 要素のリンクのうち、この画面で選び直せるもの (モジュールのプロジェクトではモジュールのパーツ)。 */
export function editableLinks(links: readonly string[], module: ModuleStatus | null): string[] {
  return links.filter((l) => !isBasePart(module, l));
}

/** クロスフェードの時間 (秒)。0 以上の有限の値でなければ null。 */
export function fadeSeconds(text: string): number | null {
  if (text.trim() === "") return null;
  const value = Number(text);
  return Number.isFinite(value) && value >= 0 ? value : null;
}
