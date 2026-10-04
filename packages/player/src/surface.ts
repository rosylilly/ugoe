// WebPuppet の寿命 (作る・作り直す・捨てる) と描画バッファの大きさ。Player とエディタのビューポート
// (apps/editor の ViewportController) で共有する (T0932。決めたことは docs/ui.md#webpuppet-の寿命)。
//
// - canvas はここが作って `container` の先頭に置く。WebGPU で作れなければ、新しい canvas で WebGL2 を試す
//   (1 つの canvas は WebGPU と WebGL2 を併用できない)。一度 WebGL2 に落ちたら、作り直しでも WebGL2 から始める。
// - `attach`・`detach` のたびに番号を進め、待った後で番号が変わっていたら、遅れてできた WebPuppet と canvas を捨てる。
// - GPU を失ったら canvas ごと作り直す。WebGPU のデバイスの喪失と WebGL2 のコンテキストの喪失は、どちらも
//   `is_lost()` で分かる (WebGL2 は ugoe-wasm が canvas の `webglcontextlost` で印を立てる)。描くたびに `checkLost` で
//   確かめるほか、要求されたときだけ描くホスト (エディタ) のために canvas の `webglcontextlost` でもすぐ作り直す。
//   作り直しは一度に 1 つだけで、捨てた WebPuppet の印やイベントは見ないので、2 つの知らせで 2 回作り直すことはない。
// - devicePixelRatio だけが変わったとき (倍率の違う画面へウィンドウを移したなど) は ResizeObserver が知らせないので、
//   `matchMedia` で聞いて描画バッファの大きさを合わせ直す。

/** PuppetSurface が使う WebPuppet のメソッド (構造の型。アプリと Player の `WebPuppet` の型はこれを満たす)。 */
export interface SurfacePuppet {
  free(): void;
  is_lost(): boolean;
  resize(width: number, height: number): void;
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
  /** WebGPU でも WebGL2 でも作れなかった (`error` は最後の失敗)。 */
  failed(error: unknown): void;
  /** GPU を失ったので作り直す (`puppet` はまだ捨てていない。値を読むことはできる)。 */
  lost?(puppet: P): void;
  /** WebPuppet を捨てた (`detach` と作り直し)。 */
  released?(): void;
  /** devicePixelRatio だけが変わり、描画バッファの大きさを合わせ直した。 */
  pixelRatioChanged?(): void;
  /** 作った canvas に付ける class (省くと付けない)。 */
  readonly canvasClass?: string;
}

/** `attach` の結果。`superseded` は待っている間に `detach` か次の `attach` があった。 */
export type AttachResult = "ready" | "missing" | "failed" | "superseded";

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

  /**
   * `container` の先頭に canvas を作り、WebPuppet を作る。前に `attach` していれば、先に `detach` する。
   * 待っている間に `detach` か次の `attach` があれば、作った WebPuppet と canvas は捨てて `"superseded"` を返す。
   */
  async attach(container: HTMLElement): Promise<AttachResult> {
    this.detach();
    const token = this.#token;
    this.#container = container;
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
  }

  /** 描く領域の大きさ (CSS ピクセル) が変わった。描画バッファは今の devicePixelRatio を掛けた画素にする。 */
  resize(width: number, height: number): void {
    this.#cssSize = [Math.max(1, width), Math.max(1, height)];
    this.#pixelRatio = currentPixelRatio();
    this.#puppet?.resize(...this.#pixelSize());
  }

  /** GPU を失っていれば作り直しを始めて `true` を返す (描く前に呼ぶ)。 */
  checkLost(): boolean {
    if (!this.#puppet?.is_lost()) return false;
    void this.recover();
    return true;
  }

  /**
   * canvas ごと作り直す (GPU を失ったとき)。作り直している間は何もしない。待っている間に `detach` されたら、作ったものは捨てる。
   */
  async recover(): Promise<void> {
    const puppet = this.#puppet;
    if (puppet === null || this.recovering) return;
    const token = this.#token;
    this.#recovering = token;
    try {
      this.#host.lost?.(puppet);
      this.#release();
      await this.#create(token, true);
    } finally {
      if (this.#recovering === token) this.#recovering = null;
    }
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
      puppet.resize(...this.#pixelSize());
      await this.#host.created(puppet, recovered);
      return token === this.#token ? "ready" : "superseded";
    }
    this.#host.failed(lastError);
    return "failed";
  }

  #release(): void {
    const puppet = this.#puppet;
    this.#puppet = null;
    this.#canvas?.removeEventListener("webglcontextlost", this.#onContextLost);
    this.#canvas?.remove();
    this.#canvas = null;
    if (puppet === null) return;
    puppet.free();
    this.#host.released?.();
  }

  /**
   * WebGL2 のコンテキストを失った。WebPuppet の `is_lost()` も真になるが、要求されたときだけ描くホストは次に描くまで
   * 気づかない (その間は真っ白) ので、ここですぐ作り直す。
   */
  #onContextLost = (event: Event): void => {
    if (event.target !== this.#canvas) return;
    console.warn("ugoe: WebGL context lost; recreating the canvas");
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
