// WebPuppet の寿命 (作る・作り直す・捨てる) と描画バッファの大きさ。Player とエディタのビューポート
// (apps/editor の ViewportController) で共有する (T0932。決めたことは docs/ui.md#webpuppet-の寿命)。
//
// - canvas はここが作って `container` の先頭に置く。WebGPU で作れなければ、新しい canvas で WebGL2 を試す
//   (1 つの canvas は WebGPU と WebGL2 を併用できない)。一度 WebGL2 に落ちたら、作り直しでも WebGL2 から始める。
// - `attach`・`detach` のたびに番号を進め、待った後で番号が変わっていたら、遅れてできた WebPuppet と canvas を捨てる。
// - GPU を失ったら canvas ごと作り直す。WebGPU のデバイスの喪失と WebGL2 のコンテキストの喪失は、どちらも
//   `is_lost()` で分かる (WebGL2 は ugoe-wasm が canvas の `webglcontextlost` で印を立てる)。WebGPU は描くたびの
//   `checkLost` で気づいてすぐ作り直す。WebGL2 は canvas の `webglcontextlost` を `preventDefault` して
//   `webglcontextrestored` を待ってから作り直す (`<ugoe-avatar>` と同じ。T0935)。待っている間は描かない。
//   `RESTORE_TIMEOUT_MS` 待っても復元が届かなければ、新しい canvas で作り直しを試す (T0938)。作り直しで作れなければ
//   (GPU がまだ戻っていない)、同じだけ待って試し直す。待ち始めたら `waiting` で知らせる。
//   作り直しは一度に 1 つだけで、捨てた WebPuppet の印やイベントは見ないので、2 つの知らせで 2 回作り直すことはない。
// - 前の作り直しから `RESTART_WINDOW_MS` 以内に失うことが続き、`MAX_RESTARTS` 回を超えたら、作り直さずに
//   `failed(GpuLostRepeatedlyError)` で知らせる (次の `attach` か `restart` まで作らない)。
// - devicePixelRatio だけが変わったとき (倍率の違う画面へウィンドウを移したなど) は ResizeObserver が知らせないので、
//   `matchMedia` で聞いて描画バッファの大きさを合わせ直す。
// - GPU を失って作り直すときは、失った WebPuppet のビュー (ズームとパン) を覚えて、作り直した WebPuppet に `created` の前に
//   渡す (作り直しは利用者の操作ではないので、見ている場所を変えない。T0942)。パンは CSS ピクセルで覚え、作り直したときの
//   devicePixelRatio で描画バッファの画素に直す。`attach` (作り直しでない作成) では既定のビュー (fit) から始める。

/** PuppetSurface が使う WebPuppet のメソッド (構造の型。アプリと Player の `WebPuppet` の型はこれを満たす)。 */
export interface SurfacePuppet {
  free(): void;
  is_lost(): boolean;
  resize(width: number, height: number): void;
  zoom(): number;
  pan_x(): number;
  pan_y(): number;
  set_view(zoom: number, pan_x: number, pan_y: number): void;
}

/** 作り直しで渡し直すビュー。`zoom` は fit に対する倍率、パンは fit の位置からのずれ (CSS ピクセル)。 */
interface SavedView {
  zoom: number;
  panX: number;
  panY: number;
}

/** WebPuppet を作る口 (初期化した ugoe-wasm のモジュール)。 */
export interface SurfaceWasm<P extends SurfacePuppet> {
  WebPuppet: {
    create(canvas: HTMLCanvasElement, backend?: string | null): Promise<P>;
  };
}

/** PuppetSurface を使うもの (Player・ViewportController) が渡す口。 */
export interface SurfaceHost<P extends SurfacePuppet> {
  /** ugoe-wasm を読み込む。生成物が無ければ `null` (`attach` は `"missing"` を返す)。 */
  loadWasm(): Promise<SurfaceWasm<P> | null>;
  /**
   * WebPuppet ができた (`attach` と作り直しのたび)。描画バッファの大きさは合わせてある。`recovered` は GPU を失った後の
   * 作り直し。約束を返せば `attach`・作り直しはそれを待つ (待っている間に `detach` されることがある)。
   */
  created(puppet: P, recovered: boolean): void | Promise<void>;
  /**
   * WebGPU でも WebGL2 でも作れなかった (`error` は最後の失敗)。GPU を続けて失って作り直しを諦めたときは
   * `GpuLostRepeatedlyError`。
   */
  failed(error: unknown): void;
  /** GPU を失ったので作り直す (`puppet` はまだ捨てていない。値を読むことはできる)。 */
  lost?(puppet: P): void;
  /**
   * GPU を失って、戻るのを待ち始めた (WebGL2 の復元を待つ・作り直しで作れずに試し直しを待つ)。次の `created` か
   * `failed` まで描かない。
   */
  waiting?(): void;
  /** WebPuppet を捨てた (`detach` と作り直し)。 */
  released?(): void;
  /** devicePixelRatio だけが変わり、描画バッファの大きさを合わせ直した。 */
  pixelRatioChanged?(): void;
  /** 作った canvas に付ける class (省くと付けない)。 */
  readonly canvasClass?: string;
}

/** GPU を失ったときに続けて作り直す回数の上限 (作り直してもすぐ失うときに止めるため。`<ugoe-avatar>` と同じ)。 */
export const MAX_RESTARTS = 3;
/** 前の作り直しからこれ (ミリ秒) より後に失ったら、続けて失ったとは数えない (スリープからの復帰などで時々失うため)。 */
export const RESTART_WINDOW_MS = 60_000;
/**
 * WebGL2 のコンテキストを失ってから復元 (`webglcontextrestored`) を待つ時間 (ミリ秒)。届かなければ新しい canvas で作り直しを
 * 試す。作り直しで作れなかったときに、次に試すまで待つ時間も同じ (`<ugoe-avatar>` と同じ。T0938)。
 */
export const RESTORE_TIMEOUT_MS = 5_000;

/** GPU を続けて失ったので、作り直しを諦めた (`SurfaceHost.failed` に渡す)。 */
export class GpuLostRepeatedlyError extends Error {
  constructor() {
    super("the GPU device was lost repeatedly");
    this.name = "GpuLostRepeatedlyError";
  }
}

/** `attach` の結果。`superseded` は待っている間に `detach` か次の `attach` があった。 */
export type AttachResult = "ready" | "missing" | "failed" | "superseded";

/** `ms` ミリ秒待つ。 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** 今の devicePixelRatio (無ければ 1)。 */
function currentPixelRatio(): number {
  return globalThis.devicePixelRatio || 1;
}

/** WebPuppet と、それを描く canvas の寿命。 */
export class PuppetSurface<P extends SurfacePuppet> {
  readonly #host: SurfaceHost<P>;
  #wasm: SurfaceWasm<P> | null = null;
  #container: HTMLElement | null = null;
  #canvas: HTMLCanvasElement | null = null;
  #puppet: P | null = null;
  /** `attach`・`detach` のたびに増える。待っている間に変わったら、作ったものを捨てる。 */
  #token = 0;
  /** 作り直している `attach` の番号 (作り直しは一度に 1 つ)。 */
  #recovering: number | null = null;
  /** `"auto"` で作れずに WebGL2 で作れたら、以後は WebGL2 から作る。 */
  #backend: "auto" | "webgl2" = "auto";
  /** 描く領域の大きさ (CSS ピクセル)。 */
  #cssSize: [number, number] = [1, 1];
  #pixelRatio = currentPixelRatio();
  /** devicePixelRatio の変化を聞いている問い合わせ (`#watchPixelRatio`)。 */
  #pixelRatioQuery: MediaQueryList | null = null;
  /** WebGL2 のコンテキストを失って、`webglcontextrestored` を待っている。 */
  #awaitingRestore = false;
  /** 復元を待つ時間切れ (`RESTORE_TIMEOUT_MS`) のタイマー。 */
  #restoreTimer: ReturnType<typeof setTimeout> | null = null;
  /** 続けて作り直した回数と、前に作り直し始めた時刻 (`performance.now()`)。`attach` で戻す。 */
  #restarts = 0;
  #restartedAt: number | null = null;
  /** GPU を失った WebPuppet のビュー (作り直した WebPuppet に渡す)。`attach`・`detach` で忘れる。 */
  #savedView: SavedView | null = null;

  constructor(host: SurfaceHost<P>) {
    this.#host = host;
  }

  /** 描画に使える WebPuppet (作っている間・作れなかったとき・`detach` の後は `null`)。 */
  get puppet(): P | null {
    return this.#puppet;
  }

  /** 描画バッファの大きさを決めた devicePixelRatio (CSS ピクセルと描画バッファの画素の換算に使う)。 */
  get pixelRatio(): number {
    return this.#pixelRatio;
  }

  /** GPU を失って作り直している間。 */
  get recovering(): boolean {
    return this.#recovering !== null && this.#recovering === this.#token;
  }

  /** WebGL2 のコンテキストを失って、復元 (`webglcontextrestored`) を待っている間 (描かない)。 */
  get awaitingRestore(): boolean {
    return this.#awaitingRestore;
  }

  /**
   * `container` の先頭に canvas を作り、WebPuppet を作る。前に `attach` していれば、先に `detach` する。
   * 待っている間に `detach` か次の `attach` があれば、作った WebPuppet と canvas は捨てて `"superseded"` を返す。
   */
  async attach(container: HTMLElement): Promise<AttachResult> {
    this.detach();
    const token = this.#token;
    this.#container = container;
    this.#restarts = 0;
    this.#restartedAt = null;
    this.#savedView = null;
    const wasm = await this.#host.loadWasm();
    if (token !== this.#token) return "superseded";
    if (wasm === null) return "missing";
    this.#wasm = wasm;
    this.#watchPixelRatio();
    return this.#create(token, false);
  }

  /** WebPuppet と canvas を捨て、devicePixelRatio を聞くのをやめる。作っている途中のものも捨てる。 */
  detach(): void {
    this.#token++;
    this.#pixelRatioQuery?.removeEventListener("change", this.#onPixelRatio);
    this.#pixelRatioQuery = null;
    this.#release();
    this.#wasm = null;
    this.#container = null;
    this.#savedView = null;
  }

  /** 描く領域の大きさ (CSS ピクセル) が変わった。描画バッファは今の devicePixelRatio を掛けた画素にする。 */
  resize(width: number, height: number): void {
    this.#cssSize = [Math.max(1, width), Math.max(1, height)];
    this.#pixelRatio = currentPixelRatio();
    this.#puppet?.resize(...this.#pixelSize());
  }

  /**
   * GPU を失っていれば `true` を返す (描く前に呼び、`true` なら描かない)。WebGL2 のコンテキストの復元を待っている間は
   * 待ち続け、そうでなければ作り直しを始める。
   */
  checkLost(): boolean {
    if (this.#puppet === null) return false;
    if (this.#awaitingRestore) return true;
    if (!this.#puppet.is_lost()) return false;
    void this.recover();
    return true;
  }

  /**
   * canvas ごと作り直す (GPU を失ったとき)。作り直している間は何もしない。待っている間に `detach` されたら、作ったものは捨てる。
   * 続けて失って `MAX_RESTARTS` 回を超えたら、作り直さずに `failed(GpuLostRepeatedlyError)` で知らせる。
   */
  async recover(): Promise<void> {
    const puppet = this.#puppet;
    if (puppet === null || this.recovering) return;
    const token = this.#token;
    this.#recovering = token;
    try {
      this.#saveView(puppet);
      this.#host.lost?.(puppet);
      this.#release();
      await this.#recreate(token);
    } finally {
      if (this.#recovering === token) this.#recovering = null;
    }
  }

  /**
   * 作り直しを諦めた後 (`failed(GpuLostRepeatedlyError)`) に、同じ container で作り直す (利用者の「作り直す」ボタン)。
   * 作り直しの回数は数え直し、`created(puppet, true)` で知らせる (使う側はモデルなどを渡し直す)。WebPuppet があれば
   * `lost` で知らせてから捨てる。`attach` していなければ何もしない。
   */
  async restart(): Promise<void> {
    if (this.#wasm === null || this.#container === null) return;
    this.#token++;
    const token = this.#token;
    this.#recovering = token;
    this.#restarts = 0;
    this.#restartedAt = null;
    try {
      if (this.#puppet !== null) {
        this.#saveView(this.#puppet);
        this.#host.lost?.(this.#puppet);
      }
      this.#release();
      await this.#recreate(token);
    } finally {
      if (this.#recovering === token) this.#recovering = null;
    }
  }

  /**
   * 捨てた WebPuppet の代わりを新しい canvas で作る。作れなければ (GPU がまだ戻っていない) `RESTORE_TIMEOUT_MS` 待って
   * 試し直す。続けて `MAX_RESTARTS` 回を超えたら `failed(GpuLostRepeatedlyError)` で知らせる。
   */
  async #recreate(token: number): Promise<void> {
    // 試し直しは `#countRestart` が数えるが、時計に頼らずに回数でも止める。
    for (let attempt = 0; attempt <= MAX_RESTARTS; attempt++) {
      if (!this.#countRestart()) break;
      if ((await this.#create(token, true)) !== "failed") return;
      this.#host.waiting?.();
      await sleep(RESTORE_TIMEOUT_MS);
      if (token !== this.#token) return;
    }
    this.#host.failed(new GpuLostRepeatedlyError());
  }

  /** 作り直しを 1 回数える。続けて失って上限を超えたら `false` (諦める)。 */
  #countRestart(): boolean {
    const now = performance.now();
    if (this.#restartedAt !== null && now - this.#restartedAt > RESTART_WINDOW_MS)
      this.#restarts = 0;
    this.#restartedAt = now;
    if (this.#restarts >= MAX_RESTARTS) return false;
    this.#restarts++;
    return true;
  }

  async #create(token: number, recovered: boolean): Promise<AttachResult> {
    const wasm = this.#wasm;
    const container = this.#container;
    if (wasm === null || container === null) return "superseded";
    let lastError: unknown = null;
    for (const backend of this.#backend === "auto" ? ["auto", "webgl2"] : ["webgl2"]) {
      const canvas = document.createElement("canvas");
      if (this.#host.canvasClass) canvas.className = this.#host.canvasClass;
      [canvas.width, canvas.height] = this.#pixelSize();
      container.prepend(canvas);
      let puppet: P;
      try {
        puppet = await wasm.WebPuppet.create(canvas, backend);
      } catch (e) {
        canvas.remove();
        if (token !== this.#token) return "superseded";
        if (backend === "auto") console.warn("ugoe: falling back to WebGL2:", e);
        else if (recovered) console.warn("ugoe: could not recreate the WebPuppet:", e);
        lastError = e;
        continue;
      }
      if (token !== this.#token) {
        puppet.free();
        canvas.remove();
        return "superseded";
      }
      if (backend === "webgl2") this.#backend = "webgl2";
      this.#puppet = puppet;
      this.#canvas = canvas;
      canvas.addEventListener("webglcontextlost", this.#onContextLost);
      canvas.addEventListener("webglcontextrestored", this.#onContextRestored);
      puppet.resize(...this.#pixelSize());
      if (recovered) this.#restoreView(puppet);
      await this.#host.created(puppet, recovered);
      return token === this.#token ? "ready" : "superseded";
    }
    // 作り直しで作れなかったときは、`#recreate` が試し直す。
    if (!recovered) this.#host.failed(lastError);
    return "failed";
  }

  /** 失った WebPuppet のビューを覚える (ビューは CPU の側の状態なので、GPU を失っても読める)。 */
  #saveView(puppet: P): void {
    const dpr = this.#pixelRatio;
    this.#savedView = {
      zoom: puppet.zoom(),
      panX: puppet.pan_x() / dpr,
      panY: puppet.pan_y() / dpr,
    };
  }

  /**
   * 覚えたビューを作り直した WebPuppet に渡す。続けて失ったとき・諦めた後の `restart` でも同じビューを渡すので、
   * `attach`・`detach` まで覚えておく。
   */
  #restoreView(puppet: P): void {
    const view = this.#savedView;
    if (view === null) return;
    const dpr = this.#pixelRatio;
    puppet.set_view(view.zoom, view.panX * dpr, view.panY * dpr);
  }

  #release(): void {
    const puppet = this.#puppet;
    this.#puppet = null;
    this.#awaitingRestore = false;
    this.#clearRestoreTimer();
    this.#canvas?.removeEventListener("webglcontextlost", this.#onContextLost);
    this.#canvas?.removeEventListener("webglcontextrestored", this.#onContextRestored);
    this.#canvas?.remove();
    this.#canvas = null;
    if (puppet === null) return;
    puppet.free();
    this.#host.released?.();
  }

  /**
   * WebGL2 のコンテキストを失った。既定の動作 (復元しない) を止めて、復元を待つ。本物の GPU のリセットでは、復元の前に
   * 新しいコンテキストを作れないことがあるため、すぐには作り直さない (WebPuppet の `is_lost()` も真になるが、
   * 待っている間は `checkLost` が作り直さない)。
   */
  #onContextLost = (event: Event): void => {
    if (event.target !== this.#canvas || this.recovering) return;
    event.preventDefault();
    if (this.#awaitingRestore) return;
    this.#awaitingRestore = true;
    console.warn("ugoe: WebGL context lost; waiting for it to be restored");
    this.#restoreTimer = setTimeout(this.#onRestoreTimeout, RESTORE_TIMEOUT_MS);
    this.#host.waiting?.();
  };

  /** 復元が `RESTORE_TIMEOUT_MS` 待っても届かない。新しい canvas で作り直しを試す。 */
  #onRestoreTimeout = (): void => {
    this.#restoreTimer = null;
    if (!this.#awaitingRestore) return;
    console.warn("ugoe: WebGL context was not restored in time; recreating");
    this.#awaitingRestore = false;
    void this.recover();
  };

  #clearRestoreTimer(): void {
    if (this.#restoreTimer !== null) clearTimeout(this.#restoreTimer);
    this.#restoreTimer = null;
  }

  /**
   * WebGL2 のコンテキストが戻った。wgpu の資源は戻らないので、新しい canvas で作り直す (要求されたときだけ描くホスト
   * (エディタ) のために、次に描くのを待たずにここで始める)。
   */
  #onContextRestored = (event: Event): void => {
    if (event.target !== this.#canvas || !this.#awaitingRestore) return;
    this.#awaitingRestore = false;
    this.#clearRestoreTimer();
    void this.recover();
  };

  #watchPixelRatio(): void {
    if (typeof matchMedia !== "function") return;
    this.#pixelRatioQuery = matchMedia(`(resolution: ${currentPixelRatio()}dppx)`);
    this.#pixelRatioQuery.addEventListener("change", this.#onPixelRatio, { once: true });
  }

  #onPixelRatio = (): void => {
    this.resize(...this.#cssSize);
    this.#host.pixelRatioChanged?.();
    this.#watchPixelRatio();
  };

  #pixelSize(): [number, number] {
    return [
      Math.max(1, Math.round(this.#cssSize[0] * this.#pixelRatio)),
      Math.max(1, Math.round(this.#cssSize[1] * this.#pixelRatio)),
    ];
  }
}
