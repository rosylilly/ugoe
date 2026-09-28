// モデル 1 体の表示と、フレームのループ。WebPuppet (ugoe-wasm) を持ち、UI に状態を公開する。
//
// フレームごとに: ドライバ (drivers.ts) の値を設定 → `update(dt)` (物理など) → スライダーの値を同期 → 描画。
// 描画は WebGPU が無ければ WebGL2 で行う (docs/adr/0003-editor-viewport-in-webview.md)。
import { parseHexColor } from "./color";
import { DriverSet } from "./drivers";
import { detectModelKind, type ModelFile } from "./modelFile";
import {
  groupParameters,
  type ParameterDef,
  type ParameterSection,
  parseParameterList,
} from "./parameters";
import { loadWasm, type UgoeWasmModule, type WebPuppet } from "./wasm/load";

export type PlayerStatus =
  /** wasm と GPU を用意している。 */
  | "starting"
  /** モデルを開いていない。 */
  | "empty"
  | "ready"
  /** wasm の生成物が無い、または描画を始められない。 */
  | "unavailable";

/** 1 フレームで進める時間の上限 (秒)。タブが裏にあった後などに物理が飛ばないように。 */
const MAX_DT = 0.1;
/** スライダーの値を書き換える差の下限。 */
const EPSILON = 1e-6;

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export class Player {
  status = $state<PlayerStatus>("starting");
  /** 直近のエラー (読み込みや描画の失敗)。 */
  error = $state<string | null>(null);
  /** 読み込めたが注意が要ること (テクスチャの代用など)。 */
  warnings = $state<string[]>([]);
  modelName = $state<string | null>(null);
  /** 描画に使っている API (`webgpu` / `webgl2`)。 */
  backend = $state<string | null>(null);
  sections = $state<ParameterSection[]>([]);
  /** パラメータの現在値 (ID → 値)。 */
  values = $state<Record<string, number>>({});
  /** 直前のフレームでドライバが値を設定したパラメータ。スライダーでは動かせない。 */
  driven = $state<ReadonlySet<string>>(new Set());
  /** フィットに対する倍率。 */
  zoom = $state(1);
  background = $state("#e6e7eb");
  transparent = $state(false);

  /** パラメータを毎フレーム外から動かすもの (drivers.ts)。 */
  readonly drivers = new DriverSet();

  #wasm: UgoeWasmModule | null = null;
  #puppet: WebPuppet | null = null;
  #container: HTMLElement | null = null;
  #canvas: HTMLCanvasElement | null = null;
  /** 開いているファイル (GPU を作り直したときに読み直す)。 */
  #file: ModelFile | null = null;
  #parameters: ParameterDef[] = [];
  #cssSize: [number, number] = [1, 1];
  #frame = 0;
  #lastTime: number | null = null;
  #recovering = false;

  /** `container` の中に canvas を作り、描画を始める。 */
  async attach(container: HTMLElement): Promise<void> {
    this.#container = container;
    this.#wasm = await loadWasm();
    if (this.#wasm === null) {
      this.status = "unavailable";
      this.error =
        "ugoe-wasm がビルドされていません (apps/viewer で `pnpm wasm` を実行してください)";
      return;
    }
    await this.#createPuppet();
    this.#frame = requestAnimationFrame(this.#tick);
  }

  detach(): void {
    cancelAnimationFrame(this.#frame);
    this.#destroyPuppet();
    this.#container = null;
  }

  /** モデルを開く。形式は中身から判別する。 */
  async load(file: ModelFile): Promise<void> {
    this.#file = file;
    const puppet = this.#puppet;
    // まだ描画の準備中なら、準備ができたときに読む (#createPuppet)。
    if (puppet === null) return;
    const kind = detectModelKind(file);
    if (kind === null) {
      this.error = `${file.name}: .ugoe か JSON のモデルではありません`;
      return;
    }
    try {
      this.warnings =
        kind === "bundle"
          ? puppet.load_bundle(file.bytes)
          : puppet.load_json(new TextDecoder().decode(file.bytes));
    } catch (e) {
      this.error = `${file.name}: ${message(e)}`;
      return;
    }
    // 読み直しでは同じ ID の値が引き継がれるが、別のファイルを開いたときは既定値から始める。
    puppet.reset_parameters();
    const list = parseParameterList(puppet.parameters_json());
    this.#parameters = list.parameters;
    this.sections = groupParameters(list, "その他");
    this.values = {};
    this.#syncValues();
    this.modelName = file.name;
    this.error = null;
    this.status = "ready";
    this.fit();
  }

  /** スライダーからの設定。ドライバが動かしているパラメータは無視する。 */
  setParameter(id: string, value: number): void {
    const puppet = this.#puppet;
    if (puppet === null || this.driven.has(id)) return;
    puppet.set_parameter(id, value);
    this.values[id] = puppet.parameter(id) ?? value;
  }

  resetParameter(id: string): void {
    const def = this.#parameters.find((p) => p.id === id);
    if (def) this.setParameter(id, def.default);
  }

  resetAllParameters(): void {
    this.#puppet?.reset_parameters();
    this.#syncValues();
  }

  // --- ビュー。座標は canvas 要素の左上からの CSS ピクセル ---

  fit(): void {
    this.#puppet?.fit();
    this.zoom = this.#puppet?.zoom() ?? 1;
  }

  /** `(x, y)` を中心に `factor` 倍する。位置を省くと中央。 */
  zoomBy(factor: number, x = this.#cssSize[0] / 2, y = this.#cssSize[1] / 2): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    const dpr = devicePixelRatio;
    puppet.zoom_at(factor, x * dpr, y * dpr);
    this.zoom = puppet.zoom();
  }

  panBy(dx: number, dy: number): void {
    const dpr = devicePixelRatio;
    this.#puppet?.pan_by(dx * dpr, dy * dpr);
  }

  /** canvas 要素の大きさ (CSS ピクセル) が変わった。 */
  resize(width: number, height: number): void {
    this.#cssSize = [Math.max(1, width), Math.max(1, height)];
    const dpr = devicePixelRatio;
    this.#puppet?.resize(
      Math.max(1, Math.round(this.#cssSize[0] * dpr)),
      Math.max(1, Math.round(this.#cssSize[1] * dpr)),
    );
  }

  setBackground(color: string, transparent: boolean): void {
    this.background = color;
    this.transparent = transparent;
    this.#applyBackground();
  }

  // --- 内部 ---

  #tick = (now: number): void => {
    this.#frame = requestAnimationFrame(this.#tick);
    const puppet = this.#puppet;
    if (puppet === null) return;
    if (puppet.is_lost()) {
      void this.#recover();
      return;
    }
    const dt = this.#lastTime === null ? 0 : Math.min((now - this.#lastTime) / 1000, MAX_DT);
    this.#lastTime = now;
    if (puppet.has_model()) {
      const driven = this.drivers.collect(dt);
      for (const [id, value] of driven) puppet.set_parameter(id, value);
      if (
        driven.size !== this.driven.size ||
        [...driven.keys()].some((id) => !this.driven.has(id))
      ) {
        this.driven = new Set(driven.keys());
      }
      puppet.update(dt);
      this.#syncValues();
    }
    try {
      puppet.render();
    } catch (e) {
      const text = message(e);
      if (this.error !== text) this.error = text;
    }
  };

  /** WebPuppet の値をスライダーの表示へ写す (変わったものだけ)。 */
  #syncValues(): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    for (const { id } of this.#parameters) {
      const value = puppet.parameter(id);
      if (value === undefined) continue;
      const shown = this.values[id];
      if (shown === undefined || Math.abs(shown - value) > EPSILON) this.values[id] = value;
    }
  }

  #applyBackground(): void {
    const rgb = parseHexColor(this.background) ?? [1, 1, 1];
    this.#puppet?.set_background(rgb[0], rgb[1], rgb[2], this.transparent ? 0 : 1);
  }

  /**
   * canvas と WebPuppet を作る。WebGPU で作れなければ、新しい canvas で WebGL2 を試す
   * (1 つの canvas は WebGPU と WebGL2 を併用できない)。
   */
  async #createPuppet(): Promise<void> {
    const wasm = this.#wasm;
    const container = this.#container;
    if (wasm === null || container === null) return;
    let lastError: unknown = null;
    for (const backend of ["auto", "webgl2"]) {
      const canvas = document.createElement("canvas");
      canvas.className = "viewport-canvas";
      container.prepend(canvas);
      try {
        this.#puppet = await wasm.WebPuppet.create(canvas, backend);
        this.#canvas = canvas;
        break;
      } catch (e) {
        lastError = e;
        canvas.remove();
      }
    }
    const puppet = this.#puppet;
    if (puppet === null) {
      this.status = "unavailable";
      this.error = `描画を始められません: ${message(lastError)}`;
      return;
    }
    this.backend = puppet.backend();
    this.resize(...this.#cssSize);
    this.#applyBackground();
    this.status = "empty";
    if (this.#file !== null) await this.load(this.#file);
  }

  #destroyPuppet(): void {
    this.#puppet?.free();
    this.#puppet = null;
    this.#canvas?.remove();
    this.#canvas = null;
  }

  /** GPU を失った (GPU プロセスのクラッシュなど)。作り直して、開いていたモデルを読み直す。 */
  async #recover(): Promise<void> {
    if (this.#recovering) return;
    this.#recovering = true;
    const values = { ...this.values };
    this.#destroyPuppet();
    await this.#createPuppet();
    for (const [id, value] of Object.entries(values)) this.#puppet?.set_parameter(id, value);
    this.#syncValues();
    this.#recovering = false;
  }
}

declare global {
  interface Window {
    /** 表示中のビューア (ヘッドレスのブラウザでの確認や、外からドライバを登録するため)。 */
    __UGOE_VIEWER__?: Player;
  }
}
