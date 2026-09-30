// プレビューの着せ替え (docs/editor.md#プレビューの着せ替え) の、DOM と wasm に依存しない部分 (dressUp.test.ts)。
// 付け外しの失敗は ugoe-wasm の DressUpError (`code` と補足の値。crates/ugoe-wasm/README.md#着せ替え-t0123)。
import type { FileFilter } from "../filePaths";
import type { Slot } from "../ipc";

export const MODULE_FILTER: FileFilter = {
  name: "ugoe module (.ugoemod)",
  extensions: ["ugoemod"],
};

/** 着せ替えのモジュールのファイル名か (拡張子 `.ugoemod`。大文字小文字は区別しない)。 */
export function isModuleFile(name: string): boolean {
  return name.toLowerCase().endsWith(".ugoemod");
}

/** パスの最後の要素から拡張子を除いたもの (読み込めなかったモジュールの仮の名前)。 */
export function fileStem(path: string): string {
  const name =
    path
      .replace(/[\\/]+$/, "")
      .split(/[\\/]/)
      .pop() ?? path;
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

/** `WebPuppet.modules_json()` の要素。 */
export interface LoadedModuleInfo {
  name: string;
  slot: string;
  requires: { avatar_level?: number | null; slots?: string[]; parameters?: string[] };
  attached: boolean;
  textures: number;
  warnings: string[];
}

export function parseModules(json: string): LoadedModuleInfo[] {
  return JSON.parse(json) as LoadedModuleInfo[];
}

/** 付け外しの失敗の、利用者向けの説明。 */
export interface DressUpProblem {
  /** 何が起きて、どうすればよいか。 */
  summary: string;
  /** 元のエラーの文言 (ツールチップと「Details」に出す)。 */
  detail: string;
  /** DressUpError の `code` (それ以外のエラーは `null`)。 */
  code: string | null;
}

interface DressUpErrorFields {
  code?: unknown;
  module?: unknown;
  slot?: unknown;
  parameter?: unknown;
  mesh?: unknown;
  attached?: unknown;
}

function text(value: unknown, fallback: string): string {
  return typeof value === "string" && value !== "" ? value : fallback;
}

/** エラー (DressUpError かそれ以外) を、分かる言葉の説明にする。 */
export function describeProblem(error: unknown): DressUpProblem {
  const detail = error instanceof Error ? error.message : String(error);
  const fields: DressUpErrorFields =
    error !== null && typeof error === "object" ? (error as DressUpErrorFields) : {};
  const code = typeof fields.code === "string" ? fields.code : null;
  const module = text(fields.module, "The module");
  const slot = text(fields.slot, "?");
  const parameter = text(fields.parameter, "?");
  const mesh = text(fields.mesh, "?");
  const summary = ((): string => {
    switch (code) {
      case "unknown_slot":
        return `${module} goes in slot "${slot}", but this model has no such slot. Add it in the Slots tab (the name must match exactly).`;
      case "missing_slot":
        return `${module} needs this model to have slot "${slot}". Add it in the Slots tab.`;
      case "missing_parameter":
        return `${module} uses parameter "${parameter}", which this model does not have.`;
      case "parameter_not_exposed":
        return `${module} uses parameter "${parameter}", but slot "${slot}" does not allow it. Tick it under the slot's Parameters in the Slots tab.`;
      case "avatar_level":
        return `${module} needs a higher avatar profile level than this model declares.`;
      case "slot_occupied":
        return `Slot "${slot}" already has ${text(fields.attached, "another module")}. Take that off first.`;
      case "unknown_mask":
        return `${module} masks mesh "${mesh}" with a mesh this model does not offer. Add it to the slot's Mask sources in the Slots tab.`;
      case "unknown_collider":
        return `${module}'s physics collides with a collider this model does not offer. Add it to the slot's Colliders in the Slots tab.`;
      case "pose_not_exposed":
        return `${module} follows a pose part that slot "${slot}" does not offer. Add it to the slot's Pose parts in the Slots tab.`;
      case "texture_out_of_range":
        return `${module} is broken: mesh "${mesh}" uses a texture the module does not have. Export the module again.`;
      case "invalid_composition":
        return "The model with this module on is not valid. See the details.";
      case "invalid_module_name":
        return `The module name "${module}" cannot be used (it must not be empty, contain / or start with #).`;
      case "duplicate_module":
        return `Two modules are named "${module}".`;
      case "compile":
        return "The model with this module on could not be prepared for drawing. See the details.";
      case "invalid_physics":
        return "The physics of the module does not fit this model. See the details.";
      case "not_attached":
        return `${module} is not on.`;
      case "not_loaded":
        return `${module} is not loaded. Load the file again.`;
      case "no_model":
        return "There is no model in the viewport.";
      case "invalid_file":
        return "This is not a .ugoemod file, or it is damaged or from a newer version of ugoe.";
      case "texture":
        return "The module's images could not be put on the GPU.";
      default:
        return detail;
    }
  })();
  return { summary, detail, code };
}

/** スロットの一覧の 1 行: 付いているモジュールと、このスロットに付けられる読み込んだモジュール。 */
export interface SlotRow {
  id: string;
  /** 付いているモジュールの名前。 */
  attached: string | null;
  /** このスロットを指す、読み込んだモジュールの名前 (読み込んだ順)。 */
  candidates: string[];
}

/** 本体のスロット (モデルの定義順) ごとの行。 */
export function slotRows(
  slots: readonly Pick<Slot, "id">[],
  modules: readonly Pick<LoadedModuleInfo, "name" | "slot" | "attached">[],
): SlotRow[] {
  return slots.map(({ id }) => {
    const here = modules.filter((m) => m.slot === id);
    return {
      id,
      attached: here.find((m) => m.attached)?.name ?? null,
      candidates: here.map((m) => m.name),
    };
  });
}

/** 順序を問わず、同じ名前の集まりか。 */
export function sameNames(a: readonly string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false;
  const set = new Set(a);
  return b.every((n) => set.has(n));
}
