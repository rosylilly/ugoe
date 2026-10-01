// モジュールのプロジェクト (ADR 0024、docs/editor.md#モジュールのプロジェクト) の表示の補助。
// 作業用のモデルは本体と合成したもので、本体の要素は読み取り専用の下敷き。
import type { ColliderReference, ModuleProblem, ModuleStatus } from "./ipc";
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
  "AI rig proposals for the costume: it follows the base deformers and meshes, swaying parts get physics that keeps out of the base's exposed colliders, and A / B variants become poses, without new parameters",
  "Pose groups of the module's parts, and links to the base's pose entries that the slot exposes (Poses tab)",
  "Move any base parameter with the sliders to check the look",
  "Preview your current edits on the base and swap them with other modules of the slot",
];

/** モジュールのプロジェクトでできないこと (本体のプロジェクトで直すもの・まだ無いもの)。 */
export const MODULE_CANNOT: readonly string[] = [
  "Change the base: its elements, parameters, avatar profile, slots, effects, lighting and physics settings",
  "Add parameters (add them in the base project and expose them in the slot)",
  "Motions and expressions",
  "The base's pose groups and the pose fade time",
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

/** メッシュが本体のものか。 */
export function isBaseMesh(module: ModuleStatus | null, id: string): boolean {
  return module?.base_meshes.includes(id) ?? false;
}

/**
 * 物理のリグの列 `rigs` の `index` 番目を `delta` だけ動かせるか。モジュールのプロジェクトでは、本体のリグの順は
 * 変えられず、モジュールのリグは本体のリグの後に続く (合成の順) ので、モジュールのリグどうしでだけ入れ替える。
 */
export function canMoveRig(
  module: ModuleStatus | null,
  rigs: readonly string[],
  index: number,
  delta: -1 | 1,
): boolean {
  const id = rigs[index];
  const other = rigs[index + delta];
  if (id === undefined || other === undefined) return false;
  return !isBaseRig(module, id) && !isBaseRig(module, other);
}

/** 物理の衝突体が本体のものか。 */
export function isBaseCollider(module: ModuleStatus | null, id: string): boolean {
  return module?.base_colliders.includes(id) ?? false;
}

/**
 * スロットが公開する本体の衝突体が本体に無い (本体に物理が無い) ときの、物理の画面の案内。無ければ `null`。
 * モジュールのリグはその衝突体に当たれず、参照すると開けない・書き出せない (T0660)。
 */
export function unavailableCollidersNotice(module: ModuleStatus | null): string | null {
  const missing = module?.unavailable_colliders ?? [];
  if (!module || missing.length === 0) return null;
  const names = missing.map((id) => `"${id}"`).join(", ");
  const them = missing.length === 1 ? "it" : "them";
  return `Slot "${module.slot.id}" exposes the base collider${missing.length === 1 ? "" : "s"} ${names}, but the base model has no physics, so this module's rigs can't collide with ${them}. To use ${them}, open the base project and add physics with ${them} in the Physics tab, then reload the base.`;
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
export type ProblemFix =
  | "locate_project"
  | "locate_bundle"
  | "choose_slot"
  | "open_base"
  | "drop_colliders";

/** 本体が `.ugoeproj` (エディタで開いて直せる) か。`.ugoe` なら書き出した本体なので開けない。 */
export function baseIsProject(base: string): boolean {
  return !base.toLowerCase().endsWith(".ugoe");
}

/**
 * 本体の問題の、何が起きてどうすればよいかの説明。ファイル名は文と続けて読めないよう引用符で囲む
 * (`The base model "moved.ugoeproj" no longer has…`)。
 */
export function describeModuleProblem(problem: ModuleProblem): string {
  const file = `"${baseFileName(problem.base)}"`;
  switch (problem.kind) {
    case "base_not_found":
      return `The base model ${file} is not where this module expects it (${problem.base}). It may have been moved or renamed. Locate it to continue.`;
    case "base_unreadable":
      return `The base model ${file} can't be read. Choose another base (a model project or a .ugoe bundle, not a module project).`;
    case "unknown_slot":
      return `The base model ${file} no longer has slot "${problem.slot}". Choose another slot for this module, or add the slot back in the base project.`;
    case "does_not_fit":
      return `This module no longer fits the base model ${file}. Fix the base (for example expose the parameter or mask in the slot), or choose another base.`;
    case "base_needs_physics":
      return `A physics rig of this module collides with a collider that slot "${problem.slot}" exposes, but the base model ${file} has no physics, so the collider doesn't exist. Open the base project and add physics with that collider (Physics tab), or choose another base.`;
  }
}

/** 本体の問題の直し方 (ボタンの並び)。 */
export function moduleProblemFixes(problem: ModuleProblem): ProblemFix[] {
  const fixes: ProblemFix[] = [];
  if (problem.kind === "unknown_slot" && problem.base_slots.length > 0) fixes.push("choose_slot");
  if (
    (problem.kind === "unknown_slot" ||
      problem.kind === "does_not_fit" ||
      problem.kind === "base_needs_physics") &&
    baseIsProject(problem.base)
  ) {
    fixes.push("open_base");
  }
  // 外せば開けるときだけ (T0661)。外すことは確かめてから (dropCollidersPrompt)。
  if (problem.kind === "base_needs_physics" && problem.collider_references.length > 0) {
    fixes.push("drop_colliders");
  }
  fixes.push("locate_project", "locate_bundle");
  return fixes;
}

/**
 * 衝突体の参照を外して開く前に確かめる文 (T0661。T0655 の「黙って消さない」)。何を外し、いつファイルに書かれ、
 * どうすれば戻せるかを言う。
 */
export function dropCollidersPrompt(references: readonly ColliderReference[]): string {
  const list = references.map((r) => `${r.rig} → ${r.collider}`).join(", ");
  const one = references.length === 1;
  const what = one ? "this collider reference" : `these ${references.length} collider references`;
  const rigs = one
    ? "The rig will no longer collide with it"
    : "The rigs will no longer collide with them";
  const colliders = one ? "the collider" : "the colliders";
  return `Open the module without ${what}: ${list}. ${rigs}. The module file changes only when you save; add physics with ${colliders} to the base to use ${one ? "it" : "them"} again.`;
}

/** 衝突体の参照を外して開いた後の知らせ。 */
export function droppedCollidersNotice(references: readonly ColliderReference[]): string {
  const n = references.length;
  const list = references.map((r) => `${r.rig} → ${r.collider}`).join(", ");
  return `Opened without ${n} collider reference${n === 1 ? "" : "s"} (${list}). Save to keep the change, or close without saving to keep ${n === 1 ? "it" : "them"} in the file.`;
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
