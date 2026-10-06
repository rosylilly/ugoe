// グルーの編集 (docs/editor.md#グルーの編集) の補助。DOM や IPC に依存しない純粋な関数だけを置く (glues.test.ts で検査する)。
// 頂点の組を作るのは editor-core (`create_glue`・`set_glue` の `distance`)。ここでは送るコマンドを組み立てるだけ。
import { t } from "../../i18n";
import type { Command, Glue, GluePair, ModuleStatus, Parameter } from "../../ipc";
import { GLUE_PREFIX as DRAW_CLONE_PREFIX } from "../inspector/drawClone/drawClone";
import { cellAt, keyAt, keyTolerance } from "../inspector/keyforms/keyforms";

/** 頂点の組を作るときの既定の距離 (キャンバスのピクセル)。 */
export const DEFAULT_GLUE_DISTANCE = 8;

/**
 * モジュールのプロジェクトで、グルーが本体のもの (読み取り専用) か。モジュールのグルーはモジュールのメッシュだけを
 * つなぐ (本体のメッシュをつなぐとモジュールに書き出せない) ので、どちらかの端が本体のメッシュなら本体のもの。
 */
export function isBaseGlue(
  glue: Pick<Glue, "mesh_a" | "mesh_b">,
  module: ModuleStatus | null,
): boolean {
  if (!module) return false;
  return module.base_meshes.includes(glue.mesh_a) || module.base_meshes.includes(glue.mesh_b);
}

/** 描画用の複製のグルー (複製の形を元に合わせるもの。インスペクタの「Sync」が作り直す) か。 */
export function isDrawCloneGlue(glue: Pick<Glue, "id">): boolean {
  const last = glue.id.split("/").pop() ?? glue.id;
  return glue.id.startsWith(DRAW_CLONE_PREFIX) || last.startsWith(DRAW_CLONE_PREFIX);
}

/** グルーでつなげるメッシュ (モジュールのプロジェクトでは本体のメッシュを除く)。 */
export function glueableMeshes<T extends { id: string }>(
  meshes: readonly T[],
  module: ModuleStatus | null,
): T[] {
  return module ? meshes.filter((m) => !module.base_meshes.includes(m.id)) : [...meshes];
}

/** グルーの重み (組ごとに持つが、画面では先頭の組の値をまとめて見せる)。組が無ければ 0.5。 */
export function glueWeight(glue: Glue): number {
  return glue.pairs[0]?.weight ?? 0.5;
}

/** 組がすべて同じ重みか (違えば画面に注記する)。 */
export function uniformWeight(glue: Glue): boolean {
  const w = glueWeight(glue);
  return glue.pairs.every((p) => p.weight === w);
}

/** すべての組の重みを `weight` にしたグルー。 */
export function withWeight(glue: Glue, weight: number): Glue {
  return { ...glue, pairs: glue.pairs.map((p) => ({ ...p, weight })) };
}

/** `index` 番目のキーフォームの強さを `intensity` にしたグルー。 */
export function withIntensity(glue: Glue, index: number, intensity: number): Glue {
  const keyforms = glue.keyforms.keyforms.map((k, i) => (i === index ? { ...k, intensity } : k));
  return { ...glue, keyforms: { ...glue.keyforms, keyforms } };
}

/** 両端のメッシュを入れ替えたグルー (組の a と b も入れ替え、重みは 1 - w にして見た目を保つ)。 */
export function swapped(glue: Glue): Glue {
  return {
    ...glue,
    mesh_a: glue.mesh_b,
    mesh_b: glue.mesh_a,
    pairs: glue.pairs.map((p) => ({ a: p.b, b: p.a, weight: 1 - p.weight })),
  };
}

/** キーフォームの見出し (軸が無ければ `Intensity`、あれば `AngleX = 30, Glue = 1` のようなセル)。 */
export function keyformLabels(glue: Glue): string[] {
  const { bindings, keyforms } = glue.keyforms;
  if (bindings.length === 0) return keyforms.map(() => t("glues.intensity.label"));
  return keyforms.map((_, i) =>
    Object.entries(cellAt(bindings, i))
      .map(([p, v]) => `${p} = ${v}`)
      .join(", "),
  );
}

/** グルーを作るコマンド。 */
export function createGlueCommand(meshA: string, meshB: string, distance: number): Command {
  return { op: "create_glue", mesh_a: meshA, mesh_b: meshB, distance };
}

/** グルーを置き換えるコマンド (`distance` を渡すと頂点の組を作り直す)。 */
export function setGlueCommand(glue: Glue, distance?: number): Command {
  return distance === undefined ? { op: "set_glue", glue } : { op: "set_glue", glue, distance };
}

/** 作るグルーの両端の既定: ツリーで選んだメッシュのうち、つなげるものの先頭 2 つ。 */
export function selectedPair(
  selected: readonly string[],
  glueable: readonly string[],
): [string, string] | null {
  const meshes = selected.filter((id) => glueable.includes(id));
  const [a, b] = meshes;
  return a !== undefined && b !== undefined ? [a, b] : null;
}

/** 距離の入力 (0 以上の有限の値)。不正なら `null`。 */
export function distanceValue(text: string): number | null {
  const value = Number(text);
  return text.trim() !== "" && Number.isFinite(value) && value >= 0 ? value : null;
}

// --- 強さの軸 (T0661) ---

/** グルーの強さの格子にキーを足すコマンド (軸が無ければ軸ごと作る。`add_keyform_key`)。 */
export function addGlueKeyCommand(glue: string, parameter: string, key: number): Command {
  return { op: "add_keyform_key", target: { kind: "glue", id: glue }, parameter, key };
}

/** グルーの強さの格子からキーを取り除くコマンド (軸の最後のキーなら軸ごと。`remove_keyform_key`)。 */
export function removeGlueKeyCommand(glue: string, parameter: string, key: number): Command {
  return { op: "remove_keyform_key", target: { kind: "glue", id: glue }, parameter, key };
}

/** 強さの軸にまだ使っていないパラメータ (軸に足せるもの。モデルの順)。 */
export function unboundParameters<T extends Pick<Parameter, "id">>(
  glue: Glue,
  parameters: readonly T[],
): T[] {
  const bound = new Set(glue.keyforms.bindings.map((b) => b.parameter));
  return parameters.filter((p) => !bound.has(p.id));
}

/**
 * キーの入力 (パラメータの範囲の有限の値)。軸に同じキー (キーフォームの格子と同じ許容差で比べる) があれば `null`。
 */
export function keyValue(
  value: number | null,
  parameter: Pick<Parameter, "min" | "max">,
  keys: readonly number[] = [],
): number | null {
  if (value === null || !Number.isFinite(value)) return null;
  if (value < parameter.min || value > parameter.max) return null;
  return keyAt([...keys], value, keyTolerance(parameter)) === undefined ? value : null;
}

/** キーの欄の矢印キーの刻み。値域を 10〜100 等分した程度の、きりのよい大きさ。 */
export function keyStep(parameter: Pick<Parameter, "min" | "max">): number {
  const range = parameter.max - parameter.min;
  if (!(range > 0 && Number.isFinite(range))) return 0.1;
  return 10 ** Math.floor(Math.log10(range / 10));
}

// --- 頂点の組をビューポートで編集する (T0661) ---

/**
 * 組 `(a, b)` を足したグルー。B の頂点はいつも 1 つの A の頂点とだけ組む (作るときと同じ) ので、同じ B の組があれば
 * 置き換える。重みはグルーの重み (先頭の組。`glueWeight`)。同じ組がもうあれば `null`。
 */
export function withPair(glue: Glue, a: number, b: number): Glue | null {
  if (hasPair(glue, a, b)) return null;
  const pair: GluePair = { a, b, weight: glueWeight(glue) };
  const index = glue.pairs.findIndex((p) => p.b === b);
  const pairs =
    index < 0 ? [...glue.pairs, pair] : glue.pairs.map((p, i) => (i === index ? pair : p));
  return { ...glue, pairs };
}

/**
 * A の頂点 `a` と B の頂点 `b` の組を外したグルー。その組が無ければ null。番号でなく頂点で指すので、前の削除を
 * 読み直す前に押した組も、送る直前のグルーから外せる。
 */
export function withoutPair(glue: Glue, a: number, b: number): Glue | null {
  const pairs = glue.pairs.filter((p) => p.a !== a || p.b !== b);
  return pairs.length === glue.pairs.length ? null : { ...glue, pairs };
}

// --- 頂点の組をパネルで 1 組ずつ編集する (T1648) ---

/**
 * パネルの欄の頂点の番号 (1 から数える。メッシュの編集の「Point 3 of 12」と同じ) を、`count` 個の頂点の
 * メッシュの頂点の番号 (0 から) にする。空・整数でない・範囲の外なら null。
 */
export function pairVertex(value: number | null, count: number): number | null {
  if (value === null || !Number.isInteger(value) || value < 1 || value > count) return null;
  return value - 1;
}

/** 組 `(a, b)` がもうあるか (足すボタンを押せなくする)。 */
export function hasPair(glue: Pick<Glue, "pairs">, a: number, b: number): boolean {
  return glue.pairs.some((p) => p.a === a && p.b === b);
}
