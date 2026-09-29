// AI の提案の、コマンドの選び方 (一部だけの採用) と表示の補助 (docs/editor.md#ai-の提案)。
// 外したコマンドの番号の集合を `excluded` として持つ。あるコマンドを外すと、それが作る要素を使うコマンド
// (`ProposalCommand.requires` で後から指すもの) も外す。戻すと、それが使う要素を作るコマンドも戻す。
// バックエンドも同じ列を複製で確かめるので、ここの依存が足りなくても誤った採用にはならない (断られる)。
import type { Classification, ProposalCommand } from "../ipc";

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
} {
  const counts = new Map<string, number>();
  for (const l of c.layers) counts.set(l.label, (counts.get(l.label) ?? 0) + 1);
  return {
    layers: c.layers.length,
    review: c.layers.filter((l) => l.needs_review).length,
    labels: [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])),
  };
}
