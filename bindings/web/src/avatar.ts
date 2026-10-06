// Web Component の `<ugoe-avatar>`。モデルを 1 体読み込み、shadow DOM の canvas に描き続ける。
//
// 属性:
// - `src`: モデルの URL (`.ugoe` バンドルかモデルの JSON)
// - `backend`: `auto` (既定) | `webgpu` | `webgl2`。`auto` で WebGPU の作成に失敗したら、新しい canvas の WebGL2 で作り直す
// - `parameters`: `"AngleX: 15; EyeLOpen: 0.5"` (読み込みのたびと、属性を変えたときに設定する)
// - `expression` / `motion` (+ `motion-priority`): バンドルの表情・モーションの名前。再生中のモーションより優先度が低くて
//   始められないモーションは、始められるまで毎フレーム試す
// - `motion-over-tracking` (+ `tracking-kept`): トラッキング中も、再生中のモーションが動かすパラメータではモーションを
//   優先する (`WebPuppet.set_motion_over_tracking`)。`tracking-kept` に並べた ID はトラッキングのまま (T0722)
// - `paused`: 時間を止める (`update(dt)` を呼ばない。パラメータを変えたときだけ描き直す)
// - `crossorigin`: `src` の取得の資格情報 (`use-credentials` で別のオリジンにも送る。次の取得から効く)
//
// JS からは `setParameter` / `parameters` / `load` / `setTracking` / `restart` と、ugoe-wasm の `WebPuppet` そのもの (`puppet`) を使う。
// 作成に失敗したときと GPU を続けて失って諦めたときは、`error` を送って `ready` を reject する。読み込みの途中に GPU を
// 失ったら (`device_lost`)、`error` は送らずにそのモデルを作り直した後に読む (T0486)。`restart()`・`backend` の変更・
// 繋ぎ直しで作り直す。
// GPU を失って戻るのを待ち始めたら `gpulost` を送る (WebGL2 の復元を待つ・作り直しで作れずに試し直しを待つ)。WebGL2 の
// 復元が `RESTORE_TIMEOUT_MS` 待っても届かなければ、新しい canvas で作り直しを試す (T0938。@ugoe/player の PuppetSurface と同じ)。
// フェイストラッキングはホストが入力を渡す口だけを持つ (トラッカーは持たない。ADR 0025)。
// 画面外 (IntersectionObserver) と `paused` の間は requestAnimationFrame を回さない。

import {
  parseBackend,
  parseCrossOrigin,
  parseIdList,
  parseMotionPriority,
  parseParameters,
} from "./attributes.js";
import type { LoadLimits } from "./limits.js";
import { parameterDefinitions, type UgoeParameterDefinition } from "./queries.js";
import { createPuppet, loadModel } from "./runtime.js";
import { type TrackingInput, trackingArguments } from "./tracking.js";
import { loadWarnings, type UgoeLoadWarning } from "./warnings.js";
import type { WebPuppet } from "./wasm/ugoe_wasm.js";

/** `load` イベントの `detail`。 */
export interface UgoeAvatarLoadDetail {
  /** 読み込みの警告の文 (英語)。ログに出すだけならこれを使う。 */
  warnings: string[];
  /** `warnings` と同じ順・同じ数の構造 (`loadWarnings(puppet)`)。利用者向けに言い換えるときは `code` を使う。 */
  warningDetails: UgoeLoadWarning[];
  /** 描画に使っている API (`"webgpu"` | `"webgl2"`)。 */
  backend: string;
}

/** `error` イベントの `detail`。 */
export interface UgoeAvatarErrorDetail {
  /** 作成・取得・読み込み・表情やモーションの適用・描画の失敗。着せ替えの失敗は `isDressUpError` で見分ける。 */
  error: Error;
}

/**
 * `<ugoe-avatar>` が送るイベント。`addEventListener("load", (e) => e.detail.backend)` のように `detail` が型付けされる。
 * HTMLElement の `load` (`Event`)・`error` (`ErrorEvent`) を `CustomEvent` に置き換えたもの。
 */
export interface UgoeAvatarElementEventMap extends Omit<HTMLElementEventMap, "load" | "error"> {
  load: CustomEvent<UgoeAvatarLoadDetail>;
  error: CustomEvent<UgoeAvatarErrorDetail>;
  /** GPU を失って、戻るのを待ち始めた (`detail` は無い。戻ったら `ready` が解決する)。 */
  gpulost: CustomEvent<null>;
}

type ModelData = ArrayBuffer | Uint8Array | string;

/** GPU を失ったときに続けて作り直す回数の上限 (作り直してもすぐ失うときに止めるため)。 */
const MAX_RESTARTS = 3;
/** 前の作り直しからこれ (ミリ秒) より後に失ったら、続けて失ったとは数えない (スリープからの復帰などで時々失うため)。 */
const RESTART_WINDOW_MS = 60_000;
/**
 * WebGL2 のコンテキストを失ってから復元を待つ時間 (ミリ秒)。届かなければ新しい canvas で作り直しを試す。作り直しで
 * 作れなかったときに、次に試すまで待つ時間も同じ (T0938)。
 */
const RESTORE_TIMEOUT_MS = 5_000;
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

interface LoadWaiter {
  resolve(warnings: string[]): void;
  reject(error: unknown): void;
}

interface Deferred<T> {
  promise: Promise<T>;
  resolve(value: T): void;
  reject(error: unknown): void;
  settled: boolean;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  // 誰も待っていない reject を unhandledrejection にしない (失敗は `error` イベントでも知らせる)
  promise.catch(() => {});
  const d: Deferred<T> = {
    promise,
    settled: false,
    resolve(value) {
      d.settled = true;
      resolve(value);
    },
    reject(error) {
      d.settled = true;
      reject(error);
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
    "motion-over-tracking",
    "tracking-kept",
    "paused",
  ];

  #root: ShadowRoot;
  #canvas: HTMLCanvasElement | null = null;
  #puppet: WebPuppet | null = null;
  #ready = deferred<WebPuppet>();
  /** 接続している (`connectedCallback` から `disconnectedCallback` まで)。 */
  #connected = false;
  /** 作成の失敗か、GPU を続けて失って諦めた理由。作り直しを始めるか切断するまで持つ。 */
  #failure: Error | null = null;
  /** 接続・切断のたびに増やす。古い非同期の処理の結果を捨てるため。 */
  #session = 0;
  /** `src` の取得のたびに増やす。古い取得の結果を捨てるため。 */
  #fetch = 0;
  /** 今の `src` の取得を中断するもの。置き換わったら中断する (切断では中断しない)。 */
  #fetchAbort: AbortController | null = null;
  /** `src` を取得している (切断しても続ける。繋ぎ直したら、古いモデルではなく取得の結果を読む)。 */
  #fetching = false;
  /** 最後に読み込めたモデル。作り直したときに読み直す (読み込めなかったものは覚えない)。 */
  #model: ModelData | null = null;
  /**
   * `WebPuppet` ができたら読み込むモデル (`load` か `src` の取得が、作成の前や作り直しの途中に終わった。読み込みの途中に
   * GPU を失った)。
   */
  #pendingModel: ModelData | null = null;
  /** `WebPuppet` ができる前 (作り直しの途中・読み込みの途中に GPU を失った後) に呼ばれた `load` の結果を待つもの。 */
  #loadWaiters: LoadWaiter[] = [];
  #warnings: string[] = [];
  #warningDetails: UgoeLoadWarning[] = [];
  /** 作り直す前のパラメータの値。作り直した後の読み込みで戻す (`backend` の変更・GPU を失ったとき)。 */
  #carry: Record<string, number> | null = null;
  /** 描画バッファの大きさ (画素)。 */
  #size: [number, number] = [0, 0];
  /** canvas の大きさ (CSS ピクセル。ResizeObserver の最後の知らせ)。devicePixelRatio だけが変わったときに使う。 */
  #cssSize: [number, number] | null = null;
  /** devicePixelRatio の変化を聞いている問い合わせ (`#watchPixelRatio`)。 */
  #pixelRatioQuery: MediaQueryList | null = null;
  #frame = 0;
  #last: number | null = null;
  #needsRender = false;
  #visible = true;
  #restarts = 0;
  /** 最後に作り直した時刻 (`performance.now()`)。 */
  #restartedAt: number | null = null;
  /** WebGL2 のコンテキストを失って、復元を待っている (wgpu は WebGL2 の喪失を `is_lost` で知らせないため、canvas のイベントで知る)。 */
  #contextLost = false;
  /** 復元を待つ時間切れ・作り直しを試し直すまでのタイマー。 */
  #restoreTimer: ReturnType<typeof setTimeout> | null = null;
  /** GPU を失った後の作り直しの途中 (作れなければ待って試し直す)。 */
  #recovering = false;
  /** GPU を失って、戻るのを待っている (`waitingForGpu`)。 */
  #waitingForGpu = false;
  /** 最後に渡されたトラッキングの入力。モデルを読み直したら渡し直す。 */
  #tracking: TrackingInput | null = null;
  /** トラッキングの入力を渡してから、まだ `update` していない (`paused` の間も反映するため)。 */
  #trackingDirty = false;
  /** `motion` 属性のモーションを、再生中のモーションより優先度が低くて始められなかった (始められるまで毎フレーム試す)。 */
  #motionPending = false;
  #loadLimits: LoadLimits | null = null;
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

  /** `src` の取得の資格情報 (`crossorigin` 属性。`"use-credentials"` で別のオリジンにも送る)。`null` で属性を消す。 */
  get crossOrigin(): string | null {
    return this.getAttribute("crossorigin");
  }
  set crossOrigin(value: string | null) {
    if (value === null) this.removeAttribute("crossorigin");
    else this.setAttribute("crossorigin", value);
  }

  /** 時間を止めているか (`paused` 属性)。 */
  get paused(): boolean {
    return this.hasAttribute("paused");
  }
  set paused(value: boolean) {
    this.toggleAttribute("paused", value);
  }

  /**
   * 読み込みの上限 (T0152。`loadModel` の `limits` と同じ)。`null` (既定) は既定値。属性は無い。
   * 次の読み込み (`src`・`load()`・作り直しの読み直し) から効く (変えただけでは読み直さない)。
   */
  get loadLimits(): LoadLimits | null {
    return this.#loadLimits;
  }
  set loadLimits(value: LoadLimits | null) {
    this.#loadLimits = value;
  }

  /** ugoe-wasm の `WebPuppet` (作成前と切断中は `null`)。直接操作したら `requestRender()` を呼ぶ。 */
  get puppet(): WebPuppet | null {
    return this.#puppet;
  }

  /**
   * `WebPuppet` ができたら解決する。作成に失敗したときと、GPU を続けて失って作り直しを諦めたときは、その理由で reject する
   * (`error` イベントも送る)。作り直し (`restart()`・`backend` の変更・GPU の喪失) や切断と、GPU が戻るのを待ち始めたとき
   * (`gpulost`) に、新しい `WebPuppet` を待つ Promise に変わる (前の Promise が解決か reject していたとき)。
   */
  get ready(): Promise<WebPuppet> {
    return this.#ready.promise;
  }

  /**
   * GPU を失って、戻るのを待っている (WebGL2 の復元を待つ・作り直しで作れずに試し直しを待つ)。待ち始めたら `gpulost` を
   * 送る。作り直せたら (`ready` が解決したら) と、諦めたら (`error`) 偽に戻る。
   */
  get waitingForGpu(): boolean {
    return this.#waitingForGpu;
  }

  /** 描画に使っている API (`"webgpu"` | `"webgl2"`)。作成前は `null`。 */
  get backend(): string | null {
    return this.#puppet?.backend() ?? null;
  }

  /** 最後の読み込みの警告の文。 */
  get warnings(): readonly string[] {
    return this.#warnings;
  }

  /** 最後の読み込みの警告の構造 (`warnings` と同じ順・同じ数)。 */
  get warningDetails(): readonly UgoeLoadWarning[] {
    return this.#warningDetails;
  }

  /** パラメータの定義 (値域・既定値)。モデルが無ければ空。 */
  get parameterDefinitions(): UgoeParameterDefinition[] {
    if (!this.#puppet) return [];
    return parameterDefinitions(this.#puppet).parameters;
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

  /** 最後に `setTracking` で渡した入力 (`clearTracking` の後は `null`)。 */
  get tracking(): TrackingInput | null {
    return this.#tracking;
  }

  /**
   * フェイストラッキングの入力を渡す (毎フレーム呼んでよい)。`null` は `clearTracking()` と同じ。
   *
   * 入力は次のフレームから、モーションの後・表情と目パチの前にパラメータへ書き込まれる (`paused` の間も反映する)。
   * トラッキングの間はモデルの目パチを止める。モデルを読み直したり作り直したりしても、最後の入力を渡し直す。
   * 平滑化・キャリブレーション・左右反転の直しはホストが行う (MediaPipe の結果は `trackingFromFaceLandmarker` で変換でき、
   * 平滑化とキャリブレーションには `TrackingFilter`・`captureNeutral`・`calibrateTracking` を使える)。
   */
  setTracking(input: TrackingInput | null): void {
    if (input === null) {
      this.clearTracking();
      return;
    }
    this.#tracking = input;
    this.#applyTracking();
  }

  /**
   * トラッキングをやめる。トラッキングが書き込んでいたパラメータを既定値に戻し (`parameters` 属性にあればその値)、
   * 目パチを再び有効にする。
   */
  clearTracking(): void {
    if (this.#tracking === null) return;
    this.#tracking = null;
    this.#trackingDirty = false;
    const puppet = this.#puppet;
    if (!puppet?.has_model()) return;
    const ids = new Set(puppet.tracking_parameter_ids());
    puppet.clear_tracking();
    puppet.set_enabled("eye_blink", true);
    for (const def of this.parameterDefinitions) {
      if (ids.has(def.id)) puppet.set_parameter(def.id, def.default);
    }
    this.#applyParameters();
    this.requestRender();
  }

  /**
   * モデルを読み込む (`src` 属性の代わりに、バイト列や JSON の文字列を渡す)。戻り値は警告の文。
   * 接続前に呼ぶと、`WebPuppet` ができてから読み込む (作成や読み込みに失敗したら reject する。
   * その前に `src` を変えると `AbortError` で reject する)。作成に失敗した後や諦めた後 (`ready` が reject した後) は、
   * 待たずにその理由で reject する (`restart()` してから呼び直す)。読み込めなかったモデルは、作り直したときにも読み直さない。
   * 読み込みの途中に GPU を失ったら (`device_lost`)、reject せずに作り直しを待ち、作り直した後に読んで解決する
   * (作り直しを諦めたら reject する)。
   */
  async load(data: ModelData): Promise<string[]> {
    this.#fetch++; // 取得中の `src` があっても、こちらを優先する
    this.#fetching = false;
    this.#abortFetch();
    if (this.#puppet) {
      const warnings = this.#applyModelOrDefer(data);
      if (warnings !== null) return warnings;
    } else {
      if (this.#failure) throw this.#failure;
      this.#pendingModel = data;
    }
    return new Promise((resolve, reject) => this.#loadWaiters.push({ resolve, reject }));
  }

  // `load`・`error` の `detail` を型付けするための重ね定義 (動作は HTMLElement のまま)。
  addEventListener<K extends keyof UgoeAvatarElementEventMap>(
    type: K,
    listener: (this: UgoeAvatarElement, ev: UgoeAvatarElementEventMap[K]) => unknown,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void;
  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void {
    super.addEventListener(type, listener, options);
  }

  removeEventListener<K extends keyof UgoeAvatarElementEventMap>(
    type: K,
    listener: (this: UgoeAvatarElement, ev: UgoeAvatarElementEventMap[K]) => unknown,
    options?: boolean | EventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions,
  ): void {
    super.removeEventListener(type, listener, options);
  }

  /**
   * `WebPuppet` を作り直す。作成に失敗した後や、GPU を続けて失って諦めた後のやり直しに使う (作り直しの回数も数え直す)。
   * 最後に読み込めたモデル (取得中の `src` があればその結果) を読み直し、パラメータの値は引き継ぐ。接続していなければ何もしない。
   */
  restart(): void {
    if (!this.#connected) return;
    this.#restarts = 0;
    this.#restartedAt = null;
    this.#recreate();
  }

  /** 次のフレームで描き直す (`puppet` を直接操作したとき用)。 */
  requestRender(): void {
    this.#needsRender = true;
    this.#schedule();
  }

  connectedCallback(): void {
    this.#connected = true;
    void this.#start();
  }

  disconnectedCallback(): void {
    this.#connected = false;
    this.#failure = null;
    this.#recovering = false;
    this.#waitingForGpu = false;
    this.#stop();
  }

  attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {
    // 開始前 (パーサーが作った要素の昇格など) は `#start` がまとめて適用する。
    // 作成に失敗した後や諦めた後も、属性は覚えておき (`src` は取得する)、`backend` の変更なら作り直す。
    if (oldValue === newValue || !this.#connected) return;
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
      case "motion-over-tracking":
      case "tracking-kept":
        this.#applyMotionOverTracking();
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
    this.#failure = null;
    this.#observe();
    let puppet: WebPuppet;
    try {
      puppet = await this.#create(session);
    } catch (e) {
      if (session !== this.#session) return;
      // GPU を失った後の作り直しで作れなければ (GPU がまだ戻っていない)、待って試し直す (`#onLost` が回数を数える)。
      if (this.#recovering) {
        console.warn("ugoe-avatar: could not recreate the puppet; retrying:", e);
        this.#waitForGpu();
        this.#setRestoreTimer(() => {
          if (session === this.#session) this.#onLost();
        });
        return;
      }
      this.#fail(e);
      return;
    }
    if (session !== this.#session) {
      puppet.free();
      return;
    }
    this.#recovering = false;
    this.#waitingForGpu = false;
    this.#puppet = puppet;
    // WebPuppet はモデルを読み直しても保つので、作ったときに渡す。
    this.#applyMotionOverTracking();
    const [w, h] = this.#size;
    if (w > 0 && h > 0) puppet.resize(w, h);
    this.#ready.resolve(puppet);
    const model = this.#pendingModel ?? this.#model;
    this.#pendingModel = null;
    if (model !== null) {
      try {
        const warnings = this.#applyModelOrDefer(model);
        if (warnings !== null) this.#resolveLoads(warnings);
      } catch (e) {
        // `load` で渡したモデルなら、その Promise で知らせる
        if (this.#loadWaiters.length > 0) this.#rejectLoads(e);
        else this.#dispatchError(e);
      }
    } else if (this.src && !this.#fetching) {
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
    this.#contextLost = false;
    canvas.addEventListener("webglcontextlost", (e) => {
      if (canvas !== this.#canvas) return;
      // 既定の動作 (復元しない) を止めて、復元を待つ。失ったコンテキストには描かない
      e.preventDefault();
      if (this.#contextLost) return;
      this.#contextLost = true;
      if (this.#frame) cancelAnimationFrame(this.#frame);
      this.#frame = 0;
      // 復元が届かないまま待ち続けないよう、時間が来たら新しい canvas で作り直しを試す
      this.#setRestoreTimer(() => {
        if (canvas !== this.#canvas || !this.#contextLost) return;
        console.warn("ugoe-avatar: the WebGL context was not restored in time; recreating");
        this.#onLost();
      });
      this.#waitForGpu();
    });
    canvas.addEventListener("webglcontextrestored", () => {
      // wgpu の資源は戻らないので、新しい canvas で作り直す
      if (canvas === this.#canvas && this.#contextLost) this.#onLost();
    });
    this.#resizeObserver?.disconnect();
    this.#resizeObserver?.observe(canvas);
    return canvas;
  }

  /**
   * 復元の時間切れ・試し直しのタイマーを置く。前のタイマーは消す (失ったまま作り直しに失敗すると、新しい canvas の
   * `webglcontextlost` と `#start` の失敗の両方が置くため。T0480)。
   */
  #setRestoreTimer(callback: () => void): void {
    if (this.#restoreTimer !== null) clearTimeout(this.#restoreTimer);
    this.#restoreTimer = setTimeout(() => {
      this.#restoreTimer = null;
      callback();
    }, RESTORE_TIMEOUT_MS);
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
    this.#watchPixelRatio();
  }

  /**
   * devicePixelRatio だけが変わったとき (倍率の違う画面へウィンドウを移したなど) は ResizeObserver が知らせないので、
   * 今の倍率から外れたら描画バッファの大きさを合わせ直す (ぼやけないように。@ugoe/player の PuppetSurface と同じ。T0932)。
   */
  #watchPixelRatio(): void {
    if (typeof matchMedia !== "function") return;
    this.#pixelRatioQuery = matchMedia(`(resolution: ${globalThis.devicePixelRatio || 1}dppx)`);
    this.#pixelRatioQuery.addEventListener("change", this.#onPixelRatio, { once: true });
  }

  #onPixelRatio = (): void => {
    const css = this.#cssSize;
    if (css !== null) {
      const dpr = globalThis.devicePixelRatio || 1;
      this.#setSize(Math.round(css[0] * dpr), Math.round(css[1] * dpr));
    }
    this.#watchPixelRatio();
  };

  #onResize(entry: ResizeObserverEntry): void {
    const box = entry.devicePixelContentBoxSize?.[0];
    const dpr = globalThis.devicePixelRatio || 1;
    this.#cssSize = [entry.contentRect.width, entry.contentRect.height];
    const w = Math.round(box ? box.inlineSize : entry.contentRect.width * dpr);
    const h = Math.round(box ? box.blockSize : entry.contentRect.height * dpr);
    this.#setSize(w, h);
  }

  #setSize(w: number, h: number): void {
    if (w <= 0 || h <= 0) return;
    this.#size = [w, h];
    this.#puppet?.resize(w, h);
    this.requestRender();
  }

  #stop(): void {
    this.#session++;
    if (this.#restoreTimer !== null) clearTimeout(this.#restoreTimer);
    this.#restoreTimer = null;
    if (this.#frame) cancelAnimationFrame(this.#frame);
    this.#frame = 0;
    this.#last = null;
    this.#resizeObserver?.disconnect();
    this.#resizeObserver = null;
    this.#intersectionObserver?.disconnect();
    this.#intersectionObserver = null;
    this.#pixelRatioQuery?.removeEventListener("change", this.#onPixelRatio);
    this.#pixelRatioQuery = null;
    this.#puppet?.free();
    this.#puppet = null;
    this.#canvas?.remove();
    this.#canvas = null;
    if (this.#ready.settled) this.#ready = deferred();
  }

  /**
   * `WebPuppet` を作り直す。モデルは読み直し、パラメータの値は引き継ぐ。`recovering` は GPU を失った後の作り直し
   * (作れなければ待って試し直す)。
   */
  #recreate(recovering = false): void {
    // 読み込みの途中に GPU を失ったモデル (`#pendingModel`) を読むときは、前のモデルの値を引き継がない。
    if (this.#pendingModel === null && this.#puppet?.has_model()) this.#carry = this.parameters;
    this.#stop();
    this.#recovering = recovering;
    if (!recovering) this.#waitingForGpu = false;
    void this.#start();
  }

  /** GPU が戻るのを待ち始めた (`gpulost` を送る。`ready` は作り直した `WebPuppet` を待つ Promise に変える)。 */
  #waitForGpu(): void {
    if (this.#waitingForGpu) return;
    this.#waitingForGpu = true;
    if (this.#ready.settled) this.#ready = deferred();
    this.dispatchEvent(new CustomEvent("gpulost", { detail: null }));
  }

  /** GPU を失ったので作り直す。続けて失ったら諦める (`restart()` までは作らない)。 */
  #onLost(): void {
    const now = performance.now();
    if (this.#restartedAt !== null && now - this.#restartedAt > RESTART_WINDOW_MS)
      this.#restarts = 0;
    this.#restartedAt = now;
    if (this.#restarts >= MAX_RESTARTS) {
      // `restart()` で作り直したときに戻せるように、パラメータの値は覚えておく
      if (this.#puppet?.has_model()) this.#carry = this.parameters;
      this.#stop();
      this.#recovering = false;
      this.#waitingForGpu = false;
      this.#fail(new Error("the GPU device was lost repeatedly"));
      return;
    }
    this.#restarts++;
    this.#recreate(true);
  }

  /** 作成の失敗・諦めたことを知らせる (`ready` と作成を待つ `load` を reject し、`error` を送る)。 */
  #fail(e: unknown): void {
    const error = e instanceof Error ? e : new Error(String(e));
    this.#failure = error;
    this.#rejectLoads(error);
    if (this.#ready.settled) this.#ready = deferred();
    this.#ready.reject(error);
    this.#dispatchError(error);
  }

  // --- モデル ---

  async #fetchSrc(): Promise<void> {
    const token = ++this.#fetch;
    this.#abortFetch();
    // 作成を待っている `load` は、こちらで置き換わる
    this.#rejectLoads(new DOMException("the load was superseded by src", "AbortError"));
    const src = this.src;
    if (!src) {
      this.#fetching = false;
      this.#model = null;
      this.#pendingModel = null;
      this.#puppet?.unload();
      this.requestRender();
      return;
    }
    this.#fetching = true;
    const abort = new AbortController();
    this.#fetchAbort = abort;
    try {
      const response = await fetch(new URL(src, document.baseURI), {
        credentials: parseCrossOrigin(this.getAttribute("crossorigin")),
        signal: abort.signal,
      });
      if (!response.ok) {
        throw new Error(`failed to fetch ${src}: ${response.status} ${response.statusText}`);
      }
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (token !== this.#fetch) return;
      if (this.#puppet) this.#applyModelOrDefer(bytes);
      else this.#pendingModel = bytes;
    } catch (e) {
      if (token === this.#fetch) this.#dispatchError(e);
    } finally {
      if (token === this.#fetch) this.#fetching = false;
      if (this.#fetchAbort === abort) this.#fetchAbort = null;
    }
  }

  /** 取得中の `src` を中断する (結果は `#fetch` の番号で捨てる)。 */
  #abortFetch(): void {
    this.#fetchAbort?.abort();
    this.#fetchAbort = null;
  }

  /**
   * `model` を読み込む (`#applyModelOrThrow`)。読み込みの途中に GPU を失った (`device_lost`) なら、失敗とせずに `model` を
   * `#pendingModel` に置いて `null` を返す (作り直した後に読む。`load()` は待たせたまま)。作り直しは描画の喪失と同じ道で
   * 始まる (WebGL2 は `webglcontextlost` の後の復元の待ち、WebGPU は次のフレームの `is_lost()`)。
   */
  #applyModelOrDefer(model: ModelData): string[] | null {
    const puppet = this.#puppet;
    try {
      return this.#applyModelOrThrow(model);
    } catch (e) {
      // ugoe-wasm は `is_lost()` のときだけ `device_lost` を投げる (crates/ugoe-wasm/README.md#読み込みの失敗-t0434)。
      if (!puppet?.is_lost()) throw e;
      console.warn(
        "ugoe-avatar: the GPU device was lost while loading; loading after recreating:",
        e,
      );
      this.#pendingModel = model;
      this.requestRender();
      return null;
    }
  }

  /** `model` を読み込み、属性の状態 (パラメータ・表情・モーション) を適用して `load` を送る。 */
  #applyModelOrThrow(model: ModelData): string[] {
    const puppet = this.#puppet;
    if (!puppet) return [];
    const warnings = loadModel(puppet, model, { limits: this.#loadLimits ?? undefined });
    const warningDetails = loadWarnings(puppet);
    this.#model = model;
    this.#warnings = warnings;
    this.#warningDetails = warningDetails;
    puppet.fit();
    this.#applyParameters();
    if (this.#carry) this.parameters = this.#carry;
    this.#carry = null;
    this.#applyTracking();
    // 表情・モーションの名前がモデルに無くても、読み込み自体は成功として扱う (`error` を送るだけ)。
    this.#guard(() => this.#applyExpression());
    this.#guard(() => this.#applyMotion());
    this.requestRender();
    this.dispatchEvent(
      new CustomEvent<UgoeAvatarLoadDetail>("load", {
        detail: { warnings, warningDetails, backend: puppet.backend() },
      }),
    );
    return warnings;
  }

  /** 作成を待っている `load` を、読み込みの警告で終える。 */
  #resolveLoads(warnings: string[]): void {
    const waiters = this.#loadWaiters;
    this.#loadWaiters = [];
    for (const w of waiters) w.resolve(warnings);
  }

  /** 作成を待っている `load` を、例外で終える。 */
  #rejectLoads(error: unknown): void {
    const waiters = this.#loadWaiters;
    this.#loadWaiters = [];
    for (const w of waiters) w.reject(error);
  }

  #applyParameters(): void {
    for (const [id, v] of parseParameters(this.getAttribute("parameters"))) {
      this.setParameter(id, v);
    }
  }

  #applyTracking(): void {
    const puppet = this.#puppet;
    if (!this.#tracking || !puppet?.has_model()) return;
    puppet.set_tracking(...trackingArguments(this.#tracking));
    puppet.set_enabled("eye_blink", false);
    this.#trackingDirty = true;
    this.requestRender();
  }

  #applyMotionOverTracking(): void {
    const puppet = this.#puppet;
    if (!puppet) return;
    puppet.set_motion_over_tracking(this.hasAttribute("motion-over-tracking"));
    puppet.set_tracking_kept(parseIdList(this.getAttribute("tracking-kept")));
    this.requestRender();
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
    this.#motionPending = false;
    if (name) {
      const priority = parseMotionPriority(this.getAttribute("motion-priority"));
      this.#motionPending = !puppet.start_motion(name, priority);
    } else {
      puppet.stop_motions();
    }
    this.requestRender();
  }

  // --- 描画のループ ---

  #animating(): boolean {
    return !this.paused && this.#visible && (this.#puppet?.has_model() ?? false);
  }

  #schedule(): void {
    if (this.#frame || !this.#puppet || this.#contextLost) return;
    if (!this.#needsRender && !this.#animating()) return;
    this.#frame = requestAnimationFrame((now) => this.#tick(now));
  }

  #tick(now: number): void {
    this.#frame = 0;
    const puppet = this.#puppet;
    if (!puppet) return;
    if (puppet.is_lost()) {
      this.#onLost();
      return;
    }
    if (this.#animating()) {
      const dt = this.#last === null ? 0 : Math.min((now - this.#last) / 1000, MAX_DT);
      this.#last = now;
      puppet.update(dt);
      if (this.#motionPending) this.#guard(() => this.#applyMotion());
    } else {
      this.#last = null;
      // 時間を止めていても、新しいトラッキングの入力は反映する (時間は進めない)
      if (this.#trackingDirty && puppet.has_model()) puppet.update(0);
    }
    this.#trackingDirty = false;
    this.#needsRender = false;
    try {
      puppet.render();
    } catch (e) {
      if (puppet.is_lost()) {
        this.#onLost();
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
    this.dispatchEvent(new CustomEvent<UgoeAvatarErrorDetail>("error", { detail: { error } }));
  }
}

/**
 * `<ugoe-avatar>` を登録する (登録済みなら何もしない)。別の名前でも登録できる
 * (1 つのクラスは 1 つの名前にしか登録できないので、そのときは派生クラスを登録する)。
 * `customElements` が無い環境 (サーバーでの描画) では何もしない。
 */
export function defineUgoeAvatar(name = "ugoe-avatar"): void {
  // DOM の無い環境 (SSR) では何もしない
  if (typeof customElements === "undefined" || customElements.get(name)) return;
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
