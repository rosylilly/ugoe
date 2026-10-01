// ugoe-wasm の JS バインディングの型の写し (wasm-bindgen が生成する ugoe_wasm.d.ts から作った)。
// 生成物 (src/wasm/) が無い環境で `pnpm check` を通すためだけに使う (scripts/check.mjs)。
// 配布物には生成した本物の型が入る。crates/ugoe-wasm の公開 API を変えたら、生成物から作り直す。

export class WebPuppet {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  active_expressions(): string[];
  add_expression(name: string): void;
  attach_module(name: string): void;
  attached_modules(): string[];
  backend(): string;
  canvas_size(): Float32Array;
  canvas_to_screen(x: number, y: number): Float32Array;
  clear_tracking(): void;
  static create(canvas: HTMLCanvasElement, backend?: string | null): Promise<WebPuppet>;
  deformer_ids(): string[];
  deformer_jacobian(id: string, x: number, y: number): Float32Array | undefined;
  deformer_point(id: string, x: number, y: number): Float32Array | undefined;
  detach_module(name: string): void;
  detached_modules_json(): string;
  drawable_bounds(): Float32Array;
  drawable_ids(): string[];
  drawable_positions(index: number): Float32Array | undefined;
  drawable_states_json(): string;
  expression_names(): string[];
  expressions_json(): string;
  fit(): void;
  has_model(): boolean;
  has_physics(): boolean;
  height(): number;
  hit_area_ids(): string[];
  hit_area_test(id: string, x: number, y: number): boolean;
  hit_areas_at(x: number, y: number): string[];
  hit_test(x: number, y: number): number | undefined;
  hit_test_all(x: number, y: number): Uint32Array;
  hit_test_standard(name: string, x: number, y: number): boolean;
  is_dynamic_physics(): boolean;
  is_enabled(stage: string): boolean;
  is_lost(): boolean;
  is_motion_over_tracking(): boolean;
  is_motion_playing(): boolean;
  is_tracking(): boolean;
  load_bundle(bytes: Uint8Array): string[];
  load_json(json: string): string[];
  load_module(bytes: Uint8Array): string;
  load_warnings_json(): string;
  modules_json(): string;
  motion_events(): string[];
  motion_names(): string[];
  motions_json(): string;
  pan_by(dx: number, dy: number): void;
  pan_x(): number;
  pan_y(): number;
  parameter(id: string): number | undefined;
  parameter_ids(): string[];
  parameters_json(): string;
  physics_json(): string | undefined;
  physics_particles(rig: number): Float32Array;
  read_pixels(): Promise<Uint8Array>;
  render(): void;
  render_order(): Uint32Array;
  reset_parameters(): void;
  reset_physics(): void;
  resize(width: number, height: number): void;
  rotation_frame(id: string): Float32Array | undefined;
  screen_to_canvas(x: number, y: number): Float32Array;
  set_background(r: number, g: number, b: number, a: number): void;
  set_effects_json(json: string): void;
  set_enabled(stage: string, enabled: boolean): void;
  set_expression(name: string): void;
  set_expression_json(json: string): void;
  set_lighting_json(json?: string | null): void;
  set_lip_sync_level(level?: number | null): void;
  set_modules(names: string[]): void;
  set_motion_over_tracking(enabled: boolean): void;
  set_parameter(id: string, value: number): boolean;
  set_physics_json(json?: string | null): void;
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
  set_tracking_kept(ids: string[]): void;
  set_view(zoom: number, pan_x: number, pan_y: number): void;
  slots_json(): string;
  show_pose(part: string): boolean;
  standard_hit_area(name: string): string | undefined;
  standard_hit_areas_at(x: number, y: number): string[];
  start_motion(name: string, priority?: string | null): boolean;
  start_motion_json(json: string, priority?: string | null): boolean;
  stop_expression(name: string): void;
  stop_expressions(): void;
  stop_motions(): void;
  tracking_kept(): string[];
  tracking_parameter_ids(): string[];
  unload(): void;
  unload_module(name: string): void;
  update(dt: number): void;
  uses_lighting(): boolean;
  view_transform(): Float32Array;
  warp_points(id: string): Float32Array | undefined;
  width(): number;
  zoom(): number;
  zoom_at(factor: number, x: number, y: number): void;
}

export function sample_motion_json(json: string, t: number): string;

export function version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

export default function __wbg_init(
  module_or_path?:
    | { module_or_path: InitInput | Promise<InitInput> }
    | InitInput
    | Promise<InitInput>,
): Promise<InitOutput>;
