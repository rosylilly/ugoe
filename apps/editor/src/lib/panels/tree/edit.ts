// ツリーの編集操作を Command に変換する (Svelte に依存しない純粋な処理。テストは edit.test.ts)。
// バックエンド (ugoe_editor_core::Document::apply) も検証するが、ドロップ先の可否は
// ドラッグ中に示したいので、循環などはここで先に弾く。
//
// 親の付け替えは生の Command (set_*_parent など) を送るだけで、見た目は保たない
// (座標は新しい親の局所座標として解釈される)。見た目を保つ付け替えは T0607。
import type { EditCommand } from "../../ipc";
import type { ElementRef } from "../../selection.svelte";
import { ancestors, type ModelIndex, meshDrawOrder, type TreeView } from "./build";

/** 節への相対的な落とし位置。`into` は親の付け替え、`above` / `below` はメッシュの並べ替え。 */
export type DropPosition = "into" | "above" | "below";

/** 落とし先。`target` が null なら最上位 (親なし) に落とす。 */
export interface DropTarget {
  target: ElementRef | null;
  position: DropPosition;
  /** 並べ替えのとき、ツリーで target の親の節 (同じ親の中だけで並べ替える)。 */
  container?: ElementRef | null;
}

/** 失敗の理由。`noop` はすでにそこにある (何も変わらない) こと。 */
/** ドラッグ中、ある行に落とすとどうなるか (行の表示用)。 */
export type DropMark = { position: DropPosition; ok: boolean } | null;

export type EditResult =
  | { ok: true; command: EditCommand }
  | { ok: false; reason: string; noop?: boolean };

const reject = (reason: string): EditResult => ({ ok: false, reason });
const noop = (reason: string): EditResult => ({ ok: false, reason, noop: true });

const label = (index: ModelIndex, r: ElementRef) => {
  const name =
    r.kind === "part"
      ? index.parts.get(r.id)?.name
      : r.kind === "deformer"
        ? index.deformers.get(r.id)?.name
        : r.kind === "mesh"
          ? index.meshes.get(r.id)?.name
          : undefined;
  return `${r.kind} "${name ?? r.id}"`;
};

/** 複数のコマンドを 1 つの Undo 単位にまとめる。 */
export function combine(label: string, commands: EditCommand[]): EditCommand | null {
  if (commands.length === 0) return null;
  if (commands.length === 1) return commands[0] ?? null;
  return { op: "batch", label, commands };
}

/** 1 要素を `target` の中へ移すコマンド。 */
function moveInto(
  index: ModelIndex,
  view: TreeView,
  item: ElementRef,
  target: ElementRef | null,
): EditResult {
  const what = label(index, item);
  if (item.kind === "part") {
    if (target === null) {
      if (index.parts.get(item.id)?.parent === null) return noop(`${what} is already at the top`);
      return { ok: true, command: { op: "set_part_parent", id: item.id, parent: null } };
    }
    if (target.kind !== "part") return reject(`a part can only be moved into another part`);
    if (ancestors(index.parts, target.id).includes(item.id)) {
      return reject(`cannot move ${what} into itself or its descendant`);
    }
    if (index.parts.get(item.id)?.parent === target.id) {
      return noop(`${what} is already in ${label(index, target)}`);
    }
    return { ok: true, command: { op: "set_part_parent", id: item.id, parent: target.id } };
  }

  if (item.kind === "deformer") {
    const d = index.deformers.get(item.id);
    if (target === null) {
      if (view === "parts") return reject(`a deformer must belong to a part`);
      if (d?.parent === null) return noop(`${what} is already at the top`);
      return { ok: true, command: { op: "set_deformer_parent", id: item.id, parent: null } };
    }
    if (target.kind === "part") {
      if (d?.part === target.id) return noop(`${what} is already in ${label(index, target)}`);
      return { ok: true, command: { op: "set_deformer_part", id: item.id, part: target.id } };
    }
    if (target.kind !== "deformer") {
      return reject(`a deformer can only be moved into a part or another deformer`);
    }
    if (ancestors(index.deformers, target.id).includes(item.id)) {
      return reject(`cannot move ${what} into itself or its descendant`);
    }
    if (d?.parent === target.id) return noop(`${what} is already under ${label(index, target)}`);
    return { ok: true, command: { op: "set_deformer_parent", id: item.id, parent: target.id } };
  }

  if (item.kind === "mesh") {
    const m = index.meshes.get(item.id);
    if (target === null) {
      if (view === "parts") return reject(`a mesh must belong to a part`);
      if (m?.parent === null) return noop(`${what} is already at the top`);
      return { ok: true, command: { op: "set_mesh_parent", id: item.id, parent: null } };
    }
    if (target.kind === "part") {
      if (m?.part === target.id) return noop(`${what} is already in ${label(index, target)}`);
      return { ok: true, command: { op: "set_mesh_properties", id: item.id, part: target.id } };
    }
    if (target.kind === "deformer") {
      if (m?.parent === target.id) return noop(`${what} is already under ${label(index, target)}`);
      return { ok: true, command: { op: "set_mesh_parent", id: item.id, parent: target.id } };
    }
    return reject(`a mesh can only be moved into a part or a deformer`);
  }

  return reject(`${item.kind} cannot be moved in the tree`);
}

/**
 * メッシュ `item` を `target` の手前 (`above`) または奥 (`below`) へ並べ替えるコマンド。
 * ReorderMesh は `meshes` 配列の順序 (描画順が同じメッシュどうしの前後) だけを変えるので、
 * 描画順が違うメッシュどうしは並べ替えられない。
 */
function reorder(
  index: ModelIndex,
  item: ElementRef,
  target: ElementRef,
  position: "above" | "below",
): EditResult {
  if (item.kind !== "mesh" || target.kind !== "mesh") {
    return reject(`only meshes can be reordered`);
  }
  if (item.id === target.id) return reject(`drop onto another mesh to reorder`);
  const a = meshDrawOrder(index, item.id);
  const b = meshDrawOrder(index, target.id);
  if (a !== b) {
    return reject(
      `${label(index, item)} (draw order ${a}) and ${label(index, target)} (draw order ${b}) ` +
        `cannot be reordered: change the draw order instead (Inspector > Draw order, set_mesh_draw_order)`,
    );
  }
  // 手前 = `meshes` で後ろ。above なら target の直後、below なら target の直前へ。
  const order = index.model.meshes.map((m) => m.id).filter((id) => id !== item.id);
  const at = order.indexOf(target.id);
  const before = position === "above" ? (order[at + 1] ?? null) : target.id;
  const current = index.model.meshes.map((m) => m.id);
  const next = current[current.indexOf(item.id) + 1] ?? null;
  if (next === before) return noop(`${label(index, item)} is already there`);
  return { ok: true, command: { op: "reorder_mesh", id: item.id, before } };
}

/**
 * ドラッグした要素 (複数可) を落としたときのコマンド。落とせなければ理由を返す。
 * 複数の要素の付け替えは 1 つの Batch (1 回の Undo) にまとめる。並べ替えは 1 要素だけ。
 */
export function dropCommand(
  index: ModelIndex,
  view: TreeView,
  items: ElementRef[],
  drop: DropTarget,
): EditResult {
  if (items.length === 0) return reject("nothing to move");
  const { target, position } = drop;

  if (position !== "into") {
    if (target === null) return reject("drop onto a mesh to reorder");
    if (items.length !== 1) return reject("reorder one mesh at a time");
    const [item] = items as [ElementRef];
    if (item.kind !== "mesh") return reject("only meshes can be reordered");
    const m = index.meshes.get(item.id);
    const container = drop.container ?? null;
    // 並べ替えは同じ親の中だけ。別の親のメッシュの間に落としても付け替えはしない (意図が曖昧なため)。
    const sameContainer =
      view === "parts"
        ? container?.kind === "part" && m?.part === container.id
        : (m?.parent ?? null) === (container?.id ?? null);
    if (!sameContainer) {
      return reject(
        `drop onto the ${view === "parts" ? "part" : "deformer"} to move the mesh first`,
      );
    }
    return reorder(index, item, target, position);
  }

  const commands: EditCommand[] = [];
  for (const item of items) {
    if (target && item.kind === target.kind && item.id === target.id) {
      return reject(`cannot move ${label(index, item)} into itself`);
    }
    const r = moveInto(index, view, item, target);
    if (!r.ok) {
      // 複数のうち、すでにそこにある要素は飛ばす。
      if (items.length > 1 && r.noop) continue;
      return r;
    }
    commands.push(r.command);
  }
  const command = combine(`Move ${commands.length} elements`, commands);
  return command ? { ok: true, command } : noop("already there");
}

export function renameCommand(ref: ElementRef, name: string): EditCommand | null {
  switch (ref.kind) {
    case "part":
      return { op: "rename_part", id: ref.id, name };
    case "deformer":
      return { op: "rename_deformer", id: ref.id, name };
    case "mesh":
      return { op: "rename_mesh", id: ref.id, name };
    default:
      return null;
  }
}

/** 選択した要素の削除 (子を先に消すよう、子孫側から並べる)。子が残っていればバックエンドが拒否する。 */
export function deleteCommand(index: ModelIndex, refs: ElementRef[]): EditCommand | null {
  const depth = (r: ElementRef) =>
    r.kind === "part"
      ? ancestors(index.parts, r.id).length
      : r.kind === "deformer"
        ? ancestors(index.deformers, r.id).length
        : 0;
  // メッシュ → デフォーマ → パーツの順に、深いものから消す。
  const rank = { mesh: 0, deformer: 1, part: 2, parameter: 3 } as const;
  const sorted = refs
    .filter((r) => r.kind !== "parameter")
    .sort((a, b) => rank[a.kind] - rank[b.kind] || depth(b) - depth(a));
  const commands: EditCommand[] = sorted.map((r) =>
    r.kind === "part"
      ? { op: "remove_part", id: r.id }
      : r.kind === "deformer"
        ? { op: "remove_deformer", id: r.id }
        : { op: "remove_mesh", id: r.id },
  );
  return combine(`Delete ${commands.length} elements`, commands);
}

/** `base`、`base2`、`base3` … のうち、まだ使われていない最初のもの。 */
export function uniqueId(taken: (id: string) => boolean, base: string): string {
  if (!taken(base)) return base;
  for (let i = 2; ; i++) {
    const id = `${base}${i}`;
    if (!taken(id)) return id;
  }
}

/** 新しいパーツを `parent` の下に作るコマンド (不透明度 1 の定数キーフォーム)。 */
export function addPartCommand(index: ModelIndex, parent: string | null): EditCommand {
  const id = uniqueId((id) => index.parts.has(id), "Part");
  return {
    op: "add_part",
    part: { id, name: id, parent, keyforms: { bindings: [], keyforms: [{ opacity: 1 }] } },
  };
}

/** 選択から、新しいパーツの親にするパーツ (パーツならそれ、メッシュ / デフォーマなら所属パーツ)。 */
export function parentPartFor(index: ModelIndex, ref: ElementRef | null): string | null {
  if (!ref) return null;
  if (ref.kind === "part") return index.parts.has(ref.id) ? ref.id : null;
  if (ref.kind === "deformer") return index.deformers.get(ref.id)?.part ?? null;
  if (ref.kind === "mesh") return index.meshes.get(ref.id)?.part ?? null;
  return null;
}
