// ビューポートのツールのコンポーネントのテスト (docs/testing.md#エディタの部品のテスト) で使う道具。
// happy-dom の上で部品を描き、偽の ViewportController・EditorDocument・WebPuppet を渡してポインタの操作を送る。
//
// 偽のモデル (`testModel`) とビューの変換 (`TRANSFORM`) は次のとおり:
// - パラメータ AngleX (-30..30、既定 0)。
// - メッシュ Quad: 頂点 4 つ (キャンバス座標 (0,0) (100,0) (100,100) (0,100))、AngleX の 3 つのキー。
// - ワープ Warp: 1×1 の格子 (点 4 つ、Quad と同じ位置)、軸なし。
// - 回転 Rot (名前 Neck): 原点 (50,50)・角度 0・拡大率 1、軸なし。
// - グルー Seam: Quad の頂点 1 と 0 の組 1 つ。
// - キャンバス座標 → CSS ピクセル: `(10, 20) + 2 * p`。happy-dom の getBoundingClientRect は 0 なので、
//   イベントの clientX / clientY がそのままツールの座標になる。
import { type Component, flushSync, mount, tick, unmount } from "svelte";
import { afterEach, vi } from "vitest";
import { type EditorDocument, type ParameterDef, parseModelJson } from "../../../document.svelte";
import type {
  ApplyReport,
  Command,
  Expression,
  Motion,
  PhysicsSettings,
  RederiveWarning,
} from "../../../ipc";
import type { ViewportController, ViewTransform } from "../../controller.svelte";
import type { WebPuppet } from "../../wasm";
import type { Point } from "../geometry";
import { revealPanOf } from "../keys";

const mounted: (() => void)[] = [];

afterEach(() => {
  for (const cleanup of mounted.splice(0)) cleanup();
});

/** 部品を body に描く。テストの後で片付ける。 */
// biome-ignore lint/suspicious/noExplicitAny: 部品ごとに props の型が違う
export function render<P extends Record<string, any>>(component: Component<P>, props: P) {
  const target = document.createElement("div");
  document.body.append(target);
  const instance = mount(component, { target, props });
  flushSync();
  mounted.push(() => {
    unmount(instance);
    target.remove();
  });
  return { target, instance };
}

export const TRANSFORM: ViewTransform = { scale: 2, offsetX: 10, offsetY: 20 };

/** キャンバス座標 → CSS ピクセル (`TRANSFORM`)。 */
export function screenOf(p: Point): Point {
  return [TRANSFORM.offsetX + TRANSFORM.scale * p[0], TRANSFORM.offsetY + TRANSFORM.scale * p[1]];
}

export const QUAD: Point[] = [
  [0, 0],
  [100, 0],
  [100, 100],
  [0, 100],
];

/** テストのモデルの JSON (ugoe_model::Model のうち、ツールが読む部分)。 */
export function testModel(): string {
  const grid = (keyform: Record<string, unknown>) => ({ bindings: [], keyforms: [keyform] });
  return JSON.stringify({
    parameters: [{ id: "AngleX", name: "Angle X", min: -30, max: 30, default: 0 }],
    parts: [],
    textures: [],
    glues: [
      {
        id: "Seam",
        mesh_a: "Quad",
        mesh_b: "Quad",
        pairs: [{ a: 1, b: 0, weight: 1 }],
        keyforms: grid({ intensity: 1 }),
      },
    ],
    deformers: [
      {
        id: "Warp",
        name: "Warp",
        parent: null,
        kind: { type: "warp", rows: 1, cols: 1, keyforms: grid({ points: QUAD }) },
      },
      {
        id: "Rot",
        name: "Neck",
        parent: null,
        kind: {
          type: "rotation",
          keyforms: grid({
            origin: [50, 50],
            angle: 0,
            scale: 1,
            reflect_x: false,
            reflect_y: false,
          }),
        },
      },
    ],
    meshes: [
      {
        id: "Quad",
        name: "Quad",
        parent: null,
        texture: 0,
        uvs: QUAD,
        indices: [0, 1, 2, 0, 2, 3],
        blend: "normal",
        culling: false,
        masks: [],
        mask_mode: "and",
        mask_only: false,
        keyforms: {
          bindings: [{ parameter: "AngleX", keys: [-30, 0, 30] }],
          keyforms: [{ positions: QUAD }, { positions: QUAD }, { positions: QUAD }],
        },
      },
    ],
  });
}

/** WebPuppet の代わり。評価結果はモデルの形そのまま (パラメータによらない)。 */
export function fakePuppet(): WebPuppet {
  const flat = (points: Point[]) => Float32Array.from(points.flat());
  const puppet = {
    drawable_ids: () => ["Quad"],
    drawable_positions: (i: number) => (i === 0 ? flat(QUAD) : undefined),
    hit_test_all: () => Uint32Array.from([0]),
    warp_points: (id: string) => (id === "Warp" ? flat(QUAD) : undefined),
    rotation_frame: (id: string) =>
      id === "Rot" ? Float32Array.from([50, 50, 1, 0, 0, 1]) : undefined,
    // 取り付け先のデフォーマの写像: Rot は (20, 10) の平行移動、ほかは恒等写像。
    deformer_point: (id: string, x: number, y: number) =>
      Float32Array.from(id === "Rot" ? [x + 20, y + 10] : [x, y]),
    deformer_jacobian: () => Float32Array.from([1, 0, 0, 1]),
    physics_particles: () => new Float32Array(),
    has_model: () => true,
    attached_modules: (): string[] => [],
    physics_json: (): string | undefined => undefined,
  };
  return puppet as unknown as WebPuppet;
}

/** ViewportController の代わり (ツールが読む状態と座標の換算だけ)。 */
export class FakeController {
  status = $state<"loading" | "ready">("ready");
  /** 読み込めないときの説明 (プレビューのパネルが出す)。 */
  message = $state<string | null>(null);
  /** モデルがライティングを使うか (プレビューのパネルが光源の節を出す)。 */
  usesLighting = $state(false);
  transform = $state<ViewTransform | null>({ ...TRANSFORM });
  parameterValues = $state<Record<string, number>>({});
  animationFrame = $state(0);
  previewingPhysics = $state(false);
  showDepth = $state(false);
  posePreview = $state.raw<readonly string[] | null>(null);
  puppet: WebPuppet | null = fakePuppet();

  canvasToScreen(x: number, y: number): Point | null {
    const t = this.transform;
    return t ? [t.offsetX + t.scale * x, t.offsetY + t.scale * y] : null;
  }

  screenToCanvas(x: number, y: number): Point | null {
    const t = this.transform;
    return t ? [(x - t.offsetX) / t.scale, (y - t.offsetY) / t.scale] : null;
  }

  setParameter(id: string, value: number): void {
    this.parameterValues[id] = value;
  }

  resetParameters(): void {
    this.parameterValues = {};
  }

  setPosePreview(parts: readonly string[] | null): void {
    this.posePreview = parts;
  }

  stopPhysicsPreview(): void {
    this.previewingPhysics = false;
  }

  /** ビューを寄せる (着せ替えの違いの強調の「Fit」)。呼ばれた引数を記録する。 */
  zoomAt = vi.fn((_factor: number, _x: number, _y: number): void => {});
  panBy = vi.fn((_dx: number, _dy: number): void => {});
  /** ビューポート要素の大きさ (CSS ピクセル)。null なら `reveal` は動かさない (ViewportController の resize の前と同じ)。 */
  size: [number, number] | null = null;

  /** `]` `[` で選んだ点を見せる (ViewportController.reveal と同じく、動かす量を `panBy` に渡す)。 */
  reveal(x: number, y: number): void {
    const delta = this.size ? revealPanOf([x, y], this.size) : null;
    if (delta) this.panBy(delta[0], delta[1]);
  }

  get asController(): ViewportController {
    return this as unknown as ViewportController;
  }
}

/** EditorDocument の代わり。`apply`・`send` で送られたコマンドを `commands` に記録する。 */
export class FakeDocument {
  model = $state.raw<{ json: string; bundle: Uint8Array | null } | null>({
    json: testModel(),
    bundle: null,
  });
  /** 解析したモデル (`EditorDocument.parsedModel` と同じく `model.json` から作る)。 */
  readonly parsedModel = $derived(parseModelJson(this.model?.json));
  physics = $state.raw<PhysicsSettings | null>(null);
  /** パラメータの一覧 (`EditorDocument.parameters`)。テストが必要なら入れる。 */
  parameters = $state.raw<ParameterDef[]>([]);
  motions = $state.raw<Motion[]>([]);
  expressions = $state.raw<Expression[]>([]);
  warnings = $state<RederiveWarning[]>([]);
  module = $state.raw(null);
  error = $state<string | null>(null);
  /** 読み直しの回数 (`EditorDocument.revision`)。テストが増やして編集の後の読み直しの代わりにする。 */
  revision = $state(0);
  readonly commands: Command[] = [];
  /** 設定すると、`apply` はこの約束が解けるまで終わらない (送っている間の操作を試す)。 */
  pending: Promise<void> | null = null;
  /**
   * 設定すると、`apply` は待ち終えた後でこれを呼んで写し (`model`・`motions`) を書き換える (本物の読み直しの代わり)。
   * false を返すと適用に失敗したことにする。
   */
  onApply: ((command: Command) => boolean | undefined) | null = null;
  /** `send` の列の末尾。 */
  #queue: Promise<unknown> = Promise.resolve();

  /** 本物の `apply` と同じく、適用できたら報告を返す (失敗は null。部品は戻り値で成否を見る)。 */
  apply = vi.fn(async (command: Command): Promise<ApplyReport | null> => {
    this.commands.push(command);
    if (this.pending) await this.pending;
    if (this.onApply?.(command) === false) return null;
    return { warnings: [], mirrored: [] };
  });

  /**
   * 本物の `send` と同じく、前に並べた送信が終わってから (読み直した後の写しで) `build` を呼んで組み立て、
   * `apply` で送る。null なら送らない。数値の欄の確定を畳む動きは真似しない。
   */
  send = vi.fn((build: () => Command | null): Promise<ApplyReport | null> => {
    const result = this.#queue.then(async () => {
      await tick();
      const command = build();
      return command ? this.apply(command) : null;
    });
    this.#queue = result.catch(() => undefined);
    return result;
  });

  dismissWarnings(): void {
    this.warnings = [];
  }

  get asDocument(): EditorDocument {
    return this as unknown as EditorDocument;
  }
}

/** 外から解ける約束。 */
export function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve = () => {};
  const promise = new Promise<void>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

type PointerKind = "pointerdown" | "pointermove" | "pointerup" | "pointercancel";

/** ポインタのイベントを送り、Svelte の更新を流す。 */
export function pointer(
  el: Element,
  type: PointerKind,
  [x, y]: Point,
  init: PointerEventInit = {},
): PointerEvent {
  const event = new PointerEvent(type, {
    bubbles: true,
    cancelable: true,
    clientX: x,
    clientY: y,
    pointerId: 1,
    button: 0,
    ...init,
  });
  el.dispatchEvent(event);
  flushSync();
  return event;
}

/** 押す → 動かす (途中の点を順に) → 離す。`up` に "pointercancel" を渡すと取り消す。 */
export function drag(
  el: Element,
  path: Point[],
  init: PointerEventInit = {},
  up: "pointerup" | "pointercancel" = "pointerup",
): void {
  const [first, ...rest] = path;
  if (!first) return;
  pointer(el, "pointerdown", first, init);
  for (const p of rest) pointer(el, "pointermove", p, init);
  pointer(el, up, rest.at(-1) ?? first, init);
}

/** キーを押す (keydown を `target` から浮上させ、Svelte の更新を流す)。 */
export function press(target: Element, key: string, init: KeyboardEventInit = {}): KeyboardEvent {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  flushSync();
  return event;
}

/** body に置いたダイアログの中のボタン (ダイアログの中のキー操作を試す)。テストの後で片付ける。 */
export function dialogButton(): HTMLButtonElement {
  const dialog = document.createElement("div");
  dialog.setAttribute("role", "dialog");
  const button = document.createElement("button");
  dialog.append(button);
  document.body.append(dialog);
  mounted.push(() => dialog.remove());
  return button;
}

/** 約束の後始末 (マイクロタスク) と Svelte の更新を流す。 */
export async function settle(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
  flushSync();
}

/** `selector` に合う要素 (無ければ例外)。 */
export function get<E extends Element = Element>(root: ParentNode, selector: string): E {
  const el = root.querySelector<E>(selector);
  if (!el) throw new Error(`not found: ${selector}`);
  return el;
}

/** `selector` に合う要素すべて。 */
export function all<E extends Element = Element>(root: ParentNode, selector: string): E[] {
  return Array.from(root.querySelectorAll<E>(selector));
}
