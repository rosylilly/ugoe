// `just wasm` の出力 (generated/ugoe_wasm.d.ts) がまだ無いときに使う型。
// tsconfig.json の paths で "ugoe-wasm" は generated/ を優先し、無ければこのファイルに解決される。
// 生成物があれば本物の型で検査されるので、ここは crates/ugoe-wasm/src/web.rs の公開 API と揃えておく。

export class WebPuppet {
  private constructor();
  free(): void;
  static create(canvas: HTMLCanvasElement, backend?: string | null): Promise<WebPuppet>;
  backend(): string;
  is_lost(): boolean;

  load_json(json: string): string[];
  load_bundle(bytes: Uint8Array): string[];
  unload(): void;
  has_model(): boolean;

  load_module(bytes: Uint8Array): string;
  attach_module(name: string): void;
  detach_module(name: string): void;
  set_modules(names: string[]): void;
  unload_module(name: string): void;
  attached_modules(): string[];
  modules_json(): string;
  slots_json(): string;
  canvas_size(): Float32Array;

  parameter_ids(): string[];
  parameters_json(): string;
  set_parameter(id: string, value: number): boolean;
  parameter(id: string): number | undefined;
  reset_parameters(): void;
  update(dt: number): void;

  set_tracking(
    names: string[],
    scores: Float32Array,
    yaw: number,
    pitch: number,
    roll: number,
    body_yaw?: number | null,
    body_pitch?: number | null,
    body_roll?: number | null,
    shoulder_left?: number | null,
    shoulder_right?: number | null,
  ): void;
  clear_tracking(): void;
  is_tracking(): boolean;
  tracking_parameter_ids(): string[];

  resize(width: number, height: number): void;
  width(): number;
  height(): number;
  set_background(r: number, g: number, b: number, a: number): void;
  render(): void;

  fit(): void;
  set_view(zoom: number, pan_x: number, pan_y: number): void;
  zoom(): number;
  pan_x(): number;
  pan_y(): number;
  zoom_at(factor: number, x: number, y: number): void;
  pan_by(dx: number, dy: number): void;
  view_transform(): Float32Array;
  canvas_to_screen(x: number, y: number): Float32Array;
  screen_to_canvas(x: number, y: number): Float32Array;

  drawable_ids(): string[];
  drawable_bounds(): Float32Array;
  drawable_positions(index: number): Float32Array | undefined;
  render_order(): Uint32Array;
  hit_test(x: number, y: number): number | undefined;
  hit_test_all(x: number, y: number): Uint32Array;

  hit_area_ids(): string[];
  hit_areas_at(x: number, y: number): string[];
  hit_area_test(id: string, x: number, y: number): boolean;

  motion_names(): string[];
  start_motion(name: string, priority?: string | null): boolean;
  start_motion_json(json: string, priority?: string | null): boolean;
  stop_motions(): void;
  is_motion_playing(): boolean;
  motion_events(): string[];
  expression_names(): string[];
  set_expression(name: string): void;
  set_expression_json(json: string): void;
  stop_expressions(): void;
  set_effects_json(json: string): void;
  set_lip_sync_level(level?: number | null): void;
  set_enabled(stage: string, enabled: boolean): void;
  is_enabled(stage: string): boolean;
  set_physics_json(json?: string | null): void;
  physics_json(): string | undefined;
  has_physics(): boolean;
  is_dynamic_physics(): boolean;
  reset_physics(): void;
  physics_particles(rig: number): Float32Array;
  standard_hit_area(name: string): string | undefined;
  hit_test_standard(name: string, x: number, y: number): boolean;
  standard_hit_areas_at(x: number, y: number): string[];

  deformer_ids(): string[];
  warp_points(id: string): Float32Array | undefined;
  rotation_frame(id: string): Float32Array | undefined;
  deformer_point(id: string, x: number, y: number): Float32Array | undefined;
  deformer_jacobian(id: string, x: number, y: number): Float32Array | undefined;
}

export function version(): string;

export function sample_motion_json(json: string, t: number): string;

export default function init(input?: unknown): Promise<unknown>;
