// 描画用の複製 (docs/editor.md#描画用の複製) を読む補助。DOM や IPC に依存しない純粋な関数だけを置く
// (drawClone.test.ts で検査する)。規則は ugoe_editor_core::draw_clone と同じ。
import type { Command, Glue, Mesh, Model } from "../../../generated/types";

/** 描画用の複製のグルーの ID の接頭辞 (ugoe_editor_core::draw_clone::GLUE_PREFIX)。 */
export const GLUE_PREFIX = "DrawClone_";

/** 複製 `clone` と元をつなぐグルーの ID。 */
export function glueId(clone: string): string {
  return `${GLUE_PREFIX}${clone}`;
}

/** 描画用の複製のグルーか (ID が mesh_b の glueId)。 */
export function isLink(glue: Glue): boolean {
  return glue.id === glueId(glue.mesh_b);
}

/** 複製 `clone` の元のメッシュの ID。複製でなければ null。 */
export function originalOf(model: Model, clone: string): string | null {
  return model.glues.find((g) => g.mesh_b === clone && isLink(g))?.mesh_a ?? null;
}

/** メッシュ `original` の複製の ID (グルーの順)。 */
export function clonesOf(model: Model, original: string): string[] {
  return model.glues.filter((g) => g.mesh_a === original && isLink(g)).map((g) => g.mesh_b);
}

function sameArray<T>(a: T[], b: T[], eq: (x: T, y: T) => boolean): boolean {
  return a.length === b.length && a.every((x, i) => eq(x, b[i] as T));
}

const samePoint = (a: [number, number], b: [number, number]) => a[0] === b[0] && a[1] === b[1];

/** 格子のすべてのセル (ブレンドシェイプ格子を含む)。 */
function allKeyforms<T>(grid: { keyforms: T[]; blend_shapes?: { keyforms: T[] }[] }): T[] {
  return [...grid.keyforms, ...(grid.blend_shapes ?? []).flatMap((g) => g.keyforms)];
}

/**
 * 複製 `clone` が元と食い違っていないか (ugoe_editor_core::draw_clone::is_in_sync)。複製でなければ false。
 * 食い違っていれば sync_draw_clone で直せる。
 */
export function isInSync(model: Model, clone: string): boolean {
  const index = model.glues.findIndex((g) => g.mesh_b === clone && isLink(g));
  const glue = model.glues[index];
  if (!glue) return false;
  const original = model.meshes.find((m) => m.id === glue.mesh_a);
  const copy = model.meshes.find((m) => m.id === clone);
  if (!original || !copy) return false;
  const n = original.uvs.length;
  return (
    sameArray(copy.uvs, original.uvs, samePoint) &&
    sameArray(copy.indices, original.indices, (a, b) => a === b) &&
    copy.texture === original.texture &&
    (copy.parent ?? null) === (original.parent ?? null) &&
    allKeyforms(copy.keyforms).every((k) => k.positions.length === n) &&
    glue.pairs.length === n &&
    glue.pairs.every((p, i) => p.a === i && p.b === i && p.weight === 0) &&
    allKeyforms(glue.keyforms).every((k) => k.intensity === 1) &&
    !model.glues.slice(index + 1).some((g) => g.mesh_a === glue.mesh_a || g.mesh_b === glue.mesh_a)
  );
}

/** インスペクタに出す、選択中のメッシュと複製の関係。 */
export type DrawCloneInfo =
  | { role: "clone"; original: Mesh | null; inSync: boolean }
  | { role: "original"; clones: Mesh[] };

/** メッシュ `id` の複製の関係。複製なら元を、そうでなければ (0 個以上の) 複製を返す。 */
export function drawCloneInfo(model: Model, id: string): DrawCloneInfo {
  const original = originalOf(model, id);
  if (original !== null) {
    return {
      role: "clone",
      original: model.meshes.find((m) => m.id === original) ?? null,
      inSync: isInSync(model, id),
    };
  }
  const ids = clonesOf(model, id);
  return { role: "original", clones: model.meshes.filter((m) => ids.includes(m.id)) };
}

/** 複製を作るコマンド (ID と名前はバックエンドが決める)。 */
export function createCommand(source: string): Command {
  return { op: "create_draw_clone", source, name: "" };
}

/** 作る前後のモデルから、新しくできた複製の ID を探す (作った後に選択するため)。 */
export function newClone(before: Model, after: Model, source: string): string | null {
  const root = originalOf(before, source) ?? source;
  const old = new Set(clonesOf(before, root));
  return clonesOf(after, root).find((id) => !old.has(id)) ?? null;
}
