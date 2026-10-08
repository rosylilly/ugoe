// スタジオの操作ウィンドウの偽のホスト (`window.__UGOE_STUDIO_HOST__`。apps/studio/README.md#ヘッドレスのブラウザで確かめる)。
//
// - プロファイルは見本のプロファイル (apps/studio/scripts/sample-profile.mjs) に、画面の既定値 (src/lib/inputs.ts・virtualCamera.ts) を
//   足したもの。シーン・パラメータ・モーション・負荷の表示・操作の API の状態を、本物の指揮役が送る形で送る。
// - 配信中のシーン「雑談」のモデル (assets/samples/avatar.ugoe) のパラメータ・モーション・表情・ポーズのグループは、見本を作る
//   ソース (crates/ugoe-cli/src/sample.rs と sample/animations/) から読む (本物の見本と同じにする。T0047)。
// - 書き換えの操作は何もしない (Promise を返すだけ)。知らないメソッドも何もしない。操作 API の承認・拒否・取り消しだけは、
//   承認の後の知らせを撮るために様子を変えて送り直す。
// - 場面を足すために、ページの `window.__uiAuditStudio` に通知を送る口 (`emit`)、負荷の数値を変える口 (`setStatus`)、
//   出力ウィンドウの様子を変えて送る口 (`setOutput`。透過が効かない画面、T0849) を置く。
//   送る通知の組 (初めて起動した画面・注意の要る状態) は下の `firstRunMessages`・`attentionMessages` で Node の側で作る (T0818)。
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dirname, "..", "..", "..");

/** 偽のホストに渡すデータ (Node の側で作り、ページへ JSON で渡す)。 */
export async function studioFixture(lang) {
  const { resolveSampleProfile, sampleProfile, SAMPLES_DIR, PROFILE_FILES } = await import(
    join(root, "apps/studio/scripts/sample-profile.mjs")
  );
  const inputs = await import(join(root, "apps/studio/src/lib/inputs.ts"));
  const camera = await import(join(root, "apps/studio/src/lib/virtualCamera.ts"));
  const profile = resolveSampleProfile(sampleProfile(lang));
  profile.output = {
    ...profile.output,
    tickRate: 60,
    renderScale: 1,
    virtualCamera: camera.DEFAULT_CAMERA_PROFILE,
  };
  profile.input = inputs.DEFAULT_INPUT;
  profile.motion = inputs.DEFAULT_MOTION;
  const avatar = sampleAvatar();
  return {
    methods: hostMethods(readFileSync(join(root, "apps/studio/src/lib/host.ts"), "utf8")),
    profile,
    // コミットした見本のプロファイルの場所 (docs/manual/studio.md#見本のプロファイル)。拡張子が保存のダイアログが付けるもの
    // (lib.rs の PROFILE_EXTENSION) と同じことは test/hosts.test.mjs が確かめる
    path: join(SAMPLES_DIR, PROFILE_FILES[lang] ?? PROFILE_FILES.ja),
    ...avatar,
    api: {
      enabled: true,
      listening: true,
      port: 28451,
      error: null,
      clients: [],
      pending: [],
      connections: 1,
      source: { enabled: true, listening: true, connections: 1 },
    },
  };
}

/** スタジオがプロファイルに付ける拡張子 (apps/studio/src-tauri/src/lib.rs の `PROFILE_EXTENSION`)。 */
export function profileExtension() {
  const source = readFileSync(join(root, "apps/studio/src-tauri/src/lib.rs"), "utf8");
  const m = source.match(/^const PROFILE_EXTENSION: &str = "(\w+)";$/m);
  if (!m) throw new Error("PROFILE_EXTENSION is not found in apps/studio/src-tauri/src/lib.rs");
  return m[1];
}

/**
 * 見本のアバター (`ugoe sample avatar` の avatar.ugoe) を読んだ指揮役が送るもの: パラメータ (名前は ID、値域は標準の値)・
 * モーション・表情・ポーズのグループ。ソース (crates/ugoe-cli/src/sample.rs・sample/animations/、ugoe-model の standard.rs) から作る。
 */
export function sampleAvatar() {
  const read = (path) => readFileSync(join(root, path), "utf8");
  const sample = read("crates/ugoe-cli/src/sample.rs");
  const standard = read("crates/ugoe-model/src/standard.rs");
  const section = (source, start) => {
    const at = source.indexOf(start);
    if (at < 0) throw new Error(`${start} is not found`);
    return source.slice(at, source.indexOf("];", at));
  };
  // 標準パラメータの定数 (`ANGLE_X` → `AngleX`) と値域 (`(ANGLE_X, -30.0, 0.0, 30.0)`)
  const ids = new Map(
    [...standard.matchAll(/^pub const ([A-Z0-9_]+): &str = "(\w+)";$/gm)].map((m) => [m[1], m[2]]),
  );
  const ranges = new Map(
    [...standard.matchAll(/^ {4}\(([A-Z0-9_]+), (-?[\d.]+), (-?[\d.]+), (-?[\d.]+)\),$/gm)].map(
      (m) => [ids.get(m[1]), { min: Number(m[2]), default: Number(m[3]), max: Number(m[4]) }],
    ),
  );
  const parameters = [
    ...section(sample, "const PARAMETERS: &[&str] = &[").matchAll(/sp::([A-Z0-9_]+),/g),
  ].map((m) => {
    const id = ids.get(m[1]);
    const range = ranges.get(id);
    if (!range) throw new Error(`no standard range for ${m[1]}`);
    // sample.rs の add_parameters: 名前は ID のまま
    return {
      id,
      name: id,
      min: range.min,
      max: range.max,
      default: range.default,
      internal: false,
    };
  });
  const constant = (name) => {
    const m = sample.match(new RegExp(`^const ${name}: &str = "([\\w-]+)";$`, "m"));
    if (!m) throw new Error(`${name} is not found in crates/ugoe-cli/src/sample.rs`);
    return m[1];
  };
  const animations = read("crates/ugoe-cli/src/sample/animations.rs");
  const files = (start) =>
    [...section(animations, start).matchAll(/include_str!\("([^"]+)"\)/g)].map((m) =>
      JSON.parse(read(join("crates/ugoe-cli/src/sample", m[1]))),
    );
  // apps/studio/src-tauri/src/animation.rs の MotionInfo::of (動かすパラメータは重なりなし、カーブの順)
  const motions = files("const MOTIONS:").map((m) => ({
    name: m.name,
    duration: m.duration,
    looping: m.looping ?? false,
    fadeIn: m.fade_in,
    fadeOut: m.fade_out,
    parameters: [
      ...new Set(m.curves.filter((c) => c.target.kind === "parameter").map((c) => c.target.id)),
    ],
  }));
  const expressions = files("const AVATAR_EXPRESSIONS:").map((e) => e.name);
  // 先頭の要素が最初に表示される (conductor.rs の pose_groups。モジュールの要素ではない)
  const poseGroups = [
    { parts: [constant("EARWEAR_PART"), constant("NO_EARWEAR_PART")], module: null },
  ];
  return { parameters, motions, expressions, poseGroups };
}

/**
 * 初めて起動した画面の通知 (新しいプロファイル: 名前の無いモデルの無いシーンが 1 つ、保存先なし、ホットキーなし、配信していない。
 * apps/studio/src-tauri/src/profile.rs の `Profile::new` と stage.rs の `new_profile`)。
 */
export function firstRunMessages(fixture) {
  const base = fixture.profile.scenes[0];
  const scene = {
    ...base,
    id: "scene-1",
    name: "",
    model: null,
    modules: [],
    lighting: null,
    transform: { x: 0, y: 0, scale: 1, rotation: 0 },
    background: { kind: "transparent" },
    pose: [],
  };
  const profile = { ...fixture.profile, scenes: [scene], activeScene: "scene-1", hotkeys: [] };
  return [
    { kind: "profile", profile, path: null, dirty: false, live: null, recent: [] },
    // 承認したクライアントは無い (前の場面の「取り消す」の知らせを残さない)
    { kind: "api", ...fixture.api },
    {
      kind: "scene",
      scene: {
        kind: "scene",
        revision: 2,
        modelRevision: 0,
        modulesRevision: 0,
        modelName: "",
        parameterIds: [],
        partIds: [],
        modules: [],
        lighting: null,
        transform: scene.transform,
        background: scene.background,
        pose: [],
        renderScale: 1,
      },
      parameters: [],
      usesLighting: false,
      poseGroups: [],
      poseShown: [],
    },
    {
      kind: "animation",
      motions: [],
      expressions: [],
      current: null,
      activeExpressions: [],
      idleMotion: null,
      tracking: false,
      trackingPaused: false,
      lipSyncPaused: false,
      trackingParameters: [],
      slots: [],
    },
    { kind: "hotkeys", problems: [], suspended: false, wayland: false },
  ];
}

/** 承認を待つクライアント (`attentionMessages` と、承認の後の知らせを撮る場面で使う)。 */
export const PENDING_CLIENT = { id: "ui-audit-deck", name: "Stream Deck", permission: "control" };

/**
 * 注意の要る状態の通知 (T0818): 承認を待つクライアント・登録できないホットキー (同じキーの 2 つ目と、OS が使っているもの)・
 * 負荷の注意 (刻みが遅れ、間隔が揺れる)。`status` は `setStatus` に渡す負荷の数値。
 */
export function attentionMessages(fixture) {
  const hotkeys = structuredClone(fixture.profile.hotkeys);
  // 2 つ目のシーンの切り替えを 1 つ目と同じキーにする (バックエンドは前のものだけを登録し、後のものに `duplicate` を出す)
  hotkeys[1].keys = hotkeys[0].keys;
  const profile = { ...fixture.profile, hotkeys };
  const active = profile.scenes.find((s) => s.id === profile.activeScene) ?? profile.scenes[0];
  return {
    messages: [
      {
        kind: "profile",
        profile,
        path: fixture.path,
        dirty: false,
        live: active.id,
        recent: [fixture.path],
      },
      { kind: "api", ...fixture.api, pending: [PENDING_CLIENT] },
      {
        kind: "hotkeys",
        problems: [
          [1, { code: "duplicate", other: 0 }],
          [8, { code: "registerFailed", message: "HotKey already registered" }],
        ],
        suspended: false,
        wayland: false,
      },
    ],
    status: { measuredRate: 51.2, maxIntervalMs: 48.5 },
  };
}

/** `export interface StudioHost { ... }` のメソッドの名前 (偽のホストに全部を置く。足りないと画面がエラーを出す)。 */
export function hostMethods(source) {
  const start = source.indexOf("export interface StudioHost {");
  if (start < 0) throw new Error("StudioHost is not found in apps/studio/src/lib/host.ts");
  const body = source.slice(start, source.indexOf("\n}", start));
  return [...body.matchAll(/^ {2}([a-zA-Z]+)\(/gm)].map((m) => m[1]);
}

/** ページの中で動かす初期化 (page.addInitScript に渡す)。 */
export function studioPageInit({ osLocale, fixture }) {
  const { methods, profile, path, parameters, motions, expressions, poseGroups } = fixture;
  let output = {
    visible: true,
    onTop: false,
    width: 1920,
    height: 1080,
    transparency: "supported",
  };
  const active = profile.scenes.find((s) => s.id === profile.activeScene) ?? profile.scenes[0];
  let api = fixture.api;
  /** 使う GPU の様子 (T0860)。GPU の優先度 (T0861) は、当てられなかった知らせも見えるように「低い」で 1 つ失敗した形にする */
  let gpu = {
    preference: "highPerformance",
    launched: "auto",
    graphicsSettings: true,
    priority: "low",
    prioritySupported: true,
    priorityErrors: [
      {
        code: "gpu_priority.access_denied",
        args: { process: "msedgewebview2 (GPU, 4242)" },
        message: "access to msedgewebview2 (GPU, 4242) was denied when changing its GPU priority",
      },
    ],
  };
  /** 指揮役の通知を受ける画面の口 (`subscribeStatus` で受け取る)。 */
  let listener = null;
  /** 負荷の数値に重ねるもの (`setStatus`)。 */
  let statusPatch = {};
  /** 最後に送ったプロファイルの通知 (ホットキーを変えたら、割り当てを替えて送り直す)。 */
  let lastProfile = null;
  const sendApi = () => {
    listener?.({ kind: "api", ...api });
    return api;
  };
  window.__uiAuditStudio = {
    emit(message) {
      if (message.kind === "profile") lastProfile = message;
      if (message.kind === "api") {
        const { kind: _, ...view } = message;
        api = view;
      }
      listener?.(message);
    },
    setStatus(patch) {
      statusPatch = patch;
    },
    setOutput(patch) {
      output = { ...output, ...patch };
      listener?.({ kind: "output", ...output });
    },
  };
  const ids = parameters.map((p) => p.id);
  const given = {
    pickFile: async () => null,
    pickSavePath: async () => null,
    saveProfile: async () => null,
    addScene: async () => "new-scene",
    duplicateScene: async () => "new-scene",
    moduleSlot: async () => "Hair",
    setParameter: async () => true,
    outputState: async () => output,
    setOutputVisible: async (visible) => {
      output = { ...output, visible };
      return output;
    },
    setOutputOnTop: async (onTop) => {
      output = { ...output, onTop };
      return output;
    },
    setOutputSize: async (width, height) => {
      output = { ...output, width, height };
      return output;
    },
    tickRate: async () => 60,
    setTickRate: async (rate) => rate,
    // 使う GPU (T0860)。起動し直しの知らせとグラフィックの設定のボタンも見えるように、Windows で選び直した形にする
    gpuSettings: async () => gpu,
    setGpuPreference: async (preference) => {
      gpu = { ...gpu, preference };
      return gpu;
    },
    setRenderScale: async () => {},
    setGpuPriority: async (priority) => {
      gpu = { ...gpu, priority, priorityErrors: [] };
      return gpu;
    },
    systemLocale: async () => osLocale,
    playMotion: async () => true,
    virtualCameraState: async () => ({ state: "stopped", unavailable: null }),
    apiState: async () => api,
    async approveApiClient(id, permission) {
      const client = api.pending.find((c) => c.id === id);
      if (client === undefined) return id;
      // 本物のバックエンドと同じく、承認したクライアントには新しい ID を付ける
      const approved = `${id}-approved`;
      api = {
        ...api,
        pending: api.pending.filter((c) => c.id !== id),
        clients: [
          ...api.clients,
          {
            id: approved,
            name: client.name,
            permission,
            approvedAt: 1767225600,
            lastSeen: null,
            connections: 1,
          },
        ],
      };
      sendApi();
      return approved;
    },
    async denyApiClient(id) {
      api = { ...api, pending: api.pending.filter((c) => c.id !== id) };
      sendApi();
    },
    // ホットキーの削除と「元に戻す」を撮る (T0845)。割り当てを替えたプロファイルを送り直す
    async setHotkeys(hotkeys) {
      if (lastProfile === null) return;
      lastProfile = { ...lastProfile, profile: { ...lastProfile.profile, hotkeys } };
      listener?.(lastProfile);
    },
    async revokeApiClient(id) {
      api = { ...api, clients: api.clients.filter((c) => c.id !== id) };
      sendApi();
    },
    setApiEnabled: async () => api,
    setApiPort: async () => api,
    setSourceEnabled: async () => api,
    apiToken: async () => "ui-audit-token",
    regenerateApiToken: async () => "ui-audit-token",
    sourceUrl: async () => "http://127.0.0.1:28451/source/ui-audit/",
    regenerateSourceToken: async () => "ui-audit",
    subscribeFrames: () => new Promise(() => {}),
    sceneModel: async () => new Uint8Array(),
    sceneModule: async () => new Uint8Array(),
    sceneBackground: async () => new Uint8Array(),
    async subscribeStatus(onMessage) {
      listener = onMessage;
      // シーンを出すと、ポーズの選択はグループごとの先頭の要素で埋まる (conductor.rs の present)
      const shown = poseGroups.map((g) => g.parts[0]);
      const idle = motions.find((m) => m.name.toLowerCase() === "idle")?.name ?? null;
      lastProfile = {
        kind: "profile",
        profile,
        path,
        dirty: true,
        live: active.id,
        recent: [path],
      };
      onMessage(lastProfile);
      onMessage({ kind: "api", ...api });
      onMessage({
        kind: "scene",
        scene: {
          kind: "scene",
          revision: 1,
          modelRevision: 1,
          modulesRevision: 1,
          modelName: active.model?.split(/[\\/]/).pop() ?? "",
          parameterIds: ids,
          modules: [],
          lighting: active.lighting,
          transform: active.transform,
          background: active.background,
          pose: shown,
          renderScale: 1,
        },
        parameters,
        usesLighting: false,
        poseGroups,
        poseShown: shown,
      });
      onMessage({
        kind: "animation",
        motions,
        expressions,
        // 既定の設定 (autoIdle) では、名前が idle のモーションを待機として流す (animation.rs の default_idle_motion)
        current: idle ? { name: idle, priority: "idle" } : null,
        activeExpressions: [],
        idleMotion: idle,
        tracking: false,
        trackingPaused: false,
        lipSyncPaused: false,
        trackingParameters: [],
        slots: [],
      });
      onMessage({ kind: "hotkeys", problems: [], suspended: false, wayland: false });
      let seq = 0;
      const status = () => {
        seq += 1;
        onMessage({
          kind: "status",
          seq,
          values: parameters.map((p) => p.default),
          tickMs: 0.18,
          outputs: 1,
          tickRate: 60,
          measuredRate: 59.8,
          maxIntervalMs: 17.2,
          renderers: [
            {
              label: "output",
              fps: 59.6,
              refreshFps: 60,
              renderMs: 1.4,
              renderMaxMs: 3.1,
              dropped: 0,
              backend: "webgpu",
              adapter: "Integrated GPU",
              stale: false,
              targetFps: null,
            },
          ],
          ...statusPatch,
        });
      };
      status();
      setInterval(status, 250);
    },
  };
  const noop = async () => {};
  window.__UGOE_STUDIO_HOST__ = Object.fromEntries(
    methods.map((name) => [name, given[name] ?? noop]),
  );
}
