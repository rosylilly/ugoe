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
  canvas_size(): Float32Array;

  parameter_ids(): string[];
  set_parameter(id: string, value: number): boolean;
  parameter(id: string): number | undefined;
  reset_parameters(): void;
  update(dt: number): void;

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
}

export function version(): string;

export default function init(input?: unknown): Promise<unknown>;
