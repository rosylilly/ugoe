// `pnpm wasm` の生成物 (generated/ugoe_wasm.d.ts) が無いときに型検査で使う宣言。
// tsconfig.json の paths で "ugoe-wasm" は generated/ を優先し、無ければこのファイルになる。
// ビューアが使うものだけを、crates/ugoe-wasm/src/web.rs の公開 API に合わせて書く。

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
  canvas_size(): Float32Array;

  set_lighting_json(json?: string | null): void;
  uses_lighting(): boolean;

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
  set_enabled(stage: string, enabled: boolean): void;
  is_enabled(stage: string): boolean;

  resize(width: number, height: number): void;
  set_background(r: number, g: number, b: number, a: number): void;
  render(): void;

  fit(): void;
  zoom(): number;
  zoom_at(factor: number, x: number, y: number): void;
  pan_by(dx: number, dy: number): void;
}

export function version(): string;

export default function init(input?: unknown): Promise<unknown>;
