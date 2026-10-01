// グルーの編集 (docs/editor.md#グルーの編集) の補助。DOM や IPC に依存しない純粋な関数だけを置く (glues.test.ts で検査する)。
// 頂点の組を作るのは editor-core (`create_glue`・`set_glue` の `distance`)。ここでは送るコマンドを組み立てるだけ。
import type { Command, Glue, ModuleStatus } from "../../ipc";
import { cellAt } from "../inspector/keyforms/keyforms";

/** 頂点の組を作るときの既定の距離 (キャンバスのピクセル)。 */
export const DEFAULT_GLUE_DISTANCE = 8;

/** 描画用の複製のグルーの ID の接頭辞 (editor-core の `draw_clone::GLUE_PREFIX`)。 */
const DRAW_CLONE_PREFIX = "DrawClone_";

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
  if (bindings.length === 0) return keyforms.map(() => "Intensity");
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
