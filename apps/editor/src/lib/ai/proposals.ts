// AI の提案の、コマンドの選び方 (一部だけの採用) と表示の補助 (docs/editor.md#ai-の提案)。
// 外したコマンドの番号の集合を `excluded` として持つ。あるコマンドを外すと、それが作る要素を使うコマンド
// (`ProposalCommand.requires` で後から指すもの) も外す。戻すと、それが使う要素を作るコマンドも戻す。
// バックエンドも同じ列を複製で確かめるので、ここの依存が足りなくても誤った採用にはならない (断られる)。

import { t } from "../i18n";
import type {
  Classification,
  CommandSection,
  NormalMapGroupChoice,
  NormalMapGroupView,
  ProposalCommand,
} from "../ipc";
import { type BulgeForm, bulgeForm, bulgeSettings } from "../panels/lighting/lighting";

/** `index` を外した集合。`index` を (直接・間接に) 使うコマンドも外す。 */
export function excludeCommand(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  index: number,
): Set<number> {
  const next = new Set(excluded);
  const stack = [index];
  while (stack.length > 0) {
    const i = stack.pop() as number;
    if (next.has(i)) continue;
    next.add(i);
    commands.forEach((c, k) => {
      if (c.requires.includes(i) && !next.has(k)) stack.push(k);
    });
  }
  return next;
}

/** `index` を戻した集合。`index` が (直接・間接に) 使う要素を作るコマンドも戻す。 */
export function includeCommand(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  index: number,
): Set<number> {
  const next = new Set(excluded);
  const stack = [index];
  while (stack.length > 0) {
    const i = stack.pop() as number;
    next.delete(i);
    for (const r of commands[i]?.requires ?? []) {
      if (next.has(r)) stack.push(r);
    }
  }
  return next;
}

/** チェックボックスの切り替え。 */
export function toggleCommand(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  index: number,
): Set<number> {
  return excluded.has(index)
    ? includeCommand(commands, excluded, index)
    : excludeCommand(commands, excluded, index);
}

/** 同じ対象への続いたコマンドのまとまり (一覧はまとまりごとに出す)。 */
export interface CommandGroup {
  /** 対象 (`ProposalCommand.subject`)。なければ最初のコマンドの種類。 */
  label: string;
  indices: number[];
  /** 提案の区切り (`ProposalState.sections`) から作ったか。1 つだけでも名前を付けて出し、中のコマンドは対象 (`subject`) 付きで出す。 */
  section?: boolean;
}

/** 続いたコマンドを対象ごとにまとめる (順序は変えない)。 */
export function groupCommands(commands: ProposalCommand[]): CommandGroup[] {
  const groups: CommandGroup[] = [];
  let lastSubject: string | null | undefined;
  commands.forEach((c, i) => {
    const last = groups.at(-1);
    if (last && c.subject !== null && c.subject === lastSubject) last.indices.push(i);
    else groups.push({ label: c.subject ?? c.op, indices: [i] });
    lastSubject = c.subject;
  });
  return groups;
}

/** まとまりの選び方: すべて選んでいる・一部だけ・何も選んでいない。 */
export function groupState(
  group: CommandGroup,
  excluded: ReadonlySet<number>,
): "all" | "some" | "none" {
  const out = group.indices.filter((i) => excluded.has(i)).length;
  return out === 0 ? "all" : out === group.indices.length ? "none" : "some";
}

/** まとまりの切り替え: 1 つでも選んでいればすべて外し (依存するものも)、何も選んでいなければすべて戻す。 */
export function toggleGroup(
  commands: ProposalCommand[],
  excluded: ReadonlySet<number>,
  group: CommandGroup,
): Set<number> {
  const include = groupState(group, excluded) === "none";
  let next = new Set(excluded);
  for (const i of group.indices) {
    next = include ? includeCommand(commands, next, i) : excludeCommand(commands, next, i);
  }
  return next;
}

/** 集合を IPC に渡す並び (昇順)。 */
export function excludedList(excluded: ReadonlySet<number>): number[] {
  return [...excluded].sort((a, b) => a - b);
}

/** 一覧に出すコマンドの 1 行 (`add_parameter AngleX`)。 */
export function commandLabel(command: ProposalCommand): string {
  return command.subject ? `${command.op} ${command.subject}` : command.op;
}

/** 分類の要約 (レイヤー数・確かめたほうがよいレイヤー数・ラベルごとの数)。 */
export function classificationSummary(c: Classification): {
  layers: number;
  review: number;
  labels: [string, number][];
  /** AI が同じ部品の別の絵 (差分) とみなしたレイヤーの組 (レイヤーの名前。組の番号の順)。 */
  variants: string[][];
} {
  const counts = new Map<string, number>();
  const variants = new Map<number, string[]>();
  for (const l of c.layers) {
    counts.set(l.label, (counts.get(l.label) ?? 0) + 1);
    if (l.variant !== null) {
      const names = variants.get(l.variant) ?? [];
      names.push(l.path[l.path.length - 1] ?? "");
      variants.set(l.variant, names);
    }
  }
  return {
    layers: c.layers.length,
    review: c.layers.filter((l) => l.needs_review).length,
    labels: [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])),
    variants: [...variants].sort((a, b) => a[0] - b[0]).map(([, names]) => names),
  };
}

/** 注文の文字数の上限 (`ugoe_ai::instructions::Instructions::MAX_CHARS`)。 */
export const MAX_INSTRUCTION_CHARS = 2000;

/** 注文の文字数 (Unicode のスカラー値の数。バックエンドの数え方と同じ)。前後の空白は数えない。 */
export function instructionChars(text: string): number {
  return [...text.trim()].length;
}

/**
 * AI に添える注文 (docs/editor.md#ai-への注文と作り直し)。AI を使わないとき・空のときは送らない (`null`)。
 * 長すぎるかどうかはバックエンドが確かめる。
 */
export function instructionsToSend(text: string, useAi: boolean): string | null {
  const trimmed = text.trim();
  return useAi && trimmed !== "" ? trimmed : null;
}

/** 区切り (法線マップの提案のまとまり。`ProposalState.sections`) を、一覧のまとまりにする。 */
export function sectionGroups(sections: CommandSection[]): CommandGroup[] {
  return sections.map((s) => ({
    label: surfaceGroupLabel(s.label),
    indices: [...s.indices],
    section: true,
  }));
}

// ---- 法線マップの提案 (docs/editor.md#法線マップの提案) ----

/** ラベルのないテクスチャのまとまりの名前 (`NormalMapGroupView.group` が null)。 */
export const UNLABELED = "unlabeled";

/** まとまりの画面の名前 (呼ぶたびに今の言語で引く)。 */
const SURFACE_GROUP_LABELS: Record<string, () => string> = {
  face: () => t("proposals.groups.face"),
  body: () => t("proposals.groups.body"),
  hair_front: () => t("proposals.groups.hairFront"),
  hair_side: () => t("proposals.groups.hairSide"),
  hair_back: () => t("proposals.groups.hairBack"),
  ear: () => t("proposals.groups.ear"),
  earwear: () => t("proposals.groups.earwear"),
  headwear: () => t("proposals.groups.headwear"),
  tail: () => t("proposals.groups.tail"),
  alone: () => t("proposals.groups.alone"),
  [UNLABELED]: () => t("proposals.groups.unlabeled"),
  lighting: () => t("proposals.groups.lighting"),
};

/** まとまり (`face` など) の画面の名前。知らない名前はそのまま。 */
export function surfaceGroupLabel(group: string): string {
  return SURFACE_GROUP_LABELS[group]?.() ?? group;
}

/** まとまりの鍵 (`group`、ラベルがなければ `unlabeled`)。 */
export function normalGroupKey(group: NormalMapGroupView): string {
  return group.group ?? UNLABELED;
}

/** まとまりごとの選び方: 含めるか・強さ・縁の丸み・ならし (長さは画面の単位。panels/lighting/lighting.ts の `BulgeForm`)。 */
export interface NormalMapPick extends BulgeForm {
  include: boolean;
}

/**
 * まとまりの一覧に合わせた選び方。前の選び方 (`previous`) にあるまとまりはそれを残し、新しいまとまりは含めてまとまりの既定の設定にする。
 */
export function normalMapPicks(
  groups: NormalMapGroupView[],
  previous: Readonly<Record<string, NormalMapPick>> = {},
): Record<string, NormalMapPick> {
  const picks: Record<string, NormalMapPick> = {};
  for (const g of groups) {
    const key = normalGroupKey(g);
    picks[key] = previous[key] ?? { include: true, ...bulgeForm(g) };
  }
  return picks;
}

/** IPC に渡す、含めるまとまりと設定 (長さはまとまりの格子の画素に直す)。 */
export function normalMapChoices(
  groups: NormalMapGroupView[],
  picks: Readonly<Record<string, NormalMapPick>>,
): NormalMapGroupChoice[] {
  return groups.flatMap((g) => {
    const pick = picks[normalGroupKey(g)];
    if (!pick?.include) return [];
    return [{ group: g.group, settings: bulgeSettings(pick, g) }];
  });
}

/** 提案で法線マップを作るテクスチャの数 (`replaceExisting` でなければ、法線マップを持つものを除く)。 */
export function normalMapTargetCount(
  groups: NormalMapGroupView[],
  picks: Readonly<Record<string, NormalMapPick>>,
  replaceExisting: boolean,
): number {
  return groups
    .filter((g) => picks[normalGroupKey(g)]?.include)
    .reduce((n, g) => n + g.textures.length - (replaceExisting ? 0 : g.with_normal_map.length), 0);
}
