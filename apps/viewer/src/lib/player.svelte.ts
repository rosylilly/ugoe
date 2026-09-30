// モデル 1 体の表示と、フレームのループ。WebPuppet (ugoe-wasm) を持ち、UI に状態を公開する。
//
// フレームごとに: ドライバ (drivers.ts) の値を設定 → トラッキングの入力を渡す → `update(dt)` (物理など)
// → スライダーの値を同期 → 描画。
// 描画は WebGPU が無ければ WebGL2 で行う (docs/adr/0003-editor-viewport-in-webview.md)。
import { setPuppetTracking, type TrackingFrame } from "@ugoe/face-tracking";
import { parseHexColor } from "./color";
import {
  describeDressUpError,
  describeLoadWarnings,
  type ModuleInfo,
  parseDetachedModules,
  parseModules,
  parseSlots,
  type SlotInfo,
} from "./dressUp";
import { DriverSet } from "./drivers";
import { type LightingSettings, loadLighting, saveLighting, toLightingJson } from "./lighting";
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
  /** 直前のフレームでドライバかトラッキングが値を設定したパラメータ。スライダーでは動かせない。 */
  driven = $state<ReadonlySet<string>>(new Set());
  /** トラッキングが書き込むパラメータ (モデルの定義順。`avatar.tracking` の上書きを反映する)。 */
  trackingParameterIds = $state<string[]>([]);
  /** トラッキングの入力を渡しているか。 */
  tracking = $state(false);
  /** モデルの自動の目パチ。モデルを開き直しても保つ。 */
  autoEyeBlink = $state(true);
  /** フィットに対する倍率。 */
  zoom = $state(1);
  background = $state("#e6e7eb");
  transparent = $state(false);
  /** 開いているモデルにライティングを受けるメッシュがあるか (光源のパネルを出すか)。 */
  usesLighting = $state(false);
  /** 光源の設定 (lighting.ts)。モデルを開き直しても保ち、localStorage に覚える。 */
  lighting = $state<LightingSettings>(loadLighting());
  /** 読み込んだ着せ替えのモジュール (読み込んだ順)。モデルを開き直しても残る。 */
  modules = $state<ModuleInfo[]>([]);
  /** 開いているモデル (本体) のスロット。 */
  slots = $state<SlotInfo[]>([]);
  /** 直近の着せ替えの失敗 (利用者に見せる文)。 */
  dressUpError = $state<string | null>(null);

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
  /** 次のフレームで渡すトラッキングの入力 (渡したら `null`)。 */
  #pendingTracking: TrackingFrame | null = null;
  #lastTracking: TrackingFrame | null = null;
  /** 読み込んだモジュールのファイル (名前ごと。GPU を作り直したときに読み直す)。 */
  #moduleFiles = new Map<string, ModelFile>();
  /** 描画の準備ができる前に開いたモジュール。 */
  #pendingModules: ModelFile[] = [];
  /** 付けているモジュールの名前 (合成の順。GPU を作り直したときに付け直す)。 */
  #attached: string[] = [];

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

  /**
   * モデルか着せ替えのモジュールを開く。形式は中身から判別する (modelFile.ts)。
   * モジュールなら読み込んで付ける (`loadModule`)。
   */
  async load(file: ModelFile): Promise<void> {
    const kind = detectModelKind(file);
    if (kind === "module") {
      this.loadModule(file);
      return;
    }
    this.#file = file;
    const puppet = this.#puppet;
    // まだ描画の準備中なら、準備ができたときに読む (#createPuppet)。
    if (puppet === null) return;
    if (kind === null) {
      this.error = `${file.name}: .ugoe・JSON のモデルか .ugoemod のモジュールではありません`;
      return;
    }
    try {
      // 付けていたモジュールは wasm が付け直す。付けられなかったものは外して警告に入る。
      const warnings =
        kind === "bundle"
          ? puppet.load_bundle(file.bytes)
          : puppet.load_json(new TextDecoder().decode(file.bytes));
      this.warnings = describeLoadWarnings(
        warnings,
        parseDetachedModules(puppet.detached_modules_json()),
      );
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
    // WebPuppet は読み直すと Puppet を作り直すので、トラッキングと目パチの設定を渡し直す。
    this.trackingParameterIds = puppet.tracking_parameter_ids();
    puppet.set_enabled("eye_blink", this.autoEyeBlink);
    this.#pendingTracking = this.#lastTracking;
    // 点光源の位置はキャンバスの大きさに対する割合なので、モデルごとに渡し直す。
    this.usesLighting = puppet.uses_lighting();
    this.#applyLighting();
    this.dressUpError = null;
    this.#syncDressUp();
    this.status = "ready";
    this.fit();
  }

  // --- 着せ替え (T0706)。付け外しの規則は docs/spec/modules.md#puppet-での着せ替え ---

  /**
   * `.ugoemod` を読み込む。`attach` なら、モデルを開いていれば続けて付ける (同じスロットのモジュールは差し替わる)。
   * 失敗は `dressUpError` に入れる。読み込めたらモジュールの名前を返す。
   */
  loadModule(file: ModelFile, attach = true): string | null {
    const puppet = this.#puppet;
    if (puppet === null) {
      // 描画の準備ができたら読んで付ける (#createPuppet)。
      this.#pendingModules.push(file);
      return null;
    }
    let name: string;
    try {
      name = puppet.load_module(file.bytes);
    } catch (e) {
      this.dressUpError = `${file.name}: ${describeDressUpError(e)}`;
      return null;
    }
    this.#moduleFiles.set(name, file);
    this.dressUpError = null;
    if (attach && puppet.has_model()) this.attachModule(name);
    else this.#syncDressUp();
    return name;
  }

  /** 読み込んだモジュールを付ける。同じスロットに付いているモジュールとは差し替える。付けられたら `true`。 */
  attachModule(name: string): boolean {
    return this.#dressUp((puppet) => puppet.attach_module(name));
  }

  /** モジュールを外す。 */
  detachModule(name: string): boolean {
    return this.#dressUp((puppet) => puppet.detach_module(name));
  }

  /** 読み込んだモジュールを一覧から除く (付けていれば外す)。 */
  unloadModule(name: string): boolean {
    const done = this.#dressUp((puppet) => puppet.unload_module(name));
    if (done) this.#moduleFiles.delete(name);
    return done;
  }

  /** スロット `slot` に付けるモジュールを `name` にする。`null` ならそのスロットのモジュールを外す。 */
  setSlotModule(slot: string, name: string | null): boolean {
    if (name !== null) return this.attachModule(name);
    const current = this.modules.find((m) => m.slot === slot && m.attached);
    return current === undefined || this.detachModule(current.name);
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

  /**
   * フェイストラッキングの入力を設定する (次のフレームで Rust の対応付けに渡す)。`null` で止め、
   * トラッキングが書き込んでいたパラメータを既定値に戻す。値の約束は `@ugoe/face-tracking` の `frame.ts`。
   */
  setTracking(frame: TrackingFrame | null): void {
    if (frame === null) {
      if (this.#lastTracking === null) return;
      this.#lastTracking = null;
      this.#pendingTracking = null;
      this.tracking = false;
      this.#puppet?.clear_tracking();
      // 最後の値が基準値として残るので、既定値に戻す。
      const ids = new Set(this.trackingParameterIds);
      for (const def of this.#parameters) {
        if (ids.has(def.id)) this.#puppet?.set_parameter(def.id, def.default);
      }
      this.#syncValues();
      return;
    }
    this.#lastTracking = frame;
    this.#pendingTracking = frame;
    this.tracking = true;
  }

  /** モデルの自動の目パチを有効・無効にする (トラッキング中は目の開閉に重なるため止められる)。 */
  setAutoEyeBlink(enabled: boolean): void {
    this.autoEyeBlink = enabled;
    const puppet = this.#puppet;
    if (puppet?.has_model()) puppet.set_enabled("eye_blink", enabled);
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

  /** 光源の設定を変える (次のフレームから効く)。設定は覚えておく。 */
  setLighting(settings: LightingSettings): void {
    this.lighting = settings;
    saveLighting(settings);
    this.#applyLighting();
  }

  /** 光源の設定の一部を書き換える (`edit` は写しを書き換える)。 */
  updateLighting(edit: (settings: LightingSettings) => void): void {
    const next = $state.snapshot(this.lighting) as LightingSettings;
    edit(next);
    this.setLighting(next);
  }

  // --- 内部 ---

  /** 付け外しを 1 つ行い、一覧を読み直す。失敗したら (wasm は何も変えない) 理由を `dressUpError` に入れる。 */
  #dressUp(action: (puppet: WebPuppet) => void): boolean {
    const puppet = this.#puppet;
    if (puppet === null) return false;
    try {
      action(puppet);
      this.dressUpError = null;
      return true;
    } catch (e) {
      this.dressUpError = describeDressUpError(e);
      return false;
    } finally {
      this.#syncDressUp();
    }
  }

  /** 着せ替えの一覧を WebPuppet から読み直す。モジュールのメッシュでライティングの有無も変わる。 */
  #syncDressUp(): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    try {
      this.modules = parseModules(puppet.modules_json());
      this.slots = parseSlots(puppet.slots_json());
    } catch (e) {
      this.dressUpError = message(e);
    }
    this.#attached = puppet.attached_modules();
    if (puppet.has_model()) this.usesLighting = puppet.uses_lighting();
  }

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
      const frame = this.#pendingTracking;
      if (frame !== null) {
        this.#pendingTracking = null;
        setPuppetTracking(puppet, frame);
      }
      const ids = new Set(driven.keys());
      if (this.tracking) {
        // トラッキングは update の中で書き込むので、ドライバの値より優先される。
        for (const id of this.trackingParameterIds) ids.add(id);
      }
      if (ids.size !== this.driven.size || [...ids].some((id) => !this.driven.has(id))) {
        this.driven = ids;
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

  #applyLighting(): void {
    const puppet = this.#puppet;
    if (puppet === null) return;
    const size = puppet.canvas_size();
    const json = toLightingJson(this.lighting, [size[0] ?? 1, size[1] ?? 1]);
    try {
      puppet.set_lighting_json(json === null ? undefined : JSON.stringify(json));
    } catch (e) {
      this.error = `光源を設定できません: ${message(e)}`;
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
    // GPU を作り直したときは、モジュールを読み直してからモデルを読み、付けていたもの (合成の順) を付け直す。
    // 準備中に開いたモジュールは、モデルを読んだ後に付ける。
    const attach = [...this.#attached];
    const known = [...this.#moduleFiles.values()];
    this.#moduleFiles.clear();
    for (const file of known) this.loadModule(file, false);
    for (const file of this.#pendingModules.splice(0)) {
      const name = this.loadModule(file, false);
      if (name !== null) attach.push(name);
    }
    if (this.#file !== null) await this.load(this.#file);
    if (puppet.has_model()) for (const name of attach) this.attachModule(name);
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
