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
  /** 本体のパーツの ID (`part_ids` の先頭。モデルを読んでいなければ `part_ids` は空)。 */
  parts?: string[];
  /** モジュールのパーツ (名前 → パーツの ID。付けると `<名前>/<パーツ>` で本体のパーツの後に付けた順で入る)。 */
  moduleParts?: Record<string, string[]>;
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
    /** `show_pose_immediately` で表示させた要素 (呼んだ順)。 */
    immediate: [] as string[],
    /** ビュー (fit に対する倍率と、fit の位置からのずれ。偽物の変換は `scale = zoom`・`offset = pan`)。 */
    view: { zoom: 1, pan: [0, 0] as [number, number] },
    /**
     * GPU を失ったか (`is_lost`)。真の間は、wasm と同じく読み込みと着せ替えが `code` = `device_lost` の例外を投げる
     * (テストは WebGL2 なら canvas の `webglcontextlost` も送る)。
     */
    lost: false,
    /**
     * 前の `render` の後に形が変わったか (`shape_changed`。wasm と同じく、読み込み・パラメータの書き換え・着せ替え・`update` で
     * 形が動いた (`moving`) ときに立ち、`render` で下りる)。
     */
    shapeChanged: true,
    /** `update` のたびに形が動くか (モーション・物理が動いている。偽なら止まったモデル)。 */
    moving: false,
    /** `render` で描いた回数 (描かずに返った回は数えない)。 */
    renders: 0,
    /**
     * この回数だけ、次からの `render` が描かずに `false` を返す (wasm がサーフェスを作り直した・時間切れのとき。T0983)。
     * そのときは `shapeChanged` を下ろさず、ログには `render (not drawn)` を残す。
     */
    skipRenders: 0,
  };
  /** 失った WebPuppet の読み込み・着せ替えの例外 (ugoe-wasm の `LoadError`・`DressUpError` と同じ形)。 */
  const checkLost = (name: "LoadError" | "DressUpError") => {
    if (state.lost)
      throw Object.assign(new Error("the GPU device was lost"), { name, code: "device_lost" });
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
    adapter_name: () => "Fake GPU",
    is_lost: () => state.lost,
    has_model: () => state.hasModel,
    load_bundle: (bytes: Uint8Array) => {
      checkLost("LoadError");
      // ugoe-wasm の `LoadError` と同じ形 (`name`・`code`)。
      if (options.failLoad)
        throw Object.assign(new Error("not a ugoe file"), {
          name: "LoadError",
          code: "invalid_file",
        });
      loaded.push(bytes);
      state.hasModel = true;
      state.shapeChanged = true;
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
      state.shapeChanged = true;
      return true;
    },
    reset_parameters: () => {
      resetValues();
      state.shapeChanged = true;
    },
    update: () => {
      if (state.moving) state.shapeChanged = true;
    },
    shape_changed: () => state.shapeChanged,
    load_module: (bytes: Uint8Array) => {
      checkLost("DressUpError");
      const { name, slot } = JSON.parse(new TextDecoder().decode(bytes.subarray(16)));
      state.modules.set(name, slot);
      log.push(`load ${name}`);
      return name;
    },
    attach_module: (name: string) => {
      checkLost("DressUpError");
      const slot = state.modules.get(name);
      if (slot === undefined) throw new Error(`${name} is not loaded`);
      state.attached = state.attached.filter((n) => state.modules.get(n) !== slot);
      state.attached.push(name);
      state.shapeChanged = true;
    },
    detach_module: (name: string) => {
      checkLost("DressUpError");
      if (!state.attached.includes(name)) throw new Error(`${name} is not attached`);
      state.attached = state.attached.filter((n) => n !== name);
      state.shapeChanged = true;
    },
    unload_module: (name: string) => {
      checkLost("DressUpError");
      if (!state.modules.delete(name)) throw new Error(`${name} is not loaded`);
      state.attached = state.attached.filter((n) => n !== name);
      log.push(`unload ${name}`);
    },
    attached_modules: () => [...state.attached],
    part_ids: () =>
      state.hasModel
        ? [
            ...(options.parts ?? []),
            ...state.attached.flatMap((name) =>
              (options.moduleParts?.[name] ?? []).map((part) => `${name}/${part}`),
            ),
          ]
        : [],
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
    show_pose_immediately: (part: string) => {
      if (!state.hasModel) throw new Error("no model");
      const group = poseGroups().find((g) => g.includes(part));
      if (group === undefined) return false;
      state.visible.set(group.join(","), part);
      state.immediate.push(part);
      return true;
    },
    // 表示中の要素 (グループの順。テストが `state.visible` を書き換えると、モーションが切り替えたことになる)。
    pose_visible_json: () =>
      state.hasModel
        ? JSON.stringify(poseGroups().map((g) => state.visible.get(g.join(",")) ?? g[0] ?? null))
        : undefined,
    uses_lighting: () => false,
    set_lighting_json: (json?: string | null) => void state.lighting.push(json),
    canvas_size: () => new Float32Array([100, 100]),
    view_transform: () => new Float32Array([state.view.zoom, state.view.pan[0], state.view.pan[1]]),
    fit: () => {
      state.view = { zoom: 1, pan: [0, 0] };
    },
    zoom: () => state.view.zoom,
    pan_x: () => state.view.pan[0],
    pan_y: () => state.view.pan[1],
    set_view: (zoom: number, x: number, y: number) => {
      state.view = { zoom, pan: [x, y] };
    },
    zoom_at: (factor: number, x: number, y: number) => {
      // 点 (x, y) が動かないようにずらす (偽物の変換で)。
      const [px, py] = state.view.pan;
      state.view = {
        zoom: state.view.zoom * factor,
        pan: [x - (x - px) * factor, y - (y - py) * factor],
      };
    },
    pan_by: (dx: number, dy: number) => {
      state.view.pan = [state.view.pan[0] + dx, state.view.pan[1] + dy];
    },
    width: () => 1,
    height: () => 1,
    // 1 画素の読み戻し。赤は先頭のパラメータの今の値 (-1..1 を 0..200 に) で、`render` で描いたかによらず今の状態を写す
    // (wasm の `read_pixels` は canvas を写さず、別のテクスチャに今の状態を描く)。
    read_pixels: async () => {
      const first = parameters[0];
      const value = first === undefined ? 0 : (state.values.get(first.id) ?? 0);
      return new Uint8Array([Math.round((value + 1) * 100), 0, 0, 255]);
    },
    render: (): boolean => {
      if (state.skipRenders > 0) {
        state.skipRenders--;
        log.push("render (not drawn)");
        return false;
      }
      state.shapeChanged = false;
      state.renders++;
      log.push("render");
      return true;
    },
    follow: (
      values: Float32Array,
      dt: number,
      _level?: number | null,
      _strength?: number | null,
      parts?: Uint32Array | null,
    ) =>
      log.push(
        // パーツの不透明度のカーブがあるときだけ、その番号を書く。
        `follow ${[...values]} ${dt}${parts && parts.length > 0 ? ` [${[...parts]}]` : ""}`,
      ),
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
