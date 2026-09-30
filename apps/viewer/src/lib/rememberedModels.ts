// モデルごとに覚えたもの (光源の設定 (T0709) と着せ替えの組 (T0711)) の一覧 (T0714)。
// 2 つの入れ物 (modelMemory.ts の `ModelMemory`) の項目を、同じモデル (`sameModel`) ごとに 1 行にまとめる。
import type { DressUpSet } from "./dressUpMemory";
import type { LightingSettings } from "./lighting";
import { type ModelIdentity, sameModel } from "./modelMemory";

/** 一覧の 1 行 (1 つのモデル)。覚えていないものは `null`。 */
export interface RememberedModel {
  /** モデルの鍵 (光源の項目があればその鍵、無ければ着せ替えの項目の鍵)。 */
  model: ModelIdentity;
  lighting: LightingSettings | null;
  dressUp: DressUpSet | null;
  /** いま開いているモデルか。 */
  current: boolean;
}

/**
 * 2 つの入れ物の項目を、同じモデルごとにまとめる。並びは開いているモデルを先頭に、あとは名前の順
 * (入れ物ごとに最近使った順は違うので、どちらにも寄せない)。
 */
export function mergeRemembered(
  lighting: readonly { model: ModelIdentity; value: LightingSettings }[],
  dressUp: readonly { model: ModelIdentity; value: DressUpSet }[],
  current: ModelIdentity | null,
): RememberedModel[] {
  const rows: RememberedModel[] = lighting.map((e) => ({
    model: e.model,
    lighting: e.value,
    dressUp: null,
    current: false,
  }));
  for (const e of dressUp) {
    const row = rows.find((r) => r.dressUp === null && sameModel(r.model, e.model));
    if (row !== undefined) row.dressUp = e.value;
    else rows.push({ model: e.model, lighting: null, dressUp: e.value, current: false });
  }
  for (const row of rows) row.current = current !== null && sameModel(current, row.model);
  return rows.sort(
    (a, b) => Number(b.current) - Number(a.current) || a.model.name.localeCompare(b.model.name),
  );
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
