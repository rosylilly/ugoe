// エディタ全体で共有する選択状態。ツリー (T0602)・ビューポートのツール (T0606 / T0607)・
// インスペクタ (T0608) は、すべてこのストアを読み書きする。選択はドキュメントの状態ではなく表示の状態なので、
// Undo の対象にしない (docs/editor.md)。

/** 選択できる要素の種類。ID は ugoe_model の各要素の ID。 */
export type ElementKind = "part" | "deformer" | "mesh" | "parameter";

export interface ElementRef {
  kind: ElementKind;
  id: string;
}

const key = (r: ElementRef) => `${r.kind}:${r.id}`;

export class Selection {
  /** 選択中の要素。先頭が「主な選択」(インスペクタに表示するもの)。 */
  items = $state<ElementRef[]>([]);
  /** メッシュごとの選択中の頂点インデックス (メッシュ編集ツール用)。 */
  vertices = $state<Record<string, number[]>>({});

  get primary(): ElementRef | null {
    return this.items[0] ?? null;
  }

  has(ref: ElementRef): boolean {
    return this.items.some((r) => key(r) === key(ref));
  }

  /** 選択を置き換える。 */
  set(refs: ElementRef[]): void {
    this.items = refs;
    this.vertices = {};
  }

  /** 追加選択 (Shift / Ctrl クリック)。すでに選択されていれば外す。 */
  toggle(ref: ElementRef): void {
    this.items = this.has(ref)
      ? this.items.filter((r) => key(r) !== key(ref))
      : [...this.items, ref];
  }

  clear(): void {
    this.items = [];
    this.vertices = {};
  }

  /** ドキュメントの読み直し後、存在しなくなった要素を選択から外す。 */
  retain(exists: (ref: ElementRef) => boolean): void {
    const next = this.items.filter(exists);
    if (next.length !== this.items.length) this.items = next;
  }
}
