// モジュールのプロジェクト (ADR 0024、docs/editor.md#モジュールのプロジェクト) の表示の補助。
// 作業用のモデルは本体と合成したもので、本体の要素は読み取り専用の下敷き。
import { formatList } from "@ugoe/i18n";
import { isMessage } from "./errors";
import { t } from "./i18n";
import type { ColliderReference, ModuleProblem, ModuleStatus, SlotSummary } from "./ipc";
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
  return t("module.badge.label", { slot: module.slot.id });
}

/** パスの最後の名前 (`/a/avatar.ugoeproj` → `avatar.ugoeproj`)。 */
export function baseFileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** モジュールのプロジェクトでできること (新規作成の画面。docs/editor.md#使えるもの・まだ無いもの)。 */
export function moduleCan(): string[] {
  return [
    t("module.can.elements"),
    t("module.can.psd"),
    t("module.can.keyforms"),
    t("module.can.physics"),
    t("module.can.ai"),
    t("module.can.poses"),
    t("module.can.parameters"),
    t("module.can.preview"),
  ];
}

/** モジュールのプロジェクトでできないこと (本体のプロジェクトで直すもの・まだ無いもの)。 */
export function moduleCannot(): string[] {
  return [
    t("module.cannot.base"),
    t("module.cannot.parameters"),
    t("module.cannot.motions"),
    t("module.cannot.poses"),
  ];
}

/** モジュールの印の説明 (名前・スロット・本体の場所と、押すと設定が開くこと)。 */
export function moduleTitle(module: ModuleStatus): string {
  return t("module.badge.tooltip", {
    name: module.name,
    slot: module.slot.id,
    file: baseFileName(module.base),
    base: module.base,
  });
}

/** 取り付け先の名前 (`null` はキャンバス)。 */
export function anchorName(anchor: string | null): string {
  return anchor ?? t("module.slot.canvas");
}

/** スロットの取り付け先・描画順の範囲・隠す本体のパーツの短い説明 (スロットを選ぶ欄)。 */
export function slotSummary(slot: SlotSummary): string {
  const summary = t("module.slot.summary", {
    anchor: anchorName(slot.anchor),
    min: slot.draw_order_min,
    max: slot.draw_order_max,
  });
  return slot.replaces.length > 0
    ? t("module.slot.summaryReplaces", { summary, parts: formatList(slot.replaces) })
    : summary;
}

/** 本体のものを直す場所の案内 (無効にした欄に添える)。 */
export function baseEditHint(): string {
  return t("module.baseEditHint");
}

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
  return t("module.unavailableColliders", {
    slot: module.slot.id,
    colliders: names,
    count: missing.length,
  });
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
  const file = baseFileName(problem.base);
  switch (problem.kind) {
    case "base_not_found":
      return t("moduleProblem.kind.baseNotFound", { file, path: problem.base });
    case "base_unreadable":
      return t("moduleProblem.kind.baseUnreadable", { file });
    case "unknown_slot":
      return t("moduleProblem.kind.unknownSlot", { file, slot: problem.slot });
    case "does_not_fit":
      return t("moduleProblem.kind.doesNotFit", { file });
    case "base_needs_physics":
      return t("moduleProblem.kind.baseNeedsPhysics", { file, slot: problem.slot });
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

/** モジュールのプロジェクトで適用を拒否されたときの直し方の案内 (文と、本体のプロジェクトを開けるか)。 */
export interface ModuleErrorHelp {
  hint: string;
  /** 「Open base project」を出す (本体が `.ugoeproj` のときだけ)。 */
  openBase: boolean;
}

/** 合成のエラーの原因 (`compose.*`) のうち、スロットが公開していないもの。 */
const NOT_EXPOSED = new Set([
  "compose.parameter_not_exposed",
  "compose.unknown_mask",
  "compose.unknown_collider",
  "compose.pose_not_exposed",
]);

/** 合成のエラーの原因のうち、本体に無いもの (条件・使うパラメータ)。 */
const MISSING_IN_BASE = new Set([
  "compose.missing_parameter",
  "compose.missing_slot",
  "compose.avatar_level",
]);

/**
 * 適用のエラーの直し方の案内 (T1609。docs/editor.md#モジュールの適用のエラーの案内)。本体を直せば済むエラーにだけ案内を返し、
 * それ以外 (名前が使えない・モジュールとして表せないなど、エラーの文で足りるもの) は `null`。`base` は本体のパス
 * (モジュールのプロジェクトでなければ `null` で、案内しない)。
 */
export function moduleErrorHelp(error: unknown, base: string | null): ModuleErrorHelp | null {
  if (!base || !isMessage(error)) return null;
  let key: "readOnlyBase" | "needsBasePhysics" | "notExposed" | "missingInBase";
  const cause = error.cause?.code ?? "";
  switch (error.code) {
    case "edit.read_only_base":
    case "module.read_only_base":
      key = "readOnlyBase";
      break;
    case "module.collider_needs_base_physics":
      key = "needsBasePhysics";
      break;
    case "module.compose_failed":
      if (NOT_EXPOSED.has(cause)) key = "notExposed";
      else if (MISSING_IN_BASE.has(cause)) key = "missingInBase";
      else return null;
      break;
    default:
      return null;
  }
  const openBase = baseIsProject(base);
  // 書き出した .ugoe の本体は開けないので、直す場所 (本体のプロジェクト) だけを言う。
  const hint =
    openBase || key === "notExposed" || key === "missingInBase"
      ? t(`module.errorHelp.${key}`)
      : t("module.errorHelp.bundleBase");
  return { hint, openBase };
}

/**
 * 衝突体の参照を外して開く前に確かめる文 (T0661。T0655 の「黙って消さない」)。何を外し、いつファイルに書かれ、
 * どうすれば戻せるかを言う。
 */
export function dropCollidersPrompt(references: readonly ColliderReference[]): string {
  return t("moduleProblem.dropColliders.prompt", {
    list: colliderList(references),
    count: references.length,
  });
}

/** 衝突体の参照を外して開いた後の知らせ。 */
export function droppedCollidersNotice(references: readonly ColliderReference[]): string {
  return t("moduleProblem.dropColliders.done", {
    list: colliderList(references),
    count: references.length,
  });
}

/** 衝突体の参照の一覧 (`Sway → Shoulders, Tip → Shoulders`)。 */
function colliderList(references: readonly ColliderReference[]): string {
  return references.map((r) => `${r.rig} → ${r.collider}`).join(", ");
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
