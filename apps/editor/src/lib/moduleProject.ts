// モジュールのプロジェクト (ADR 0024、docs/editor.md#モジュールのプロジェクト) の表示の補助。
// 作業用のモデルは本体と合成したもので、本体の要素は読み取り専用の下敷き。
import type { ModuleStatus } from "./ipc";
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

/** 要素が本体のもの (モジュールのプロジェクトでは編集できない) か。 */
export function isBaseElement(base: Set<string>, ref: ElementRef): boolean {
  return base.has(`${ref.kind}:${ref.id}`);
}

/** 本体が見つからないときのエラー (バックエンドの `ProjectError::BaseNotFound` の表示)。 */
export function isBaseNotFoundError(message: string | null): boolean {
  return message?.includes("base model not found") ?? false;
}

/** ファイルメニューに出すモジュールの印 (例: `Module: Hair`)。本体は `moduleTitle` で見せる。 */
export function moduleLabel(module: ModuleStatus): string {
  return `Module: ${module.slot.id}`;
}

/** モジュールの印の説明 (例: `Slot Hair of avatar.ugoeproj (read-only base: /path/avatar.ugoeproj)`)。 */
export function moduleTitle(module: ModuleStatus): string {
  const file = module.base.split(/[\\/]/).filter(Boolean).pop() ?? module.base;
  return `Slot ${module.slot.id} of ${file} (read-only base: ${module.base})`;
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
