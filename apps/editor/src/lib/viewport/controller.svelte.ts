// ビューポートの状態と操作 (docs/editor.md#ビューポート)。
//
// - モデルの描画は ugoe-wasm の WebPuppet が行う。パラメータの値は表示の状態なので、
//   スライダーはここ (setParameter) を直接呼び、IPC は通らない。
// - 描画は要求されたときだけ requestAnimationFrame で 1 回行う (requestRender)。
// - 座標: 公開するメソッドはすべて CSS ピクセル (ビューポート要素の左上原点)。
//   WebPuppet は canvas の描画バッファの画素 (CSS ピクセル × devicePixelRatio) で受け取るので、ここで換算する。
import { loadUgoeWasm, type WebPuppet } from "./wasm";

/** ビューポートに表示するモデル。bundle があればテクスチャ付きで、無ければ JSON (白いテクスチャ) で表示する。 */
export interface ViewportModel {
  json: string;
  bundle: Uint8Array | null;
}

export type ViewportStatus = "loading" | "ready" | "unbuilt" | "error";

/** キャンバス座標 → CSS ピクセルの相似変換 (`css = offset + scale * canvas`)。 */
export interface ViewTransform {
  scale: number;
  offsetX: number;
  offsetY: number;
}

export class ViewportController {
  status = $state<ViewportStatus>("loading");
  message = $state<string | null>(null);
  /** 使っているバックエンド ("webgpu" / "webgl2")。 */
  backend = $state<string | null>(null);
  /** パラメータの表示中の値 (ID → 値)。モデルを読み直しても同じ ID の値は保たれる。 */
  parameterValues = $state<Record<string, number>>({});
  /** オーバーレイ用の変換。ビュー・大きさ・モデルが変わるたびに更新される。 */
  transform = $state<ViewTransform | null>(null);
  /** canvas を作り直すたびに増える (Viewport.svelte が {#key} に使う)。 */
  generation = $state(0);
  /** 選択中のメッシュ・ワープの深度を色の点で重ねるか (インスペクタの深度の節で切り替える)。 */
  showDepth = $state(false);

  #puppet: WebPuppet | null = null;
  #model: ViewportModel | null = null;
  /** 次に読み込むモデルでパラメータを既定値から始める (resetParametersOnNextModel)。 */
  #resetOnLoad = false;
  #size = { width: 1, height: 1, dpr: 1 };
  #background: [number, number, number, number] = [0, 0, 0, 0];
  #frame = 0;
  /** "auto" で失敗したら "webgl2" で作り直す (1 つの canvas は WebGPU と WebGL2 を併用できないため)。 */
  #backendPreference: "auto" | "webgl2" = "auto";

  /** 描画中の WebPuppet (ヒットテストなどの問い合わせ用)。 */
  get puppet(): WebPuppet | null {
    return this.#puppet;
  }

  /** canvas に WebPuppet を作る。Viewport.svelte がマウント時に呼ぶ。 */
  async attach(canvas: HTMLCanvasElement): Promise<void> {
    this.detach();
    const generation = this.generation;
    try {
      const wasm = await loadUgoeWasm();
      if (!wasm) {
        this.status = "unbuilt";
        this.message = "ugoe-wasm is not built. Run `just wasm` and reload.";
        return;
      }
      const puppet = await wasm.WebPuppet.create(canvas, this.#backendPreference);
      if (generation !== this.generation) {
        puppet.free();
        return;
      }
      this.#puppet = puppet;
      this.backend = puppet.backend();
      this.status = "ready";
      this.message = null;
      puppet.set_background(...this.#background);
      puppet.resize(this.#pixelWidth(), this.#pixelHeight());
      if (this.#model) this.#load(this.#model);
      this.requestRender();
    } catch (e) {
      if (generation !== this.generation) return;
      if (this.#backendPreference === "auto") {
        console.warn("ugoe: falling back to WebGL2:", e);
        this.#backendPreference = "webgl2";
        this.generation += 1;
        return;
      }
      this.status = "error";
      this.message = String(e);
    }
  }

  /** WebPuppet を破棄する。Viewport.svelte がアンマウント時に呼ぶ。 */
  detach(): void {
    cancelAnimationFrame(this.#frame);
    this.#frame = 0;
    this.#puppet?.free();
    this.#puppet = null;
  }

  /** 表示するモデルを設定する。モデルが変わるたびに呼ぶ (編集・Undo・Redo・新規・開くの後)。 */
  setModel(model: ViewportModel): void {
    if (model === this.#model) return;
    this.#model = model;
    if (this.#puppet) this.#load(model);
  }

  setParameter(id: string, value: number): void {
    this.parameterValues[id] = value;
    if (this.#puppet?.set_parameter(id, value)) {
      this.parameterValues[id] = this.#puppet.parameter(id) ?? value;
      this.requestRender();
    }
  }

  /**
   * 次に読み込むモデル (setModel) では、今の値を引き継がずにパラメータを既定値から始める。
   * 別のモデルに替わるとき (新規・PSD から新規・開く) に、読み直しの前に呼ぶ。
   */
  resetParametersOnNextModel(): void {
    this.#resetOnLoad = true;
  }

  resetParameters(): void {
    this.#puppet?.reset_parameters();
    this.#syncParameters();
    this.requestRender();
  }

  /** ビューポート要素の大きさ (CSS ピクセル) と devicePixelRatio。 */
  resize(width: number, height: number, dpr: number): void {
    this.#size = { width: Math.max(1, width), height: Math.max(1, height), dpr };
    this.#puppet?.resize(this.#pixelWidth(), this.#pixelHeight());
    this.requestRender();
  }

  /** 背景色 (ストレートアルファ、各成分 0..1)。 */
  setBackground(r: number, g: number, b: number, a: number): void {
    this.#background = [r, g, b, a];
    this.#puppet?.set_background(r, g, b, a);
    this.requestRender();
  }

  fit(): void {
    this.#puppet?.fit();
    this.requestRender();
  }

  /** CSS ピクセルの点 `(x, y)` を中心にズームを `factor` 倍する。 */
  zoomAt(factor: number, x: number, y: number): void {
    const { dpr } = this.#size;
    this.#puppet?.zoom_at(factor, x * dpr, y * dpr);
    this.requestRender();
  }

  /** CSS ピクセルで `(dx, dy)` だけずらす。 */
  panBy(dx: number, dy: number): void {
    const { dpr } = this.#size;
    this.#puppet?.pan_by(dx * dpr, dy * dpr);
    this.requestRender();
  }

  /** キャンバス座標 → CSS ピクセル。 */
  canvasToScreen(x: number, y: number): [number, number] | null {
    const t = this.transform;
    return t ? [t.offsetX + t.scale * x, t.offsetY + t.scale * y] : null;
  }

  /** CSS ピクセル → キャンバス座標。 */
  screenToCanvas(x: number, y: number): [number, number] | null {
    const t = this.transform;
    return t ? [(x - t.offsetX) / t.scale, (y - t.offsetY) / t.scale] : null;
  }

  /** CSS ピクセルの点にある、いちばん手前の描画可能要素の ID。 */
  hitTest(x: number, y: number): string | null {
    const p = this.screenToCanvas(x, y);
    const puppet = this.#puppet;
    if (!p || !puppet) return null;
    const index = puppet.hit_test(p[0], p[1]);
    return index === undefined ? null : (puppet.drawable_ids()[index] ?? null);
  }

  /** 次のアニメーションフレームで 1 回描画する (同じフレーム内の要求はまとめる)。 */
  requestRender(): void {
    if (this.#frame || !this.#puppet) return;
    this.#frame = requestAnimationFrame(() => {
      this.#frame = 0;
      this.#render();
    });
  }

  #render(): void {
    const puppet = this.#puppet;
    if (!puppet) return;
    try {
      // TODO(T0303): 物理・モーションを再生するときは update(dt) を毎フレーム呼んで描き続ける。
      puppet.render();
      this.#updateTransform();
    } catch (e) {
      if (puppet.is_lost()) {
        // GPU プロセスのクラッシュなど。canvas ごと作り直す。
        console.warn("ugoe: GPU device lost; recreating the viewport");
        this.detach();
        this.generation += 1;
        return;
      }
      this.status = "error";
      this.message = String(e);
    }
  }

  #load(model: ViewportModel): void {
    const puppet = this.#puppet;
    if (!puppet) return;
    try {
      const warnings = model.bundle
        ? puppet.load_bundle(model.bundle)
        : puppet.load_json(model.json);
      for (const warning of warnings) console.warn("ugoe:", warning);
      if (this.#resetOnLoad) {
        // WebPuppet は同じ ID の値を引き継ぐので、既定値に戻す。
        this.#resetOnLoad = false;
        puppet.reset_parameters();
      } else {
        // 表示中の値を新しいモデルに写す (WebPuppet も同じ ID の値を引き継ぐが、
        // 作り直した WebPuppet では失われているため)。
        for (const [id, value] of Object.entries(this.parameterValues)) {
          puppet.set_parameter(id, value);
        }
      }
      this.#syncParameters();
      if (this.status === "error") {
        this.status = "ready";
        this.message = null;
      }
      this.requestRender();
    } catch (e) {
      this.status = "error";
      this.message = String(e);
    }
  }

  #syncParameters(): void {
    const puppet = this.#puppet;
    if (!puppet) return;
    const values: Record<string, number> = {};
    for (const id of puppet.parameter_ids()) {
      const value = puppet.parameter(id);
      if (value !== undefined) values[id] = value;
    }
    this.parameterValues = values;
  }

  #updateTransform(): void {
    const t = this.#puppet?.view_transform();
    const { dpr } = this.#size;
    const next =
      t && t.length === 3
        ? { scale: (t[0] ?? 1) / dpr, offsetX: (t[1] ?? 0) / dpr, offsetY: (t[2] ?? 0) / dpr }
        : null;
    const prev = this.transform;
    if (
      prev?.scale !== next?.scale ||
      prev?.offsetX !== next?.offsetX ||
      prev?.offsetY !== next?.offsetY
    ) {
      this.transform = next;
    }
  }

  #pixelWidth(): number {
    return Math.max(1, Math.round(this.#size.width * this.#size.dpr));
  }

  #pixelHeight(): number {
    return Math.max(1, Math.round(this.#size.height * this.#size.dpr));
  }
}
