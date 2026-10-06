// プレーヤーが使う ugoe-wasm の口 (crates/ugoe-wasm/src/web.rs の公開 API の一部)。
//
// wasm の読み込みと生成物の置き場所はアプリが決める (`PlayerHost.loadWasm`)。ここは使うものだけを構造で書き、
// アプリの生成物 (`wasm-bindgen` の `ugoe_wasm.d.ts`) の `WebPuppet` がこれを満たすことを、アプリの型検査で確かめる。
import type { TrackingPuppet } from "@ugoe/face-tracking";

/** プレーヤーが使う `WebPuppet` のメソッド。 */
export interface WebPuppet extends TrackingPuppet {
  free(): void;
  backend(): string;
  is_lost(): boolean;

  load_json(json: string): string[];
  load_bundle(bytes: Uint8Array): string[];
  load_warnings_json(): string;
  has_model(): boolean;
  canvas_size(): Float32Array;

  set_lighting_json(json?: string | null): void;
  uses_lighting(): boolean;

  load_module(bytes: Uint8Array): string;
  attach_module(name: string): void;
  detach_module(name: string): void;
  unload_module(name: string): void;
  attached_modules(): string[];
  modules_json(): string;
  slots_json(): string;
  avatar_slots_json(): string;

  parameters_json(): string;
  set_parameter(id: string, value: number): boolean;
  parameter(id: string): number | undefined;
  reset_parameters(): void;
  update(dt: number): void;
  /**
   * 受け手のモード (follower.ts) でだけ使う。値を基準値として書き、物理 → リップシンク → パーツの不透明度 (モーションのカーブと
   * ポーズ) の順に `dt` 秒進める (`lipSyncLevel` を省くとリップシンクを当てない。`lipSyncStrength` を省くと 1。
   * `partOpacityParts` はパーツの番号、`partOpacityValues` は値と重みを交互に並べたもの。省くとカーブなし)。
   * 受け手のモードを使わないアプリ (ビューア) の型の宣言には無くてよいので省ける。
   */
  follow?(
    values: Float32Array,
    dt: number,
    lipSyncLevel?: number | null,
    lipSyncStrength?: number | null,
    partOpacityParts?: Uint32Array | null,
    partOpacityValues?: Float32Array | null,
  ): void;
  reset_physics?(): void;

  clear_tracking(): void;
  tracking_parameter_ids(): string[];
  set_motion_over_tracking(enabled: boolean): void;
  set_tracking_kept(ids: string[]): void;
  motions_json(): string;
  start_motion(name: string, priority?: string | null): boolean;
  stop_motions(): void;
  is_motion_playing(): boolean;
  motion_events_json(): string;
  expressions_json(): string;
  active_expressions(): string[];
  set_expression(name: string): void;
  add_expression(name: string): void;
  stop_expression(name: string): void;
  stop_expressions(): void;
  show_pose(part: string): boolean;
  pose_json(): string | undefined;
  pose_visible_json(): string | undefined;

  set_enabled(stage: string, enabled: boolean): void;

  resize(width: number, height: number): void;
  width(): number;
  height(): number;
  set_background(r: number, g: number, b: number, a: number): void;
  render(): void;
  read_pixels(): Promise<Uint8Array>;

  fit(): void;
  zoom(): number;
  pan_x(): number;
  pan_y(): number;
  set_view(zoom: number, pan_x: number, pan_y: number): void;
  zoom_at(factor: number, x: number, y: number): void;
  pan_by(dx: number, dy: number): void;
  view_transform(): Float32Array;
}

/** 初期化した ugoe-wasm のモジュール (プレーヤーが使う部分)。 */
export interface UgoeWasmModule {
  WebPuppet: {
    /** canvas に描く `WebPuppet` を作る。`backend` は `auto` / `webgl2` など。 */
    create(canvas: HTMLCanvasElement, backend?: string | null): Promise<WebPuppet>;
  };
}
