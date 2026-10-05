// ビューポートの状態と操作 (docs/editor.md#ビューポート)。
//
// - モデルの描画は ugoe-wasm の WebPuppet が行う。パラメータの値は表示の状態なので、
//   スライダーはここ (setParameter) を直接呼び、IPC は通らない。
// - 描画は要求されたときだけ requestAnimationFrame で 1 回行う (requestRender)。
// - 座標: 公開するメソッドはすべて CSS ピクセル (ビューポート要素の左上原点)。
//   WebPuppet は canvas の描画バッファの画素 (CSS ピクセル × devicePixelRatio) で受け取るので、ここで換算する。
// - WebPuppet と canvas の寿命 (作成・WebGL2 への切り替え・GPU を失ったときの作り直し・破棄・devicePixelRatio の追従) は
//   @ugoe/player の PuppetSurface に任せる (Player と共有。T0932)。

import { GpuLostRepeatedlyError, PuppetSurface, type Screenshot } from "@ugoe/player";
import { errorText } from "../errors";
import { t } from "../i18n";
import { loadUgoeWasm, type WebPuppet } from "./wasm";

/** ビューポートに表示するモデル。bundle があればテクスチャ付きで、無ければ JSON (白いテクスチャ) で表示する。 */
export interface ViewportModel {
  json: string;
  bundle: Uint8Array | null;
}

export type ViewportStatus = "loading" | "ready" | "unbuilt" | "error";

/** WebPuppet の段 (`set_enabled` の名前)。 */
export type PuppetStage = "physics" | "eye_blink" | "breath" | "lip_sync" | "pose";

/**
 * ビューポートを毎フレーム動かすもの (プレビューモード、lib/preview/)。設定している間はモデルを
 * `update(dt)` で動かし続け、編集用のパラメータの値 (`parameterValues`) を WebPuppet に書かない。
 */
export interface FrameDriver {
  /** 動かす段 (`set_enabled`)。モデルを読み直すたびにも設定し直す。 */
  readonly stages: Record<PuppetStage, boolean>;
  /** モデルを読み込んだ直後 (設定した時と、読み直し・作り直しの後)。値や再生の設定を渡し直す。 */
  loaded(puppet: WebPuppet): void;
  /** `update(dt)` の直前と直後 (描画の前)。 */
  beforeUpdate?(puppet: WebPuppet, dt: number): void;
  afterUpdate(puppet: WebPuppet): void;
}

/** 1 フレームに進める時間の上限 (秒)。タブが裏に回って戻ったときに大きく跳ばないように。 */
const MAX_FRAME_DT = 0.1;

/**
 * 編集中にポーズを見るとき (setPosePreview)、表示する要素を切り替えた後に進める時間 (秒)。クロスフェードを
 * 待たずに切り替わった姿を出すため、ポーズのフェードの時間より長くする (編集中はポーズのほかの段は止めてある)。
 */
const POSE_SETTLE_DT = 3600;

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
  /** 選択中のメッシュ・ワープの深度を色の点で重ねるか (インスペクタの深度の節で切り替える)。 */
  showDepth = $state(false);
  /**
   * 物理のプレビュー中か (docs/editor.md#物理のプレビュー)。プレビュー中は毎フレーム `update(dt)` で
   * 物理を進めて描き続ける。止めると物理を外し、編集中の姿 (スライダーの値) に戻す。
   */
  previewingPhysics = $state(false);
  /**
   * `FrameDriver` (プレビューモード) で動かしているか (docs/editor.md#プレビューモード)。
   * 動かしている間、編集のパネルがスライダーの値を変えても表示に出さない。
   */
  driven = $state(false);
  /** プレビューで 1 フレーム進めるたびに増える (オーバーレイが評価結果を読み直すため)。 */
  animationFrame = $state(0);
  /** 読み込んだモデルにライティングを受けるメッシュがあるか (`WebPuppet.uses_lighting`)。 */
  usesLighting = $state(false);
  /**
   * 編集中のビューポートでポーズを効かせて見ているときの、表示する要素のパーツ (グループごとに 1 つまで。
   * 挙げていないグループは最初の要素)。`null` ならポーズを効かせない (すべての要素が見える。既定)。
   * Poses タブで切り替える (docs/editor.md#ポーズの編集)。プレビューモードはこれを使わない。
   */
  posePreview = $state.raw<readonly string[] | null>(null);

  /** WebPuppet と canvas の寿命 (@ugoe/player の surface.ts)。 */
  readonly #surface = new PuppetSurface<WebPuppet>({
    loadWasm: loadUgoeWasm,
    created: (puppet) => this.#setUp(puppet),
    failed: (e) => {
      this.status = "error";
      this.message =
        e instanceof GpuLostRepeatedlyError ? t("viewport.gpuLostRepeatedly") : errorText(e);
    },
    pixelRatioChanged: () => this.requestRender(),
  });
  #model: ViewportModel | null = null;
  /** プレビュー中だけ代わりに描くモデル (モジュールのプロジェクトの本体だけのもの。setPreviewModel)。 */
  #previewModel: ViewportModel | null = null;
  /** 次に読み込むモデルでパラメータを既定値から始める (resetParametersOnNextModel)。 */
  #resetOnLoad = false;
  #background: [number, number, number, number] = [0, 0, 0, 0];
  #frame = 0;
  /** 前のプレビューのフレームの時刻 (ms)。0 なら最初のフレーム。 */
  #lastTime = 0;
  #animation = 0;
  #driver: FrameDriver | null = null;
  /** 光源の JSON (`set_lighting_json`)。null なら渡さない。WebPuppet を作り直しても渡し直す。 */
  #lighting: string | null = null;

  /** 描画中の WebPuppet (ヒットテストなどの問い合わせ用)。 */
  get puppet(): WebPuppet | null {
    return this.#surface.puppet;
  }

  get #puppet(): WebPuppet | null {
    return this.#surface.puppet;
  }

  /**
   * `container` の中に canvas を作り、WebPuppet を作る。Viewport.svelte がマウント時に呼ぶ。待っている間に detach か
   * 次の attach があれば、作った WebPuppet は捨てる。GPU を失ったら canvas ごと作り直す (PuppetSurface)。
   */
  async attach(container: HTMLElement): Promise<void> {
    this.detach();
    const result = await this.#surface.attach(container);
    if (result === "missing") {
      this.status = "unbuilt";
      this.message = "ugoe-wasm is not built. Run `just wasm` and reload.";
    }
  }

  /** WebPuppet を作った (attach と、GPU を失った後の作り直し)。今の表示の設定とモデルを渡す。 */
  #setUp(puppet: WebPuppet): void {
    this.backend = puppet.backend();
    this.status = "ready";
    this.message = null;
    puppet.set_background(...this.#background);
    this.#applyLighting();
    const shown = this.#shown();
    if (shown) this.#load(shown);
    this.requestRender();
    this.#scheduleAnimation();
  }

  /** WebPuppet を破棄する。Viewport.svelte がアンマウント時に呼ぶ。 */
  detach(): void {
    cancelAnimationFrame(this.#frame);
    this.#frame = 0;
    cancelAnimationFrame(this.#animation);
    this.#animation = 0;
    this.#surface.detach();
  }

  /** 表示するモデルを設定する。モデルが変わるたびに呼ぶ (編集・Undo・Redo・新規・開くの後)。 */
  setModel(model: ViewportModel): void {
    if (model === this.#model) return;
    this.#model = model;
    const shown = this.#shown();
    if (this.#puppet && shown) this.#load(shown);
  }

  /**
   * プレビュー中 (FrameDriver を設定している間) だけ、編集中のモデルの代わりに描くモデル (モジュールのプロジェクトで、
   * 本体だけのモデルに今の編集のモジュールを付けて比べる。T0651)。`null` で編集中のモデルに戻す。
   * プレビューを出ると (setDriver(null)) 忘れる。
   */
  setPreviewModel(model: ViewportModel | null): void {
    if (model === this.#previewModel) return;
    this.#previewModel = model;
    const shown = this.#shown();
    if (this.#driver && this.#puppet && shown) this.#load(shown);
  }

  /** 今描くモデル。 */
  #shown(): ViewportModel | null {
    return (this.#driver ? this.#previewModel : null) ?? this.#model;
  }

  setParameter(id: string, value: number): void {
    this.parameterValues[id] = value;
    // プレビューモードの値は別に持つ。編集用の値は抜けたときに WebPuppet へ書き戻す。
    if (this.#driver) return;
    if (this.#puppet?.set_parameter(id, value)) {
      this.parameterValues[id] = this.#puppet.parameter(id) ?? value;
      this.requestRender();
    }
  }

  /**
   * 物理のプレビューを始める: 粒子を静止姿勢に置き直し、毎フレーム `update(dt)` で物理を進める。
   * パラメータ (スライダー) や頭のデフォーマを動かすと揺れる。
   */
  startPhysicsPreview(): void {
    if (this.previewingPhysics || this.#driver) return;
    this.previewingPhysics = true;
    this.#lastTime = 0;
    this.#puppet?.reset_physics();
    this.#applyStages();
    this.#scheduleAnimation();
  }

  /** 物理のプレビューを止め、編集中の姿 (物理なし・スライダーの値) に戻す。 */
  stopPhysicsPreview(): void {
    if (!this.previewingPhysics) return;
    this.previewingPhysics = false;
    cancelAnimationFrame(this.#animation);
    this.#animation = 0;
    const puppet = this.#puppet;
    if (puppet) {
      puppet.reset_physics();
      this.#applyStages();
      // 物理が書き込んだパラメータ (揺れ用のパラメータ) を、スライダーの値に戻す。
      for (const [id, value] of Object.entries(this.parameterValues)) {
        puppet.set_parameter(id, value);
      }
    }
    this.animationFrame += 1;
    this.requestRender();
  }

  /**
   * 毎フレーム動かすもの (プレビューモード) を設定する。物理のプレビューは止める。
   * `null` で外すと、モデルを読み直して (物理・モーション・表情・目パチなどの状態を初めからにして)
   * 編集中の姿 (`parameterValues`) に戻す。
   */
  setDriver(driver: FrameDriver | null): void {
    if (driver === this.#driver) return;
    this.stopPhysicsPreview();
    this.#driver = driver;
    this.driven = driver !== null;
    cancelAnimationFrame(this.#animation);
    this.#animation = 0;
    this.#lastTime = 0;
    const puppet = this.#puppet;
    if (driver) {
      if (puppet?.has_model()) {
        puppet.reset_physics();
        this.#applyStages();
        driver.loaded(puppet);
      }
      this.#scheduleAnimation();
    } else {
      this.#previewModel = null;
      if (this.#model) this.#load(this.#model);
    }
    this.animationFrame += 1;
    this.requestRender();
  }

  /**
   * 編集中のビューポートで、ポーズを効かせて `parts` の要素を表示する (`null` で効かせない)。モデルを読み直しても
   * 続ける。クロスフェードは待たずに切り替わった姿を出す。
   */
  setPosePreview(parts: readonly string[] | null): void {
    this.posePreview = parts;
    const puppet = this.#puppet;
    if (!puppet?.has_model()) return;
    this.#applyStages();
    this.#applyPosePreview(puppet);
    this.animationFrame += 1;
    this.requestRender();
  }

  /**
   * 編集中のポーズを WebPuppet に渡す。効かせないときも 1 度 `update` して、前のポーズの不透明度の上書きを消す。
   * プレビューモードと物理のプレビューの間は、毎フレームの `update` に任せる (表示する要素だけ伝える)。
   */
  #applyPosePreview(puppet: WebPuppet): void {
    if (this.#driver) return;
    try {
      for (const part of this.posePreview ?? []) puppet.show_pose(part);
      if (!this.previewingPhysics) puppet.update(this.posePreview ? POSE_SETTLE_DT : 0);
    } catch (e) {
      console.warn("ugoe:", e);
    }
  }

  /** 段の有効・無効を設定し直す (FrameDriver の `stages` を変えた後に呼ぶ)。 */
  applyStages(): void {
    this.#applyStages();
  }

  /**
   * 次に読み込むモデル (setModel) では、今の値を引き継がずにパラメータを既定値から始める。
   * 別のモデルに替わるとき (新規・PSD から新規・開く) に、読み直しの前に呼ぶ。
   */
  resetParametersOnNextModel(): void {
    this.#resetOnLoad = true;
  }

  resetParameters(): void {
    if (this.#driver) return;
    this.#puppet?.reset_parameters();
    this.#syncParameters();
    this.requestRender();
  }

  /** ビューポート要素の大きさ (CSS ピクセル)。描画バッファは devicePixelRatio を掛けた画素にする。 */
  resize(width: number, height: number): void {
    this.#surface.resize(width, height);
    this.requestRender();
  }

  /**
   * 光源 (docs/editor.md#ライティングの画面)。`WebPuppet.set_lighting_json` の JSON で、null なら光源を渡さない
   * (環境光が白で光源なし)。モデルを読み直しても保つ。
   */
  setLighting(json: string | null): void {
    if (json === this.#lighting) return;
    this.#lighting = json;
    this.#applyLighting();
    this.requestRender();
  }

  #applyLighting(): void {
    try {
      this.#puppet?.set_lighting_json(this.#lighting ?? undefined);
    } catch (e) {
      console.warn("ugoe: lighting:", e);
    }
  }

  /** 背景色 (ストレートアルファ、各成分 0..1)。 */
  setBackground(r: number, g: number, b: number, a: number): void {
    this.#background = [r, g, b, a];
    this.#puppet?.set_background(r, g, b, a);
    this.requestRender();
  }

  /**
   * 今の表示を画像で読み戻す (スクリーンショット、T0420。PNG への変換は @ugoe/player の encodePng)。`WebPuppet.read_pixels()` で、canvas と同じ
   * 大きさ (描画バッファの画素)・ビュー・背景・光源で別のテクスチャに描いたものを取る。オーバーレイ (ギズモ・選択の輪郭・
   * 光源の取っ手など、HTML と SVG で重ねるもの) は写らない。モデルを読み込んでいなければ `null`。
   */
  async capture(): Promise<Screenshot | null> {
    const puppet = this.#puppet;
    if (puppet === null || this.status !== "ready" || !puppet.has_model()) return null;
    const width = puppet.width();
    const height = puppet.height();
    const pixels = await puppet.read_pixels();
    return { width, height, pixels };
  }

  fit(): void {
    this.#puppet?.fit();
    this.requestRender();
  }

  /** CSS ピクセルの点 `(x, y)` を中心にズームを `factor` 倍する。 */
  zoomAt(factor: number, x: number, y: number): void {
    const dpr = this.#surface.pixelRatio;
    this.#puppet?.zoom_at(factor, x * dpr, y * dpr);
    this.requestRender();
  }

  /** CSS ピクセルで `(dx, dy)` だけずらす。 */
  panBy(dx: number, dy: number): void {
    const dpr = this.#surface.pixelRatio;
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
    // GPU を失った (GPU プロセスのクラッシュ・WebGL2 のコンテキストの喪失など)。canvas ごと作り直す
    // (WebGL2 はコンテキストの復元を待つ)。
    if (!puppet || this.#surface.checkLost()) return;
    try {
      puppet.render();
      this.#updateTransform();
    } catch (e) {
      if (this.#surface.checkLost()) return;
      this.status = "error";
      this.message = errorText(e);
    }
  }

  /**
   * 段の有効・無効を WebPuppet に設定する。編集中は物理・目パチなどを動かさず (`update` を呼ばないので
   * 動かないが、呼ぶときのために止めておく)、物理のプレビュー中は物理だけを動かす。
   */
  #applyStages(): void {
    const puppet = this.#puppet;
    if (!puppet?.has_model()) return;
    const stages: Record<PuppetStage, boolean> = this.#driver?.stages ?? {
      physics: this.previewingPhysics,
      eye_blink: false,
      breath: false,
      lip_sync: false,
      pose: this.posePreview !== null,
    };
    for (const [stage, enabled] of Object.entries(stages)) {
      try {
        puppet.set_enabled(stage, enabled);
      } catch (e) {
        console.warn("ugoe:", e);
      }
    }
  }

  #scheduleAnimation(): void {
    if (this.#animation || !(this.previewingPhysics || this.#driver)) return;
    this.#animation = requestAnimationFrame((now) => {
      this.#animation = 0;
      this.#animate(now);
    });
  }

  /** プレビューの 1 フレーム: 経過時間だけ物理を進めて描く。 */
  #animate(now: number): void {
    const puppet = this.#puppet;
    const driver = this.#driver;
    if (!puppet || !(this.previewingPhysics || driver)) return;
    const dt = this.#lastTime ? Math.min((now - this.#lastTime) / 1000, MAX_FRAME_DT) : 0;
    this.#lastTime = now;
    try {
      if (puppet.has_model()) {
        driver?.beforeUpdate?.(puppet, dt);
        puppet.update(dt);
        driver?.afterUpdate(puppet);
      }
    } catch (e) {
      console.warn("ugoe:", e);
    }
    this.#render();
    this.animationFrame += 1;
    this.#scheduleAnimation();
  }

  #load(model: ViewportModel): void {
    const puppet = this.#puppet;
    if (!puppet) return;
    try {
      const warnings = model.bundle
        ? puppet.load_bundle(model.bundle)
        : puppet.load_json(model.json);
      for (const warning of warnings) console.warn("ugoe:", warning);
      this.usesLighting = puppet.uses_lighting();
      this.#applyStages();
      if (this.#driver) {
        // プレビュー中の読み直し (別の WebPuppet への作り直しなど)。値はプレビューのものを渡す。
        this.#driver.loaded(puppet);
      } else if (this.#resetOnLoad) {
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
      if (!this.#driver) {
        this.#syncParameters();
        if (this.posePreview) this.#applyPosePreview(puppet);
      }
      if (this.status === "error") {
        this.status = "ready";
        this.message = null;
      }
      this.requestRender();
    } catch (e) {
      this.status = "error";
      this.message = errorText(e);
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
    const dpr = this.#surface.pixelRatio;
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
}
