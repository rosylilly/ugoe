// プレビューの着せ替え (docs/editor.md#プレビューの着せ替え) の、DOM と wasm に依存しない部分 (dressUp.test.ts)。
// 付け外しの失敗は ugoe-wasm の DressUpError (`code` と補足の値。crates/ugoe-wasm/README.md#着せ替え-t0123)。
import type { FileFilter } from "../filePaths";
import { t } from "../i18n";
import type { Slot } from "../ipc";
import { describeModuleWarnings } from "./loadWarnings";

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
  /** 読み込みの警告 (`warning_details` から作った英語の知らせ。構造の無いものは wasm の文)。 */
  warnings: string[];
}

/** `modules_json()` を読む。警告は `warning_details` から知らせにする (`loadWarnings.ts` と同じ言い回し)。 */
export function parseModules(json: string): LoadedModuleInfo[] {
  const list = JSON.parse(json) as (LoadedModuleInfo & { warning_details?: unknown })[];
  return list.map(({ warning_details, ...m }) => ({
    ...m,
    warnings: describeModuleWarnings(m.warnings ?? [], warning_details),
  }));
}

/**
 * `WebPuppet.detached_modules_json()` の要素: モデルを読み直したときに WebPuppet が付け直せずに外したモジュール。
 * `error` は DressUpError と同じ `code` と補足の値 (と英語の文 `message`)。
 */
export interface DetachedModule {
  module: string;
  slot: string;
  error: { code: string; message: string } & Record<string, unknown>;
  /** 読み込みの戻り値の文の警告と同じ文字列。 */
  warning: string;
}

export function parseDetachedModules(json: string): DetachedModule[] {
  return JSON.parse(json) as DetachedModule[];
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
  message?: unknown;
  module?: unknown;
  slot?: unknown;
  parameter?: unknown;
  mesh?: unknown;
  attached?: unknown;
}

function text(value: unknown, fallback: string): string {
  return typeof value === "string" && value !== "" ? value : fallback;
}

/**
 * エラー (DressUpError かそれ以外) を、分かる言葉の説明にする。外したモジュールの `error`
 * (`DetachedModule.error`。`code` と `message` を持つオブジェクト) も読める。
 */
export function describeProblem(error: unknown): DressUpProblem {
  const fields: DressUpErrorFields =
    error !== null && typeof error === "object" ? (error as DressUpErrorFields) : {};
  const detail =
    error instanceof Error
      ? error.message
      : typeof fields.message === "string"
        ? fields.message
        : String(error);
  const code = typeof fields.code === "string" ? fields.code : null;
  const module = text(fields.module, t("preview.dressUp.problem.theModule"));
  const slot = text(fields.slot, "?");
  const parameter = text(fields.parameter, "?");
  const mesh = text(fields.mesh, "?");
  const summary = ((): string => {
    switch (code) {
      case "unknown_slot":
        return t("preview.dressUp.problem.unknownSlot", { module, slot });
      case "missing_slot":
        return t("preview.dressUp.problem.missingSlot", { module, slot });
      case "missing_parameter":
        return t("preview.dressUp.problem.missingParameter", { module, parameter });
      case "parameter_not_exposed":
        return t("preview.dressUp.problem.parameterNotExposed", { module, parameter, slot });
      case "avatar_level":
        return t("preview.dressUp.problem.avatarLevel", { module });
      case "slot_occupied":
        return t("preview.dressUp.problem.slotOccupied", {
          slot,
          attached: text(fields.attached, t("preview.dressUp.problem.anotherModule")),
        });
      case "unknown_mask":
        return t("preview.dressUp.problem.unknownMask", { module, mesh });
      case "unknown_collider":
        return t("preview.dressUp.problem.unknownCollider", { module });
      case "pose_not_exposed":
        return t("preview.dressUp.problem.poseNotExposed", { module, slot });
      case "texture_out_of_range":
        return t("preview.dressUp.problem.textureOutOfRange", { module, mesh });
      case "invalid_composition":
        return t("preview.dressUp.problem.invalidComposition");
      case "invalid_module_name":
        return t("preview.dressUp.problem.invalidModuleName", { module });
      case "duplicate_module":
        return t("preview.dressUp.problem.duplicateModule", { module });
      case "compile":
        return t("preview.dressUp.problem.compile");
      case "invalid_physics":
        return t("preview.dressUp.problem.invalidPhysics");
      case "not_attached":
        return t("preview.dressUp.problem.notAttached", { module });
      case "not_loaded":
        return t("preview.dressUp.problem.notLoaded", { module });
      case "no_model":
        return t("preview.dressUp.problem.noModel");
      case "invalid_file":
        return t("preview.dressUp.problem.invalidFile");
      case "texture":
        return t("preview.dressUp.problem.texture");
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
  /**
   * アバタープロファイルがアプリに公開するスロットか (`avatar.slots`。プロファイルが無ければすべて公開とみなす)。
   * 公開していないスロットはビューアなどのアプリが利用者に見せない (T0715)。エディタは作者のツールなので隠さず印を付ける (T0655)。
   */
  published: boolean;
}

/**
 * 本体のスロット (モデルの定義順) ごとの行。`published` はアバタープロファイルの `slots`
 * (`docs/spec/avatar-profile.md#アプリ向けの情報`)。プロファイルが無ければ `null` (公開の区別が無い)。
 */
export function slotRows(
  slots: readonly Pick<Slot, "id">[],
  modules: readonly Pick<LoadedModuleInfo, "name" | "slot" | "attached">[],
  published: readonly string[] | null = null,
): SlotRow[] {
  return slots.map(({ id }) => {
    const here = modules.filter((m) => m.slot === id);
    return {
      id,
      attached: here.find((m) => m.attached)?.name ?? null,
      candidates: here.map((m) => m.name),
      published: published === null || published.includes(id),
    };
  });
}

/** 順序を問わず、同じ名前の集まりか。 */
export function sameNames(a: readonly string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false;
  const set = new Set(a);
  return b.every((n) => set.has(n));
}

/**
 * モジュールのプロジェクトのプレビューで、今の編集を読み込む名前 (T0653)。モジュールの名前のままだと、前に書き出した
 * 同じ名前の `.ugoemod` と一緒に読み込めない (WebPuppet は名前で区別する) ので、別の名前にする。
 */
export function editingName(name: string): string {
  return `${name} (editing)`;
}
