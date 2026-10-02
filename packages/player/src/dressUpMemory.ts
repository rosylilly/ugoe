// 付けているモジュールの組をモデルごとに覚える (T0711)。覚える入れ物とモデルの鍵は modelMemory.ts (T0709 と共有)。
// 覚えるのはモジュールの名前・ファイルの鍵 (名前・指紋・読み直せる場所) と、付けていたもの (合成の順)。
// ファイルの中身は覚えない (localStorage の容量のため)。戻すときに場所から読み直す (player.svelte.ts)。

import { t } from "./i18n";
import type { FileLocation, ModelFile } from "./modelFile";
import { type ModelIdentity, ModelMemory, parseIdentity } from "./modelMemory";

/** localStorage のキー。 */
export const DRESS_UP_STORAGE_KEY = "ugoe-viewer.dress-up";
/** 自動で戻すかの設定の localStorage のキー。 */
export const AUTO_RESTORE_STORAGE_KEY = "ugoe-viewer.dress-up.auto-restore";
/** 覚えるモデルの数の上限。 */
export const MAX_MODELS = 20;
/** 1 つのモデルで覚えるモジュールの数の上限 (壊れた値で大きな一覧を読まないため)。 */
export const MAX_MODULES = 64;

/** 読み込んでいたモジュール。 */
export interface RememberedModule {
  /** モジュールの名前 (wasm の `load_module` が返す名前)。 */
  name: string;
  /** ファイルの鍵 (`name` はファイル名)。 */
  file: ModelIdentity;
}

/** 1 つのモデルで覚える組。 */
export interface DressUpSet {
  /** 読み込んでいたモジュール (読み込んだ順。付けていないものも含む)。 */
  modules: RememberedModule[];
  /** 付けていたモジュールの名前 (合成の順)。 */
  attached: string[];
}

/** 保存した組を読む。形の違うモジュールは読み飛ばし、読み込んでいないモジュールを付けていたことにはしない。 */
export function parseDressUpSet(v: unknown): DressUpSet | null {
  if (typeof v !== "object" || v === null) return null;
  const r = v as Record<string, unknown>;
  if (!Array.isArray(r.modules)) return null;
  const modules: RememberedModule[] = [];
  for (const item of r.modules) {
    if (modules.length >= MAX_MODULES) break;
    if (typeof item !== "object" || item === null) continue;
    const m = item as Record<string, unknown>;
    const file = parseIdentity(m.file);
    if (typeof m.name !== "string" || m.name === "" || file === null) continue;
    if (modules.some((x) => x.name === m.name)) continue;
    modules.push({ name: m.name, file });
  }
  const names = new Set(modules.map((m) => m.name));
  const attached = Array.isArray(r.attached)
    ? [...new Set(r.attached.filter((n): n is string => typeof n === "string" && names.has(n)))]
    : [];
  return { modules, attached };
}

/** 付けているものが同じか (合成の順も比べる)。 */
export function sameAttached(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((name, i) => name === b[i]);
}

/**
 * 覚えている組を戻す必要があるか: 付けているものが違うか、覚えているモジュールを読み込んでいない。
 * `loaded` はいま読み込んでいるモジュールの名前、`attached` は付けているもの (合成の順)。
 */
export function needsRestore(
  set: DressUpSet,
  loaded: ReadonlySet<string>,
  attached: readonly string[],
): boolean {
  return !sameAttached(set.attached, attached) || set.modules.some((m) => !loaded.has(m.name));
}

/** モデルごとの組の入れ物。 */
export function dressUpMemory(storage?: Storage): ModelMemory<DressUpSet> {
  return new ModelMemory(DRESS_UP_STORAGE_KEY, parseDressUpSet, { limit: MAX_MODELS, storage });
}

/** 戻すときに、モジュールのファイルをどこから得るか。 */
export type RestoreSource =
  /** いま読み込んでいる (同じ名前のモジュールがある)。 */
  | { kind: "loaded" }
  /** このセッションで読んだことがあり、手元に中身がある。 */
  | { kind: "cached"; file: ModelFile }
  /** 場所から読み直す。 */
  | { kind: "location"; location: FileLocation }
  /** 読み直せない (ドロップやブラウザで選んだファイルで、手元に中身も無い)。 */
  | { kind: "unavailable" };

/**
 * 組を戻すために、覚えているモジュールをそれぞれどこから得るかを決める。
 * 読み込んでいるもの (名前) → このセッションで読んだ中身 (指紋) → 場所の順。
 */
export function planRestore(
  set: DressUpSet,
  loaded: ReadonlySet<string>,
  cached: (fingerprint: string) => ModelFile | undefined,
): { module: RememberedModule; source: RestoreSource }[] {
  return set.modules.map((module) => {
    if (loaded.has(module.name)) return { module, source: { kind: "loaded" } };
    const file = cached(module.file.fingerprint);
    if (file !== undefined) return { module, source: { kind: "cached", file } };
    const location = module.file.location;
    if (location !== undefined) return { module, source: { kind: "location", location } };
    return { module, source: { kind: "unavailable" } };
  });
}

/** 戻せなかったモジュールの知らせ。`reason` が無ければ読み直せる場所が無かった。 */
export function describeRestoreFailure(module: RememberedModule, reason?: string): string {
  const args = { module: module.name, file: module.file.name };
  return reason === undefined
    ? t("dressUp.restore.noLocation", args)
    : t("dressUp.restore.unreadable", { ...args, reason });
}

/** 開いたときに自動で戻すか (既定は尋ねる)。読めなければ `false`。 */
export function loadAutoRestore(storage?: Storage): boolean {
  try {
    return (storage ?? globalThis.localStorage)?.getItem(AUTO_RESTORE_STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

/** 自動で戻すかを覚える。覚えられなくても無視する。 */
export function saveAutoRestore(enabled: boolean, storage?: Storage): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(AUTO_RESTORE_STORAGE_KEY, String(enabled));
  } catch {
    // 覚えられないだけで、今回の設定は効く。
  }
}
