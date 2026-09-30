// Web Component の `<ugoe-avatar>`。モデルを 1 体読み込み、shadow DOM の canvas に描き続ける。
//
// 属性:
// - `src`: モデルの URL (`.ugoe` バンドルかモデルの JSON)
// - `backend`: `auto` (既定) | `webgpu` | `webgl2`。`auto` で WebGPU の作成に失敗したら、新しい canvas の WebGL2 で作り直す
// - `parameters`: `"AngleX: 15; EyeLOpen: 0.5"` (読み込みのたびと、属性を変えたときに設定する)
// - `expression` / `motion` (+ `motion-priority`): バンドルの表情・モーションの名前
// - `paused`: 時間を止める (`update(dt)` を呼ばない。パラメータを変えたときだけ描き直す)
//
// JS からは `setParameter` / `parameters` / `load` と、ugoe-wasm の `WebPuppet` そのもの (`puppet`) を使う。
// 画面外 (IntersectionObserver) と `paused` の間は requestAnimationFrame を回さない。

import { parseBackend, parseMotionPriority, parseParameters } from "./attributes.js";
import { createPuppet, loadModel } from "./runtime.js";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** `load` イベントの `detail`。 */
export interface UgoeAvatarLoadDetail {
  /** 読み込みの警告の文 (英語)。構造は `puppet.load_warnings_json()`。 */
  warnings: string[];
  /** 描画に使っている API (`"webgpu"` | `"webgl2"`)。 */
  backend: string;
}

/** モデルのパラメータの定義 (`WebPuppet.parameters_json()` の要素。モデルの JSON 形式と同じ)。 */
export interface UgoeParameterDefinition {
  id: string;
  name: string;
  /** 中継用 (内部) のパラメータ (docs/spec/model.md#パラメータ)。 */
  internal?: boolean;
  min: number;
  max: number;
  default: number;
  [key: string]: unknown;
}

type ModelData = ArrayBuffer | Uint8Array | string;

/** GPU を失ったときに作り直す回数の上限 (作り直してもすぐ失うときに止めるため)。 */
const MAX_RESTARTS = 3;
/** 1 フレームで進める時間の上限 (秒)。タブが裏にあった後などに物理が飛ばないようにする。 */
const MAX_DT = 0.1;

const STYLE = `
:host { display: inline-block; position: relative; width: 300px; height: 300px; contain: content; }
:host([hidden]) { display: none; }
canvas { display: block; width: 100%; height: 100%; }
`;

// Node (SSR) で import しても落ちないように、HTMLElement が無ければ空のクラスを継承する。
const BaseElement: typeof HTMLElement =
  globalThis.HTMLElement ?? (class {} as unknown as typeof HTMLElement);

interface Deferred<T> {
  promise: Promise<T>;
  resolve(value: T): void;
  settled: boolean;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  const d: Deferred<T> = {
    promise,
    settled: false,
    resolve(value) {
      d.settled = true;
      resolve(value);
    },
  };
  return d;
}

/** `<ugoe-avatar>` の実装。`defineUgoeAvatar()` か `@ugoe/runtime/element` の import で登録する。 */
export class UgoeAvatarElement extends BaseElement {
  static readonly observedAttributes = [
    "src",
    "backend",
    "parameters",
    "expression",
    "motion",
    "motion-priority",
    "paused",
  ];

  #root: ShadowRoot;
  #canvas: HTMLCanvasElement | null = null;
  #puppet: WebPuppet | null = null;
  #ready = deferred<WebPuppet>();
  /** 接続・切断のたびに増やす。古い非同期の処理の結果を捨てるため。 */
  #session = 0;
  /** `src` の取得のたびに増やす。古い取得の結果を捨てるため。 */
  #fetch = 0;
  /** 最後に読み込んだ (または読み込みを待っている) モデル。作り直したときに読み直す。 */
  #model: ModelData | null = null;
  #warnings: string[] = [];
  /** 作り直す前のパラメータの値。作り直した後の読み込みで戻す (`backend` の変更・GPU を失ったとき)。 */
  #carry: Record<string, number> | null = null;
  /** 描画バッファの大きさ (画素)。 */
  #size: [number, number] = [0, 0];
  #frame = 0;
  #last: number | null = null;
  #needsRender = false;
  #visible = true;
  #restarts = 0;
  #resizeObserver: ResizeObserver | null = null;
  #intersectionObserver: IntersectionObserver | null = null;

  constructor() {
    super();
    this.#root = this.attachShadow({ mode: "open" });
    const style = document.createElement("style");
    style.textContent = STYLE;
    this.#root.append(style);
  }

  // --- 属性とプロパティ ---

  /** モデルの URL (`src` 属性)。 */
  get src(): string {
    return this.getAttribute("src") ?? "";
  }
  set src(value: string) {
    this.setAttribute("src", value);
  }

  /** 時間を止めているか (`paused` 属性)。 */
  get paused(): boolean {
    return this.hasAttribute("paused");
  }
  set paused(value: boolean) {
    this.toggleAttribute("paused", value);
  }

  /** ugoe-wasm の `WebPuppet` (作成前と切断中は `null`)。直接操作したら `requestRender()` を呼ぶ。 */
  get puppet(): WebPuppet | null {
    return this.#puppet;
  }

  /** `WebPuppet` ができたら解決する。切断して繋ぎ直すと新しい `WebPuppet` を待つ Promise に変わる。 */
  get ready(): Promise<WebPuppet> {
    return this.#ready.promise;
  }

  /** 描画に使っている API (`"webgpu"` | `"webgl2"`)。作成前は `null`。 */
  get backend(): string | null {
    return this.#puppet?.backend() ?? null;
  }

  /** 最後の読み込みの警告の文。 */
  get warnings(): readonly string[] {
    return this.#warnings;
  }

  /** パラメータの定義 (値域・既定値)。モデルが無ければ空。 */
  get parameterDefinitions(): UgoeParameterDefinition[] {
    if (!this.#puppet) return [];
    return (JSON.parse(this.#puppet.parameters_json()) as { parameters: UgoeParameterDefinition[] })
      .parameters;
  }

  /** パラメータの今の値 (ID → 値)。代入すると、含まれるパラメータだけを設定する。 */
  get parameters(): Record<string, number> {
    const puppet = this.#puppet;
    const values: Record<string, number> = {};
    if (!puppet) return values;
    for (const id of puppet.parameter_ids()) {
      const v = puppet.parameter(id);
      if (v !== undefined) values[id] = v;
    }
    return values;
  }
  set parameters(values: Record<string, number> | Map<string, number>) {
    const entries = values instanceof Map ? values.entries() : Object.entries(values);
    for (const [id, v] of entries) this.setParameter(id, v);
  }

  /** パラメータを設定する (値域にクランプされる)。モデルが無いか ID が無ければ `false`。 */
  setParameter(id: string, value: number): boolean {
    const ok = this.#puppet?.set_parameter(id, value) ?? false;
    if (ok) this.requestRender();
    return ok;
  }

  /** パラメータの今の値。モデルが無いか ID が無ければ `undefined`。 */
  getParameter(id: string): number | undefined {
    return this.#puppet?.parameter(id);
  }

  /**
   * モデルを読み込む (`src` 属性の代わりに、バイト列や JSON の文字列を渡す)。戻り値は警告の文。
   * 接続前に呼ぶと、`WebPuppet` ができてから読み込む。
   */
  async load(data: ModelData): Promise<string[]> {
    this.#fetch++; // 取得中の `src` があっても、こちらを優先する
    this.#model = data;
    if (this.#puppet) return this.#applyModelOrThrow();
    await this.ready;
    return this.#warnings;
  }

  /** 次のフレームで描き直す (`puppet` を直接操作したとき用)。 */
  requestRender(): void {
    this.#needsRender = true;
    this.#schedule();
  }

  connectedCallback(): void {
    void this.#start();
  }

  disconnectedCallback(): void {
    this.#stop();
  }

  attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {
    // 開始前 (パーサーが作った要素の昇格など) は `#start` がまとめて適用する。
    if (oldValue === newValue || !this.#canvas) return;
    switch (name) {
      case "src":
        void this.#fetchSrc();
        break;
      case "backend":
        this.#recreate();
        break;
      case "parameters":
        this.#applyParameters();
        break;
      case "expression":
        this.#guard(() => this.#applyExpression());
        break;
      case "motion":
      case "motion-priority":
        this.#guard(() => this.#applyMotion());
        break;
      case "paused":
        this.#last = null;
        this.requestRender();
        break;
    }
  }

  // --- 作成と破棄 ---

  async #start(): Promise<void> {
    if (this.#canvas) return;
    const session = ++this.#session;
    this.#observe();
    let puppet: WebPuppet;
    try {
      puppet = await this.#create(session);
    } catch (e) {
      if (session === this.#session) this.#dispatchError(e);
      return;
    }
    if (session !== this.#session) {
      puppet.free();
      return;
    }
    this.#puppet = puppet;
    const [w, h] = this.#size;
    if (w > 0 && h > 0) puppet.resize(w, h);
    this.#ready.resolve(puppet);
    if (this.#model !== null) {
      this.#guard(() => this.#applyModelOrThrow());
    } else if (this.src) {
      void this.#fetchSrc();
    }
    this.requestRender();
  }

  /** canvas を作って `WebPuppet` を作る。`auto` で失敗したら、新しい canvas の WebGL2 で作り直す。 */
  async #create(session: number): Promise<WebPuppet> {
    const backend = parseBackend(this.getAttribute("backend"));
    try {
      return await createPuppet(this.#newCanvas(), { backend });
    } catch (e) {
      if (backend !== "auto" || session !== this.#session) throw e;
      console.warn("ugoe-avatar: falling back to WebGL2:", e);
      return createPuppet(this.#newCanvas(), { backend: "webgl2" });
    }
  }

  #newCanvas(): HTMLCanvasElement {
    this.#canvas?.remove();
    const canvas = document.createElement("canvas");
    canvas.setAttribute("part", "canvas");
    const [w, h] = this.#size;
    canvas.width = Math.max(1, w);
    canvas.height = Math.max(1, h);
    this.#root.append(canvas);
    this.#canvas = canvas;
    this.#resizeObserver?.disconnect();
    this.#resizeObserver?.observe(canvas);
    return canvas;
  }

  #observe(): void {
    if (typeof ResizeObserver !== "undefined") {
      this.#resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) this.#onResize(entry);
      });
    }
    if (typeof IntersectionObserver !== "undefined") {
      this.#intersectionObserver = new IntersectionObserver((entries) => {
        const entry = entries[entries.length - 1];
        if (!entry) return;
        this.#visible = entry.isIntersecting;
        this.#last = null;
        this.#schedule();
      });
      this.#intersectionObserver.observe(this);
    }
  }

  #onResize(entry: ResizeObserverEntry): void {
    const box = entry.devicePixelContentBoxSize?.[0];
    const dpr = globalThis.devicePixelRatio ?? 1;
    const w = Math.round(box ? box.inlineSize : entry.contentRect.width * dpr);
    const h = Math.round(box ? box.blockSize : entry.contentRect.height * dpr);
    if (w <= 0 || h <= 0) return;
    this.#size = [w, h];
    this.#puppet?.resize(w, h);
    this.requestRender();
  }

  #stop(): void {
    this.#session++;
    this.#fetch++;
    if (this.#frame) cancelAnimationFrame(this.#frame);
    this.#frame = 0;
    this.#last = null;
    this.#resizeObserver?.disconnect();
    this.#resizeObserver = null;
    this.#intersectionObserver?.disconnect();
    this.#intersectionObserver = null;
    this.#puppet?.free();
    this.#puppet = null;
    this.#canvas?.remove();
    this.#canvas = null;
    if (this.#ready.settled) this.#ready = deferred();
  }

  /** `WebPuppet` を作り直す。モデルは読み直し、パラメータの値は引き継ぐ。 */
  #recreate(): void {
    if (this.#puppet?.has_model()) this.#carry = this.parameters;
    this.#stop();
    void this.#start();
  }

  /** GPU を失ったので作り直す。 */
  #restart(): void {
    if (this.#restarts >= MAX_RESTARTS) {
      this.#stop();
      this.#dispatchError(new Error("the GPU device was lost repeatedly"));
      return;
    }
    this.#restarts++;
    this.#recreate();
  }

  // --- モデル ---

  async #fetchSrc(): Promise<void> {
    const token = ++this.#fetch;
    const src = this.src;
    if (!src) {
      this.#model = null;
      this.#puppet?.unload();
      this.requestRender();
      return;
    }
    try {
      const response = await fetch(new URL(src, document.baseURI));
      if (!response.ok) {
        throw new Error(`failed to fetch ${src}: ${response.status} ${response.statusText}`);
      }
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (token !== this.#fetch) return;
      this.#model = bytes;
      if (this.#puppet) this.#applyModelOrThrow();
    } catch (e) {
      if (token === this.#fetch) this.#dispatchError(e);
    }
  }

  /** `#model` を読み込み、属性の状態 (パラメータ・表情・モーション) を適用して `load` を送る。 */
  #applyModelOrThrow(): string[] {
    const puppet = this.#puppet;
    if (!puppet || this.#model === null) return [];
    const warnings = loadModel(puppet, this.#model);
    this.#warnings = warnings;
    puppet.fit();
    this.#applyParameters();
    if (this.#carry) this.parameters = this.#carry;
    this.#carry = null;
    // 表情・モーションの名前がモデルに無くても、読み込み自体は成功として扱う (`error` を送るだけ)。
    this.#guard(() => this.#applyExpression());
    this.#guard(() => this.#applyMotion());
    this.requestRender();
    this.dispatchEvent(
      new CustomEvent<UgoeAvatarLoadDetail>("load", {
        detail: { warnings, backend: puppet.backend() },
      }),
    );
    return warnings;
  }

  #applyParameters(): void {
    for (const [id, v] of parseParameters(this.getAttribute("parameters"))) {
      this.setParameter(id, v);
    }
  }

  #applyExpression(): void {
    const puppet = this.#puppet;
    if (!puppet?.has_model()) return;
    const name = this.getAttribute("expression");
    if (name) puppet.set_expression(name);
    else puppet.stop_expressions();
    this.requestRender();
  }

  #applyMotion(): void {
    const puppet = this.#puppet;
    if (!puppet?.has_model()) return;
    const name = this.getAttribute("motion");
    if (name) puppet.start_motion(name, parseMotionPriority(this.getAttribute("motion-priority")));
    else puppet.stop_motions();
    this.requestRender();
  }

  // --- 描画のループ ---

  #animating(): boolean {
    return !this.paused && this.#visible && (this.#puppet?.has_model() ?? false);
  }

  #schedule(): void {
    if (this.#frame || !this.#puppet) return;
    if (!this.#needsRender && !this.#animating()) return;
    this.#frame = requestAnimationFrame((now) => this.#tick(now));
  }

  #tick(now: number): void {
    this.#frame = 0;
    const puppet = this.#puppet;
    if (!puppet) return;
    if (puppet.is_lost()) {
      this.#restart();
      return;
    }
    if (this.#animating()) {
      const dt = this.#last === null ? 0 : Math.min((now - this.#last) / 1000, MAX_DT);
      this.#last = now;
      puppet.update(dt);
    } else {
      this.#last = null;
    }
    this.#needsRender = false;
    try {
      puppet.render();
    } catch (e) {
      if (puppet.is_lost()) {
        this.#restart();
        return;
      }
      this.#dispatchError(e);
      return;
    }
    this.#schedule();
  }

  // --- エラー ---

  #guard(f: () => void): void {
    try {
      f();
    } catch (e) {
      this.#dispatchError(e);
    }
  }

  #dispatchError(e: unknown): void {
    const error = e instanceof Error ? e : new Error(String(e));
    this.dispatchEvent(new CustomEvent("error", { detail: { error } }));
  }
}

/**
 * `<ugoe-avatar>` を登録する (登録済みなら何もしない)。別の名前でも登録できる
 * (1 つのクラスは 1 つの名前にしか登録できないので、そのときは派生クラスを登録する)。
 */
export function defineUgoeAvatar(name = "ugoe-avatar"): void {
  if (customElements.get(name)) return;
  customElements.define(
    name,
    name === "ugoe-avatar" ? UgoeAvatarElement : class extends UgoeAvatarElement {},
  );
}

declare global {
  interface HTMLElementTagNameMap {
    "ugoe-avatar": UgoeAvatarElement;
  }
}
