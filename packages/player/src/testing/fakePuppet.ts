// テスト用の部品 (配布物に入らない。src/index.ts からは出さない)。偽の WebPuppet (Proxy で、Player が使うメソッドだけを
// 状態付きで真似る) と、Player に渡す偽の canvas とファイル。
import type { FileLocation, ModelFile } from "../modelFile";
import type { WebPuppet } from "../wasm";

/** `document.createElement` が返す canvas の代わり。 */
export class FakeCanvas extends EventTarget {
  className = "";
  removed = false;
  remove(): void {
    this.removed = true;
  }
}

export interface FakeOptions {
  failLoad?: boolean;
  /** モデルのパラメータ (既定値は省くと 0)。 */
  parameters?: { id: string; default?: number }[];
  /** モデル (バンドル) のモーション。 */
  motions?: { name: string; looping?: boolean; parameters?: string[] }[];
  /** トラッキングが書き込むパラメータ。 */
  trackingIds?: string[];
  /** 受け手の口 (`follow`・`reset_physics`) を持つか (省くと持つ)。 */
  follower?: boolean;
  /** 本体のポーズのグループ (要素のパーツの並び)。 */
  pose?: string[][];
  /** モジュールのポーズのグループ (名前 → 要素のパーツの並び。付けると `<名前>/<パーツ>` で本体のグループの後に入る)。 */
  modulePoses?: Record<string, string[][]>;
}

/**
 * 偽の WebPuppet。`calls` には `free` を、`loaded` には読んだバイト列を、`log` には着せ替え・モーション・トラッキング・
 * 受け手の呼び出しを残す。`state` はテストが書き換えてよい (`playing` を偽にするとモーションが終わったことになる)。
 * 着せ替えは wasm と同じく、モデルを読み直しても読み込んだモジュールと付けていたものを保つ。
 */
export function fakePuppet(options: FakeOptions = {}) {
  const calls: string[] = [];
  const loaded: Uint8Array[] = [];
  const log: string[] = [];
  const parameters = (options.parameters ?? []).map((p) => ({
    id: p.id,
    name: p.id,
    min: -1,
    max: 1,
    default: p.default ?? 0,
  }));
  const state = {
    hasModel: false,
    /** モーションを再生しているか。 */
    playing: false,
    /** `start_motion` が始めるか (偽なら優先度が低くて始まらなかったことにする)。 */
    accept: true,
    /** 次に `motion_events_json` が返すイベント (`[{value, time, motion, number}]`)。 */
    events: [] as unknown[],
    values: new Map<string, number>(),
    /** 読み込んだモジュール (名前 → スロット。読み込んだ順)。 */
    modules: new Map<string, string>(),
    attached: [] as string[],
    /** `set_lighting_json` に渡されたもの。 */
    lighting: [] as (string | null | undefined)[],
    /** 次にモデルを読んだとき、付けられずに外れるモジュール (モデルが書き換わって合わなくなった)。 */
    unfit: [] as string[],
    /**
     * ポーズのグループごとに表示している要素 (要素の並びを `,` でつないだもの → パーツ)。無いグループは先頭の要素。
     * モデルを読むと空になり、付け外しでは要素の並びが同じグループの分が残る (Puppet と同じ)。
     */
    visible: new Map<string, string>(),
  };
  /** 今のポーズのグループ (本体 → 付けたモジュールの順)。 */
  const poseGroups = (): string[][] => [
    ...(options.pose ?? []),
    ...state.attached.flatMap((name) =>
      (options.modulePoses?.[name] ?? []).map((g) => g.map((part) => `${name}/${part}`)),
    ),
  ];
  const resetValues = () => {
    for (const p of parameters) state.values.set(p.id, p.default);
  };
  const methods: Record<string, ((...args: never[]) => unknown) | undefined> = {
    free: () => calls.push("free"),
    backend: () => "webgl2",
    is_lost: () => false,
    has_model: () => state.hasModel,
    load_bundle: (bytes: Uint8Array) => {
      // ugoe-wasm の `LoadError` と同じ形 (`name`・`code`)。
      if (options.failLoad)
        throw Object.assign(new Error("not a ugoe file"), {
          name: "LoadError",
          code: "invalid_file",
        });
      loaded.push(bytes);
      state.hasModel = true;
      state.playing = false;
      resetValues();
      state.visible.clear();
      state.attached = state.attached.filter((n) => !state.unfit.includes(n));
      return [];
    },
    load_warnings_json: () => "[]",
    parameters_json: () => JSON.stringify({ parameters, groups: [] }),
    parameter: (id: string) => state.values.get(id),
    set_parameter: (id: string, value: number) => {
      if (!state.values.has(id)) return false;
      state.values.set(id, value);
      return true;
    },
    reset_parameters: resetValues,
    load_module: (bytes: Uint8Array) => {
      const { name, slot } = JSON.parse(new TextDecoder().decode(bytes.subarray(16)));
      state.modules.set(name, slot);
      log.push(`load ${name}`);
      return name;
    },
    attach_module: (name: string) => {
      const slot = state.modules.get(name);
      if (slot === undefined) throw new Error(`${name} is not loaded`);
      state.attached = state.attached.filter((n) => state.modules.get(n) !== slot);
      state.attached.push(name);
    },
    detach_module: (name: string) => {
      if (!state.attached.includes(name)) throw new Error(`${name} is not attached`);
      state.attached = state.attached.filter((n) => n !== name);
    },
    unload_module: (name: string) => {
      if (!state.modules.delete(name)) throw new Error(`${name} is not loaded`);
      state.attached = state.attached.filter((n) => n !== name);
      log.push(`unload ${name}`);
    },
    attached_modules: () => [...state.attached],
    modules_json: () =>
      JSON.stringify(
        [...state.modules].map(([name, slot]) => ({
          name,
          slot,
          attached: state.attached.includes(name),
        })),
      ),
    slots_json: () => "[]",
    avatar_slots_json: () => "null",
    motions_json: () =>
      JSON.stringify(
        (options.motions ?? []).map((m) => ({
          name: m.name,
          duration: 1,
          looping: m.looping ?? false,
          fade_in: 0,
          fade_out: 0,
          parameters: m.parameters ?? [],
        })),
      ),
    start_motion: (name: string, priority: string) => {
      log.push(`start ${name} ${priority}`);
      if (!state.accept) return false;
      state.playing = true;
      return true;
    },
    stop_motions: () => {
      log.push("stop");
      state.playing = false;
    },
    is_motion_playing: () => state.playing,
    expressions_json: () => "[]",
    tracking_parameter_ids: () => options.trackingIds ?? [],
    clear_tracking: () => log.push("clear_tracking"),
    set_motion_over_tracking: (enabled: boolean) => log.push(`over ${enabled}`),
    set_tracking_kept: (ids: string[]) => log.push(`kept ${ids.join(",")}`),
    active_expressions: () => [],
    motion_events_json: () => {
      // 読むと空にする (Player は update の直後に 1 度だけ読むので、次の update で発火したことになる)。
      const fired = state.events;
      state.events = [];
      return JSON.stringify(fired);
    },
    pose_json: () =>
      state.hasModel
        ? JSON.stringify({
            fade_duration: 0.5,
            groups: poseGroups().map((g) => ({ entries: g.map((part) => ({ part, links: [] })) })),
          })
        : undefined,
    show_pose: (part: string) => {
      if (!state.hasModel) throw new Error("no model");
      const group = poseGroups().find((g) => g.includes(part));
      if (group === undefined) return false;
      state.visible.set(group.join(","), part);
      return true;
    },
    uses_lighting: () => false,
    set_lighting_json: (json?: string | null) => void state.lighting.push(json),
    canvas_size: () => new Float32Array([100, 100]),
    view_transform: () => new Float32Array([1, 0, 0]),
    zoom: () => 1,
    render: () => log.push("render"),
    follow: (values: Float32Array, dt: number) => log.push(`follow ${[...values]} ${dt}`),
    reset_physics: () => log.push("reset_physics"),
  };
  if (options.follower === false) {
    methods.follow = undefined;
    methods.reset_physics = undefined;
  }
  // ほかのメソッド (resize・set_background など) は何もしない。`then` は無い (約束の値として解くため)。
  const puppet = new Proxy(methods, {
    get: (target, key: string) =>
      key in target ? target[key] : key === "then" ? undefined : () => undefined,
  }) as unknown as WebPuppet;
  return { puppet, calls, loaded, log, state };
}

/** 中身が無いバンドル (形式の判別だけ通る)。 */
export function bundle(name: string, tag: number, location?: FileLocation): ModelFile {
  const file: ModelFile = { name, bytes: new Uint8Array([0x55, 0x47, 0x4f, 0x45, tag]) };
  if (location !== undefined) file.location = location;
  return file;
}

/** 着せ替えのモジュール (偽の `load_module` が名前とスロットを読む)。 */
export function moduleFile(name: string, slot: string, location?: FileLocation): ModelFile {
  const json = new TextEncoder().encode(JSON.stringify({ name, slot }));
  const bytes = new Uint8Array(16 + json.length);
  bytes.set([0x55, 0x47, 0x4f, 0x45]);
  bytes.set(json, 16);
  const file: ModelFile = { name: `${name}.ugoemod`, bytes };
  if (location !== undefined) file.location = location;
  return file;
}
