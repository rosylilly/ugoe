// モデルごとに覚えたもの (光源の設定 (T0709)・着せ替えの組 (T0711)・待機モーションの選択 (T0722)) の一覧 (T0714)。
// 入れ物 (modelMemory.ts の `ModelMemory`) の項目を、同じモデル (`sameModel`) ごとに 1 行にまとめる。
import type { IdleChoice } from "./animation";
import type { DressUpSet } from "./dressUpMemory";
import type { LightingSettings } from "./lighting";
import { type MemoryEntry, type ModelIdentity, sameModel } from "./modelMemory";

/** 一覧の 1 行 (1 つのモデル)。覚えていないものは `null`。 */
export interface RememberedModel {
  /** モデルの鍵 (光源の項目があればその鍵、無ければ着せ替え、待機モーションの項目の鍵)。 */
  model: ModelIdentity;
  lighting: LightingSettings | null;
  dressUp: DressUpSet | null;
  /** 待機モーションの選択 (T0722)。 */
  idle: IdleChoice | null;
  /** いま開いているモデルか。 */
  current: boolean;
  /**
   * 最後に覚えた日時 (ミリ秒。覚えたものの中で最も新しいもの、T0717)。日時を持たない前の保存値だけなら `null`。
   */
  rememberedAt: number | null;
}

/** 一覧の並び。`recent` は覚えた日時の新しい順 (日時なしは最後に名前の順)、`name` は名前の順。 */
export type RememberedOrder = "recent" | "name";

/** 一覧の並びの選択の localStorage のキー (T0718)。 */
export const REMEMBERED_ORDER_STORAGE_KEY = "ugoe-viewer.remembered-order";

/** 覚えた並びの選択。覚えていない・読めない・知らない値なら既定の `recent`。 */
export function loadRememberedOrder(storage?: Storage): RememberedOrder {
  try {
    const value = (storage ?? globalThis.localStorage)?.getItem(REMEMBERED_ORDER_STORAGE_KEY);
    return value === "name" ? "name" : "recent";
  } catch {
    return "recent";
  }
}

/** 並びの選択を覚える。覚えられなくても無視する (今回の並びは効く)。 */
export function saveRememberedOrder(order: RememberedOrder, storage?: Storage): void {
  try {
    (storage ?? globalThis.localStorage)?.setItem(REMEMBERED_ORDER_STORAGE_KEY, order);
  } catch {
    // 覚えられないだけで、今回の並びは効く。
  }
}

/** 一覧にまとめる入れ物ごとの項目。 */
export interface RememberedSources {
  lighting: readonly MemoryEntry<LightingSettings>[];
  dressUp: readonly MemoryEntry<DressUpSet>[];
  /** 待機モーションの選択 (T0722)。 */
  idle: readonly MemoryEntry<IdleChoice>[];
}

/** 一覧の 1 行の、入れ物ごとの値の欄。 */
type Kind = keyof RememberedSources;

/**
 * 入れ物ごとの項目を、同じモデルごとにまとめる。並びは開いているモデルを先頭に、あとは `order`
 * (既定は覚えた日時の新しい順。T0717 より前は、入れ物ごとの最近使った順が揃わないので名前の順だった)。
 * 行の鍵は、最初に見つかった入れ物 (光源・組・待機モーションの順) の項目の鍵。
 */
export function mergeRemembered(
  sources: RememberedSources,
  current: ModelIdentity | null,
  order: RememberedOrder = "recent",
): RememberedModel[] {
  const rows: RememberedModel[] = [];
  const add = <K extends Kind>(kind: K, entries: RememberedSources[K]) => {
    for (const e of entries) {
      let row = rows.find((r) => r[kind] === null && sameModel(r.model, e.model));
      if (row === undefined) {
        row = {
          model: e.model,
          lighting: null,
          dressUp: null,
          idle: null,
          current: false,
          rememberedAt: null,
        };
        rows.push(row);
      }
      (row as Record<Kind, unknown>)[kind] = e.value;
      row.rememberedAt = newer(row.rememberedAt, e.at ?? null);
    }
  };
  add("lighting", sources.lighting);
  add("dressUp", sources.dressUp);
  add("idle", sources.idle);
  for (const row of rows) row.current = current !== null && sameModel(current, row.model);
  const byName = (a: RememberedModel, b: RememberedModel) =>
    a.model.name.localeCompare(b.model.name);
  const byRecent = (a: RememberedModel, b: RememberedModel) =>
    (b.rememberedAt ?? Number.NEGATIVE_INFINITY) - (a.rememberedAt ?? Number.NEGATIVE_INFINITY) ||
    byName(a, b);
  return rows.sort(
    (a, b) =>
      Number(b.current) - Number(a.current) || (order === "recent" ? byRecent : byName)(a, b),
  );
}

/** 行に覚えているものの数 (2 つ以上なら「すべて忘れる」を出す)。 */
export function rememberedCount(row: RememberedModel): number {
  return [row.lighting, row.dressUp, row.idle].filter((v) => v !== null).length;
}

function newer(a: number | null, b: number | null): number | null {
  if (a === null) return b;
  return b === null ? a : Math.max(a, b);
}

/** 覚えた日時の表示 (`2026/10/01 09:05`、その環境の時刻)。日時が無ければ「日時なし」。 */
export function describeRememberedAt(at: number | null): string {
  if (at === null) return "日時なし";
  const d = new Date(at);
  const two = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}/${two(d.getMonth() + 1)}/${two(d.getDate())} ${two(d.getHours())}:${two(d.getMinutes())}`;
}

/** 覚えた場所の表示 (`sample:` は見本、`path:` はパス、`url:` は URL。無ければ中身で見分けることを書く)。 */
export function describeLocation(model: ModelIdentity): string {
  const location = model.location;
  if (location === undefined) return "場所なし (中身で見分ける)";
  const colon = location.indexOf(":");
  const kind = location.slice(0, colon);
  const rest = location.slice(colon + 1);
  if (kind === "sample") return `見本 (${rest})`;
  return rest;
}

/** 着せ替えの組の短い説明 (付けていたモジュールの名前)。 */
export function describeDressUpSet(set: DressUpSet): string {
  if (set.attached.length > 0) return set.attached.join("・");
  return set.modules.length > 0 ? `付けていない (${set.modules.length} 個を読み込み)` : "なし";
}
