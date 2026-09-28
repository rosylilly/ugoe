// ツールが見る「今ビューポートに描かれているもの」: モデルの要約と、wasm で評価した頂点位置。
//
// 評価後の位置は WebPuppet に問い合わせる (パラメータのプレビューも反映される)。
// WebPuppet は反応性を持たないので、モデル・パラメータ・ビューが変わったら ToolOverlay が
// invalidate() を呼び、それを読む $derived を計算し直させる。
import type { EditorDocument } from "../../document.svelte";
import type { ViewportController } from "../controller.svelte";
import type { Point } from "./geometry";
import { type MeshInfo, type ModelSummary, summarizeModel } from "./model";

export class ViewportScene {
  readonly #controller: ViewportController;
  readonly #doc: EditorDocument;
  /** 評価結果が変わりうるたびに増える。 */
  #tick = $state(0);
  readonly model: ModelSummary = $derived.by(() => summarizeModel(this.#doc.model?.json));
  readonly #index: Map<string, number> = $derived(
    new Map(this.model.meshes.map((m, i) => [m.id, i])),
  );

  constructor(controller: ViewportController, doc: EditorDocument) {
    this.#controller = controller;
    this.#doc = doc;
  }

  invalidate(): void {
    this.#tick += 1;
  }

  mesh(id: string): MeshInfo | null {
    const i = this.#index.get(id);
    return i === undefined ? null : (this.model.meshes[i] ?? null);
  }

  /**
   * メッシュの評価済みの頂点位置 (キャンバス座標、[x0, y0, ...])。
   * WebPuppet がまだ新しいモデルを読んでいない (頂点数が合わない) ときは null。
   */
  canvasPositions(id: string): Float32Array | null {
    void this.#tick;
    const puppet = this.#controller.puppet;
    const i = this.#index.get(id);
    const mesh = this.mesh(id);
    if (!puppet || i === undefined || !mesh || this.#controller.status !== "ready") return null;
    if (puppet.drawable_ids()[i] !== id) return null;
    const positions = puppet.drawable_positions(i);
    return positions && positions.length === mesh.vertices * 2 ? positions : null;
  }

  /** メッシュの評価済みの頂点位置 (CSS ピクセル、[x0, y0, ...])。 */
  screenPositions(id: string): number[] | null {
    const positions = this.canvasPositions(id);
    const t = this.#controller.transform;
    if (!positions || !t) return null;
    return Array.from(positions, (v, k) => (k % 2 === 0 ? t.offsetX : t.offsetY) + t.scale * v);
  }

  /** CSS ピクセルの点にある表示中のメッシュの ID (手前から奥へ)。 */
  hitAll(x: number, y: number): string[] {
    const p = this.#controller.screenToCanvas(x, y);
    const puppet = this.#controller.puppet;
    if (!p || !puppet) return [];
    const ids = puppet.drawable_ids();
    return Array.from(puppet.hit_test_all(p[0], p[1]), (i) => ids[i]).filter(
      (id): id is string => id !== undefined,
    );
  }

  /** CSS ピクセル → キャンバス座標。 */
  toCanvas(x: number, y: number): Point | null {
    return this.#controller.screenToCanvas(x, y);
  }

  /** キャンバス座標での長さ (CSS ピクセルの長さを換算)。 */
  canvasLength(pixels: number): number {
    return pixels / (this.#controller.transform?.scale ?? 1);
  }
}
