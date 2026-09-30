// 読み込みの警告 (T0125): WebPuppet の `load_warnings_json()` と、モジュールの `warning_details`
// (crates/ugoe-wasm/README.md#読み込みの警告-t0125) の読み取りと、利用者に見せる文。
// 着せ替えのモジュールを外した警告 (`module_detached`) の文は dressUp.ts の `describeLoadWarnings` が作る。
import type { DressUpErrorInfo } from "./dressUp";

/** 構造にした警告 1 つ。`warning` は wasm の文の警告 (英語) と同じ文字列。 */
export interface LoadWarning {
  /**
   * `texture_ktx2` | `texture_missing` | `normal_map_ktx2` | `physics_removed` | `module_detached` |
   * `unknown_optional_feature` | `unknown_value` (T0412。ほかは将来の種類)。
   */
  code: string;
  warning: string;
  /** テクスチャの番号 (テクスチャ・法線マップの警告)。 */
  texture?: number;
  /** 物理を外した理由 (英語)。 */
  reason?: string;
  /** 知らない任意の機能の ID (`unknown_optional_feature`)。 */
  feature?: string;
  /** 知らない値のあった場所 (`unknown_value`。チャンクとその中の場所)。 */
  path?: string;
  /** 外したモジュールの名前とスロット、付けられなかった理由 (`module_detached`)。 */
  module?: string;
  slot?: string;
  error?: DressUpErrorInfo;
}

function record(v: unknown): Record<string, unknown> {
  return typeof v === "object" && v !== null ? (v as Record<string, unknown>) : {};
}

/** 警告の配列 (JSON を解析したもの) を読む。`code` か `warning` の無い要素は読み飛ばす。 */
export function readLoadWarnings(list: unknown): LoadWarning[] {
  if (!Array.isArray(list)) return [];
  return list.flatMap((item) => {
    const w = record(item);
    if (typeof w.code !== "string" || typeof w.warning !== "string") return [];
    const text = (r: Record<string, unknown>, key: string) =>
      typeof r[key] === "string" ? (r[key] as string) : undefined;
    const warning: LoadWarning = { code: w.code, warning: w.warning };
    if (typeof w.texture === "number") warning.texture = w.texture;
    if (typeof w.reason === "string") warning.reason = w.reason;
    if (typeof w.feature === "string") warning.feature = w.feature;
    if (typeof w.path === "string") warning.path = w.path;
    if (typeof w.module === "string") warning.module = w.module;
    if (typeof w.slot === "string") warning.slot = w.slot;
    if (w.error !== undefined) {
      const e = record(w.error);
      warning.error = {
        code: text(e, "code") ?? "unknown",
        message: text(e, "message") ?? "",
        module: text(e, "module"),
        slot: text(e, "slot"),
        parameter: text(e, "parameter"),
        mesh: text(e, "mesh"),
        attached: text(e, "attached"),
      };
    }
    return [warning];
  });
}

/** `load_warnings_json()` を読む。 */
export function parseLoadWarnings(json: string): LoadWarning[] {
  return readLoadWarnings(JSON.parse(json));
}

/**
 * 着せ替え以外の警告を、利用者に分かる言葉にする。知らない種類と `module_detached` は `null`
 * (呼ぶ側が決める)。
 */
export function describeLoadWarning(w: LoadWarning): string | null {
  const texture = `テクスチャ ${w.texture ?? "?"}`;
  switch (w.code) {
    case "texture_ktx2":
      return `${texture} は KTX2 の画像で、まだ表示できないため白で代わりに描いています`;
    case "texture_missing":
      return `${texture} の画像がファイルに無いため、白で代わりに描いています`;
    case "normal_map_ktx2":
      return `${texture} の法線マップは KTX2 の画像で、まだ使えないため法線マップなしで描いています`;
    case "physics_removed":
      return `物理の設定がこのモデルに合わないため外しました (${w.reason ?? w.warning})`;
    case "unknown_optional_feature":
      return `このファイルは新しい版の機能「${w.feature ?? "?"}」を使っています。この版では代わりの方法で表示しています`;
    case "unknown_value":
      return `このファイルには、この版が知らない値があります (${w.path ?? "?"})。既定の扱いで表示しています`;
    default:
      return null;
  }
}

/**
 * 文の警告の一覧を、構造から作った文に置き換える (順は文の警告のまま)。構造の `warning` と同じ文は
 * `describe` の文にし、構造の無い文 (と `describe` が `null` の種類) はそのまま残す。
 */
export function replaceWarnings(
  warnings: readonly string[],
  structured: readonly LoadWarning[],
  describe: (w: LoadWarning) => string | null,
): string[] {
  const byText = new Map(structured.map((w) => [w.warning, w]));
  return warnings.map((text) => {
    const w = byText.get(text);
    return (w && describe(w)) ?? text;
  });
}
