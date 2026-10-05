// 着せ替え (T0706): WebPuppet の着せ替えの口 (crates/ugoe-wasm/README.md#着せ替え-t0123) が返す値の読み取りと、
// 利用者に見せる文 (名前空間 `player` のカタログ)。付け外しそのものは player.svelte.ts が行う。
import { t } from "./i18n";
import { describeLimitError } from "./loadError";
import {
  describeLoadWarning,
  type LoadWarning,
  readLoadWarnings,
  replaceWarnings,
} from "./loadWarnings";

/** 読み込んだモジュール (`WebPuppet.modules_json()` の要素)。 */
export interface ModuleInfo {
  name: string;
  slot: string;
  requires: { avatar_level?: number; slots?: string[]; parameters?: string[] };
  /** 今のモデルに付けているか。 */
  attached: boolean;
  textures: number;
  /** 読み込めたが注意が要ること (テクスチャの代用など)。`warning_details` から作った今の言語の文 (T0125)。 */
  warnings: string[];
}

/** 本体のスロット (`WebPuppet.slots_json()` の要素のうち、画面で使うもの)。 */
export interface SlotInfo {
  id: string;
  /** スロットが公開するパラメータ。 */
  parameters: string[];
}

/** スロットごとの表示: 本体のスロットと、そのスロットのモジュール (読み込んだ順)。 */
export interface SlotGroup {
  id: string;
  /** 本体に無いスロット (付けられない)。 */
  missing: boolean;
  /**
   * 本体のアバタープロファイルがアプリに公開していない (作者用の) スロット。そのスロットのモジュールを
   * 読み込んだときだけ出る (付けられない。付いていれば外せる)。
   */
  hidden: boolean;
  modules: ModuleInfo[];
  /** 付けているモジュール。 */
  attached: ModuleInfo | null;
}

function record(v: unknown): Record<string, unknown> {
  return typeof v === "object" && v !== null ? (v as Record<string, unknown>) : {};
}

function strings(v: unknown): string[] {
  return Array.isArray(v) ? v.filter((s): s is string => typeof s === "string") : [];
}

/** `modules_json()` を読む。形の違う要素は読み飛ばす。 */
export function parseModules(json: string): ModuleInfo[] {
  const list: unknown = JSON.parse(json);
  if (!Array.isArray(list)) return [];
  return list.flatMap((item) => {
    const m = record(item);
    if (typeof m.name !== "string" || typeof m.slot !== "string") return [];
    const r = record(m.requires);
    const requires: ModuleInfo["requires"] = {};
    if (typeof r.avatar_level === "number") requires.avatar_level = r.avatar_level;
    if (Array.isArray(r.slots)) requires.slots = strings(r.slots);
    if (Array.isArray(r.parameters)) requires.parameters = strings(r.parameters);
    return [
      {
        name: m.name,
        slot: m.slot,
        requires,
        attached: m.attached === true,
        textures: typeof m.textures === "number" ? m.textures : 0,
        warnings: replaceWarnings(
          strings(m.warnings),
          readLoadWarnings(m.warning_details),
          describeLoadWarning,
        ),
      },
    ];
  });
}

/** `slots_json()` を読む。 */
export function parseSlots(json: string): SlotInfo[] {
  const list: unknown = JSON.parse(json);
  if (!Array.isArray(list)) return [];
  return list.flatMap((item) => {
    const s = record(item);
    return typeof s.id === "string" ? [{ id: s.id, parameters: strings(s.parameters) }] : [];
  });
}

/**
 * `avatar_slots_json()` を読む: 本体のアバタープロファイルがアプリに公開するスロットの ID
 * (`docs/spec/avatar-profile.md#アプリ向けの情報`)。プロファイルが無い・形が違うなら `null` (公開の区別が無い)。
 */
export function parseAvatarSlots(json: string): string[] | null {
  const value: unknown = JSON.parse(json);
  return Array.isArray(value) ? strings(value) : null;
}

/**
 * 利用者に見せるスロット: 本体のアバタープロファイルが公開するもの (`published` が `null` ならすべて)。
 * 公開していないスロットは作者用で、アプリは利用者に見せない (avatar-profile.md の推奨)。
 */
export function visibleSlots(slots: SlotInfo[], published: string[] | null): SlotInfo[] {
  return published === null ? slots : slots.filter((s) => published.includes(s.id));
}

/**
 * スロットごとにまとめる。利用者に見せる本体のスロット (`visibleSlots`) を定義の順に並べ、それ以外のスロットの
 * モジュールは、公開していないスロットなら `hidden`、本体に無いスロットなら `missing` のグループとして後ろに付ける。
 */
export function groupBySlot(
  slots: SlotInfo[],
  modules: ModuleInfo[],
  published: string[] | null = null,
): SlotGroup[] {
  const groups: SlotGroup[] = visibleSlots(slots, published).map((s) => ({
    id: s.id,
    missing: false,
    hidden: false,
    modules: [],
    attached: null,
  }));
  for (const m of modules) {
    let group = groups.find((g) => g.id === m.slot);
    if (group === undefined) {
      const inModel = slots.some((s) => s.id === m.slot);
      group = { id: m.slot, missing: !inModel, hidden: inModel, modules: [], attached: null };
      groups.push(group);
    }
    group.modules.push(m);
    if (m.attached) group.attached = m;
  }
  return groups;
}

/** `DressUpError` (ugoe-wasm の `Error`) の中身。 */
export interface DressUpErrorInfo {
  code: string;
  message: string;
  module?: string;
  slot?: string;
  parameter?: string;
  mesh?: string;
  attached?: string;
}

/** 付け外しの失敗を読む。`DressUpError` でなければ `code` を `"unknown"` にする。 */
export function dressUpErrorInfo(e: unknown): DressUpErrorInfo {
  const message = e instanceof Error ? e.message : String(e);
  const r = record(e);
  const text = (key: string) => (typeof r[key] === "string" ? (r[key] as string) : undefined);
  const isDressUp = e instanceof Error && e.name === "DressUpError" && typeof r.code === "string";
  return {
    code: isDressUp ? (r.code as string) : "unknown",
    message,
    module: text("module"),
    slot: text("slot"),
    parameter: text("parameter"),
    mesh: text("mesh"),
    attached: text("attached"),
  };
}

/** 引数に入れる名前 (無ければ `?`)。 */
const name = (s: string | undefined) => s ?? "?";

/** `code` ごとの文のキー (コードの一覧は crates/ugoe-wasm/README.md#着せ替え-t0123)。 */
const DRESS_UP_ERROR_KEYS = {
  unknown_slot: "dressUp.error.unknownSlot",
  missing_slot: "dressUp.error.missingSlot",
  missing_parameter: "dressUp.error.missingParameter",
  parameter_not_exposed: "dressUp.error.parameterNotExposed",
  avatar_level: "dressUp.error.avatarLevel",
  slot_occupied: "dressUp.error.slotOccupied",
  unknown_mask: "dressUp.error.unknownMask",
  unknown_collider: "dressUp.error.unknownCollider",
  pose_not_exposed: "dressUp.error.poseNotExposed",
  texture_out_of_range: "dressUp.error.textureOutOfRange",
  invalid_module_name: "dressUp.error.invalidModuleName",
  duplicate_module: "dressUp.error.duplicateModule",
  invalid_composition: "dressUp.error.invalidComposition",
  compile: "dressUp.error.compile",
  invalid_physics: "dressUp.error.invalidPhysics",
  not_attached: "dressUp.error.notAttached",
  not_loaded: "dressUp.error.notLoaded",
  no_model: "dressUp.error.noModel",
  invalid_file: "dressUp.error.invalidFile",
  texture: "dressUp.error.texture",
  // GPU が止まったのはモデルの読み込みと同じ文 (T0934)。
  device_lost: "loadError.deviceLost",
} as const;

/**
 * 付け外しの失敗を、利用者に分かる言葉 (今の言語) にする。
 * 原因が細かいもの (合成・コンパイル・物理の検証など) は、wasm の文 (英語) を括弧で添える (`detail`)。
 */
export function describeDressUpError(e: unknown): string {
  // 読み込みの上限を超えたもの (`load_module`。T0152) は、モデルの読み込みと同じ文にする。
  return describeLimitError(e) ?? describeDressUpInfo(dressUpErrorInfo(e));
}

/** `describeDressUpError` の、読み取った後の中身から文を作る部分 (外したモジュールの `error` にも使う)。 */
export function describeDressUpInfo(info: DressUpErrorInfo): string {
  const key = DRESS_UP_ERROR_KEYS[info.code as keyof typeof DRESS_UP_ERROR_KEYS];
  if (key === undefined) return info.message;
  return t(key, {
    module: name(info.module),
    slot: name(info.slot),
    parameter: name(info.parameter),
    mesh: name(info.mesh),
    attached: name(info.attached),
    detail: info.message,
  });
}

/** モジュールを外した警告 (`module_detached`) の文。 */
function describeDetached(w: LoadWarning): string {
  const reason = w.error ? describeDressUpInfo(w.error) : w.warning;
  return t("dressUp.detached", { module: name(w.module), reason });
}

/**
 * モデルを開いたときの警告 (`load_json` / `load_bundle` の戻り値) を、`load_warnings_json()` の構造から
 * 今の言語の文の一覧にする (T0125)。付け直せずに外したモジュールは付けられないときと同じ原因の文にする。
 * 構造の無い文と知らない種類は、wasm の文 (英語) のまま残す。
 */
export function describeLoadWarnings(warnings: string[], structured: LoadWarning[]): string[] {
  return replaceWarnings(warnings, structured, (w) =>
    w.code === "module_detached" ? describeDetached(w) : describeLoadWarning(w),
  );
}
