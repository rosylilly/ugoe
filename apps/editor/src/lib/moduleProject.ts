// モジュールのプロジェクト (ADR 0024、docs/editor.md#モジュールのプロジェクト) の表示の補助。
// 作業用のモデルは本体と合成したもので、本体の要素は読み取り専用の下敷き。
import type { ModuleProblem, ModuleStatus } from "./ipc";
import type { ElementRef } from "./selection.svelte";

/** 本体の要素の集合 (`kind:id`)。本体のモデルのプロジェクトなら空。 */
export function baseElements(module: ModuleStatus | null): Set<string> {
  const set = new Set<string>();
  if (!module) return set;
  for (const id of module.base_parts) set.add(`part:${id}`);
  for (const id of module.base_deformers) set.add(`deformer:${id}`);
  for (const id of module.base_meshes) set.add(`mesh:${id}`);
  return set;
}

/**
 * モジュールの名前を `from` から `to` に変えたときの、選択の付け替え (作業用のモデルの ID の接頭辞が変わる)。
 * 接頭辞の付いていない ID (本体の要素) はそのまま。
 */
export function renamePrefix(refs: readonly ElementRef[], from: string, to: string): ElementRef[] {
  const prefix = `${from}/`;
  return refs.map((r) =>
    r.kind !== "parameter" && r.id.startsWith(prefix)
      ? { ...r, id: `${to}/${r.id.slice(prefix.length)}` }
      : r,
  );
}

/** 要素が本体のもの (モジュールのプロジェクトでは編集できない) か。 */
export function isBaseElement(base: Set<string>, ref: ElementRef): boolean {
  return base.has(`${ref.kind}:${ref.id}`);
}

/** ファイルメニューに出すモジュールの印 (例: `Module: Hair`)。本体は `moduleTitle` で見せる。 */
export function moduleLabel(module: ModuleStatus): string {
  return `Module: ${module.slot.id}`;
}

/** パスの最後の名前 (`/a/avatar.ugoeproj` → `avatar.ugoeproj`)。 */
export function baseFileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** モジュールのプロジェクトでできること (新規作成の画面と `Module:` の印のツールチップ。docs/editor.md#使えるもの・まだ無いもの)。 */
export const MODULE_CAN: readonly string[] = [
  "Add and edit the module's parts, deformers, meshes, glues and textures",
  "Import PSD (draw order goes into the slot's range), auto mesh, mesh editing",
  "Keyforms with standard parameters and the ones the slot exposes; inference and mirroring",
  "Physics rigs and colliders of the module, normal maps of its textures",
  "AI rig proposals for the costume: it follows the base deformers and swaying parts get physics, without new parameters",
  "Move any base parameter with the sliders to check the look",
];

/** モジュールのプロジェクトでできないこと (本体のプロジェクトで直すもの・まだ無いもの)。 */
export const MODULE_CANNOT: readonly string[] = [
  "Change the base: its elements, parameters, avatar profile, slots, effects, lighting and physics settings",
  "Add parameters (add them in the base project and expose them in the slot)",
  "Motions and expressions",
  "Pose groups (there is no command for them yet)",
];

/** モジュールの印の説明 (本体の場所と、できること・できないこと)。 */
export function moduleTitle(module: ModuleStatus): string {
  return [
    `Module "${module.name}" for slot ${module.slot.id} of ${baseFileName(module.base)}`,
    `Read-only base: ${module.base}`,
    "Click for the module settings (name, slot, requirements).",
    "",
    "You can:",
    ...MODULE_CAN.map((line) => `• ${line}`),
    "Edit in the base project instead:",
    ...MODULE_CANNOT.map((line) => `• ${line}`),
  ].join("\n");
}

/** 本体のものを直す場所の案内 (無効にした欄に添える)。 */
export const BASE_EDIT_HINT =
  "This belongs to the base model and can't be edited here. Edit it in the base project.";

/** テクスチャが本体のもの (モジュールのプロジェクトでは変えられない) か。 */
export function isBaseTexture(module: ModuleStatus | null, name: string): boolean {
  return module?.base_textures.includes(name) ?? false;
}

/** 物理のリグが本体のものか。 */
export function isBaseRig(module: ModuleStatus | null, id: string): boolean {
  return module?.base_rigs.includes(id) ?? false;
}

/** 物理の衝突体が本体のものか。 */
export function isBaseCollider(module: ModuleStatus | null, id: string): boolean {
  return module?.base_colliders.includes(id) ?? false;
}

/** モジュールの条件の、省略された欄を埋めたもの (JSON では空の欄を書かない)。 */
export function requirementsOf(module: ModuleStatus): {
  avatar_level: number | null;
  slots: string[];
  parameters: string[];
} {
  const r = module.requires as Partial<ModuleStatus["requires"]>;
  return {
    avatar_level: r.avatar_level ?? null,
    slots: r.slots ?? [],
    parameters: r.parameters ?? [],
  };
}

/** 一覧 `list` に `id` を入れる (`on`) か外す。並びは `order` の順にする。 */
export function toggled(
  list: readonly string[],
  id: string,
  on: boolean,
  order: readonly string[],
): string[] {
  const set = new Set(list);
  if (on) set.add(id);
  else set.delete(id);
  return order.filter((x) => set.has(x));
}

/** モジュールの問題の直し方 (モジュールの問題の枠のボタン)。 */
export type ProblemFix = "locate_project" | "locate_bundle" | "choose_slot" | "open_base";

/** 本体が `.ugoeproj` (エディタで開いて直せる) か。`.ugoe` なら書き出した本体なので開けない。 */
export function baseIsProject(base: string): boolean {
  return !base.toLowerCase().endsWith(".ugoe");
}

/** 本体の問題の、何が起きてどうすればよいかの説明。 */
export function describeModuleProblem(problem: ModuleProblem): string {
  const file = baseFileName(problem.base);
  switch (problem.kind) {
    case "base_not_found":
      return `The base model ${file} is not where this module expects it (${problem.base}). It may have been moved or renamed. Locate it to continue.`;
    case "base_unreadable":
      return `The base model ${file} can't be read. Choose another base (a model project or a .ugoe bundle, not a module project).`;
    case "unknown_slot":
      return `The base model ${file} no longer has slot "${problem.slot}". Choose another slot for this module, or add the slot back in the base project.`;
    case "does_not_fit":
      return `This module no longer fits the base model ${file}. Fix the base (for example expose the parameter or mask in the slot), or choose another base.`;
  }
}

/** 本体の問題の直し方 (ボタンの並び)。 */
export function moduleProblemFixes(problem: ModuleProblem): ProblemFix[] {
  const fixes: ProblemFix[] = [];
  if (problem.kind === "unknown_slot" && problem.base_slots.length > 0) fixes.push("choose_slot");
  if (
    (problem.kind === "unknown_slot" || problem.kind === "does_not_fit") &&
    baseIsProject(problem.base)
  ) {
    fixes.push("open_base");
  }
  fixes.push("locate_project", "locate_bundle");
  return fixes;
}

/** 新しいモジュールの名前の候補: スロットの名前 (モジュールの名前に使えない文字は `_` にする)。 */
export function suggestModuleName(slot: string): string {
  const name = slot.replaceAll("/", "_").replace(/^#+/, "");
  return name.length > 0 ? name : "Module";
}

/** モジュールの名前として使えるか (空でなく、`/` を含まず、`#` で始まらない)。 */
export function isValidModuleName(name: string): boolean {
  const trimmed = name.trim();
  return trimmed.length > 0 && !trimmed.includes("/") && !trimmed.startsWith("#");
}
