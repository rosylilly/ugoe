// プレビューで見せる、モデルの読み込みの警告 (T0125)。WebPuppet の `load_warnings_json()` と、着せ替えのモジュールの
// `warning_details` (T0652) (crates/ugoe-wasm/README.md#読み込みの警告-t0125) の `code` から英語の知らせを作る。
// DOM と wasm に依存しない。

/** `load_warnings_json()` の要素のうち、プレビューが読む項目。 */
export interface LoadWarning {
  code: string;
  /** wasm の文の警告 (知らない種類はこれを出す)。 */
  warning: string;
  /** テクスチャの番号 (テクスチャ・法線マップの警告)。 */
  texture?: number;
  /** 物理を外した理由 (`physics_removed`)。 */
  reason?: string;
}

/** 警告の配列 (JSON を解析したもの) を読む。`code` か `warning` の無い要素は読み飛ばす。 */
export function readLoadWarnings(list: unknown): LoadWarning[] {
  if (!Array.isArray(list)) return [];
  return list.flatMap((item) => {
    const w = (typeof item === "object" && item !== null ? item : {}) as Record<string, unknown>;
    if (typeof w.code !== "string" || typeof w.warning !== "string") return [];
    const warning: LoadWarning = { code: w.code, warning: w.warning };
    if (typeof w.texture === "number") warning.texture = w.texture;
    if (typeof w.reason === "string") warning.reason = w.reason;
    return [warning];
  });
}

/** `load_warnings_json()` を読む。 */
export function parseLoadWarnings(json: string): LoadWarning[] {
  return readLoadWarnings(JSON.parse(json));
}

/** 警告 1 つの英語の知らせ。知らない種類は wasm の文 (`warning`) をそのまま返す。 */
export function describeLoadWarning(w: LoadWarning): string {
  const texture = `Texture ${w.texture ?? "?"}`;
  switch (w.code) {
    case "physics_removed":
      return `The physics settings no longer fit this model and were turned off (${w.reason ?? w.warning}).`;
    case "texture_ktx2":
      return `${texture} is a KTX2 image, which the preview can't show yet, so it is drawn white.`;
    case "texture_missing":
      return `${texture} has no image, so it is drawn white.`;
    case "normal_map_ktx2":
      return `The normal map of ${texture.toLowerCase()} is a KTX2 image, which the preview can't use yet, so it is lit without it.`;
    default:
      return w.warning;
  }
}

/**
 * `modules_json()` の要素の文の警告 (`warnings`) を、同じ順の構造 (`warning_details`) から作った知らせにする。
 * 構造の無い文 (古い wasm など) はそのまま残す。
 */
export function describeModuleWarnings(warnings: readonly string[], details: unknown): string[] {
  const byText = new Map(readLoadWarnings(details).map((w) => [w.warning, w]));
  return warnings.map((text) => {
    const w = byText.get(text);
    return w ? describeLoadWarning(w) : text;
  });
}

/**
 * プレビューに出す知らせ (読み込みの順)。外したモジュール (`module_detached`) は着せ替え (DressUp) が知らせるので除く。
 * `physicsReplaced` (読み直した後にドキュメントの物理を渡し直した) なら、前の物理を外したこと (`physics_removed`) も除く。
 */
export function describeLoadWarnings(
  warnings: readonly LoadWarning[],
  physicsReplaced: boolean,
): string[] {
  return warnings.flatMap((w) => {
    if (w.code === "module_detached") return [];
    if (w.code === "physics_removed" && physicsReplaced) return [];
    return [describeLoadWarning(w)];
  });
}
