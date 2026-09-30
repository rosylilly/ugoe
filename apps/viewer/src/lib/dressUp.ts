// 着せ替え (T0706): WebPuppet の着せ替えの口 (crates/ugoe-wasm/README.md#着せ替え-t0123) が返す値の読み取りと、
// 利用者に見せる文。付け外しそのものは player.svelte.ts が行う。

/** 読み込んだモジュール (`WebPuppet.modules_json()` の要素)。 */
export interface ModuleInfo {
  name: string;
  slot: string;
  requires: { avatar_level?: number; slots?: string[]; parameters?: string[] };
  /** 今のモデルに付けているか。 */
  attached: boolean;
  textures: number;
  /** 読み込めたが注意が要ること (テクスチャの代用など。英語)。 */
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
        warnings: strings(m.warnings),
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
 * スロットごとにまとめる。本体のスロットを定義の順に並べ、本体に無いスロットのモジュールは
 * `missing` のグループとして後ろに付ける。
 */
export function groupBySlot(slots: SlotInfo[], modules: ModuleInfo[]): SlotGroup[] {
  const groups: SlotGroup[] = slots.map((s) => ({
    id: s.id,
    missing: false,
    modules: [],
    attached: null,
  }));
  for (const m of modules) {
    let group = groups.find((g) => g.id === m.slot);
    if (group === undefined) {
      group = { id: m.slot, missing: true, modules: [], attached: null };
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

const q = (s: string | undefined) => `「${s ?? "?"}」`;

/**
 * 付け外しの失敗を、利用者に分かる言葉にする (コードの一覧は crates/ugoe-wasm/README.md#着せ替え-t0123)。
 * 原因が細かいもの (合成・コンパイル・物理の検証など) は、wasm の文 (英語) を括弧で添える。
 */
export function describeDressUpError(e: unknown): string {
  return describeDressUpInfo(dressUpErrorInfo(e));
}

/** `describeDressUpError` の、読み取った後の中身から文を作る部分 (外したモジュールの `error` にも使う)。 */
export function describeDressUpInfo(info: DressUpErrorInfo): string {
  const m = q(info.module);
  const detail = ` (${info.message})`;
  switch (info.code) {
    case "unknown_slot":
      return `${m}を付けるスロット${q(info.slot)}が、このモデルにありません`;
    case "missing_slot":
      return `${m}には、モデルにスロット${q(info.slot)}が必要です`;
    case "missing_parameter":
      return `${m}に必要なパラメータ${q(info.parameter)}が、このモデルにありません`;
    case "parameter_not_exposed":
      return `${m}が使うパラメータ${q(info.parameter)}を、スロット${q(info.slot)}が公開していません`;
    case "avatar_level":
      return `${m}が求めるアバタープロファイルのレベルを、このモデルが満たしていません`;
    case "slot_occupied":
      return `スロット${q(info.slot)}には${q(info.attached)}が付いているので、${m}を一緒に付けられません`;
    case "unknown_mask":
      return `${m}のメッシュ${q(info.mesh)}のマスクが、モデルにもモジュールにもありません`;
    case "unknown_collider":
      return `${m}の物理が、このモデルに無い衝突体を使っています`;
    case "pose_not_exposed":
      return `${m}のポーズが、スロット${q(info.slot)}の公開していないパーツを使っています`;
    case "texture_out_of_range":
      return `${m}のメッシュ${q(info.mesh)}が、モジュールに無いテクスチャを使っています`;
    case "invalid_module_name":
      return `${m}はモジュールの名前として使えません`;
    case "duplicate_module":
      return `同じ名前のモジュール${m}を 2 つ付けることはできません`;
    case "invalid_composition":
      return `モデルとモジュールを合成できません${detail}`;
    case "compile":
      return `合成したモデルを組み立てられません${detail}`;
    case "invalid_physics":
      return `合成した物理の設定が正しくありません${detail}`;
    case "not_attached":
      return `${m}は付いていません`;
    case "not_loaded":
      return `${m}は読み込まれていません`;
    case "no_model":
      return "先にモデル (.ugoe か .json) を開いてください";
    case "invalid_file":
      return `着せ替えのモジュール (.ugoemod) として読めません${detail}`;
    case "texture":
      return `モジュールのテクスチャを登録できません${detail}`;
    default:
      return info.message;
  }
}

/** モデルを開き直したときに付け直せずに外したモジュール (`WebPuppet.detached_modules_json()` の要素)。 */
export interface DetachedModule {
  module: string;
  slot: string;
  /** 付けられなかった理由 (`DressUpError` と同じ `code` と補足の値)。 */
  error: DressUpErrorInfo;
  /** 読み込みの戻り値に入る文の警告と同じ文字列 (一覧から除くときに比べる)。 */
  warning: string;
}

/** `detached_modules_json()` を読む。形の違う要素は読み飛ばす。 */
export function parseDetachedModules(json: string): DetachedModule[] {
  const list: unknown = JSON.parse(json);
  if (!Array.isArray(list)) return [];
  return list.flatMap((item) => {
    const d = record(item);
    const e = record(d.error);
    if (typeof d.module !== "string" || typeof d.slot !== "string") return [];
    const text = (key: string) => (typeof e[key] === "string" ? (e[key] as string) : undefined);
    return [
      {
        module: d.module,
        slot: d.slot,
        error: {
          code: text("code") ?? "unknown",
          message: text("message") ?? "",
          module: text("module"),
          slot: text("slot"),
          parameter: text("parameter"),
          mesh: text("mesh"),
          attached: text("attached"),
        },
        warning: typeof d.warning === "string" ? d.warning : "",
      },
    ];
  });
}

/**
 * モデルを開き直したときの警告を、利用者に見せる一覧にする。付け直せずに外したモジュール (`detached`) は
 * 文の警告を除き、`code` から日本語の知らせを作って後ろに並べる。ほかの警告 (英語) はそのまま。
 */
export function describeLoadWarnings(warnings: string[], detached: DetachedModule[]): string[] {
  const covered = new Set(detached.map((d) => d.warning));
  return [
    ...warnings.filter((w) => !covered.has(w)),
    ...detached.map(
      (d) =>
        `このモデルに合わないため、着せ替えの${q(d.module)}を外しました (${describeDressUpInfo(d.error)})`,
    ),
  ];
}
