// スタジオの操作ウィンドウの偽のホスト (`window.__UGOE_STUDIO_HOST__`。apps/studio/README.md#ヘッドレスのブラウザで確かめる)。
//
// - プロファイルは見本のプロファイル (apps/studio/scripts/sample-profile.mjs) に、画面の既定値 (src/lib/inputs.ts・virtualCamera.ts) を
//   足したもの。シーン・パラメータ・モーション・負荷の表示・操作の API の状態を、本物の指揮役が送る形で送る。
// - 書き換えの操作は何もしない (Promise を返すだけ)。知らないメソッドも何もしない。
import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dirname, "..", "..", "..");

/** 偽のホストに渡すデータ (Node の側で作り、ページへ JSON で渡す)。 */
export async function studioFixture(lang) {
  const { sampleProfile, SAMPLES_DIR } = await import(
    join(root, "apps/studio/scripts/sample-profile.mjs")
  );
  const inputs = await import(join(root, "apps/studio/src/lib/inputs.ts"));
  const camera = await import(join(root, "apps/studio/src/lib/virtualCamera.ts"));
  const profile = sampleProfile(SAMPLES_DIR, lang);
  profile.output = {
    ...profile.output,
    tickRate: 60,
    virtualCamera: camera.DEFAULT_CAMERA_PROFILE,
  };
  profile.input = inputs.DEFAULT_INPUT;
  profile.motion = inputs.DEFAULT_MOTION;
  const motion = (name, duration, looping, parameters) => ({
    name,
    duration,
    looping,
    fadeIn: 0.2,
    fadeOut: 0.3,
    parameters,
  });
  return {
    methods: hostMethods(readFileSync(join(root, "apps/studio/src/lib/host.ts"), "utf8")),
    profile,
    path: join(SAMPLES_DIR, "..", "sample-profile.ugoestudio"),
    parameters: [
      { id: "AngleX", name: "Angle X", min: -30, max: 30, default: 0, internal: false },
      { id: "AngleY", name: "Angle Y", min: -30, max: 30, default: 0, internal: false },
      { id: "AngleZ", name: "Angle Z", min: -30, max: 30, default: 0, internal: false },
      { id: "EyeLOpen", name: "Eye L open", min: 0, max: 1, default: 1, internal: false },
      { id: "EyeROpen", name: "Eye R open", min: 0, max: 1, default: 1, internal: false },
      { id: "MouthOpenY", name: "Mouth open", min: 0, max: 1, default: 0, internal: false },
    ],
    motions: [
      motion("idle", 4, true, ["AngleX", "BodyAngleZ"]),
      motion("nod", 1.2, false, ["AngleY"]),
      motion("shake", 1.6, false, ["AngleX"]),
      motion("tilt", 1.5, false, ["AngleZ", "EyeBallX"]),
    ],
    expressions: ["sad", "smile", "surprised"],
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
  const { methods, profile, path, parameters, motions, expressions } = fixture;
  let output = { visible: true, onTop: false, width: 1920, height: 1080 };
  const active = profile.scenes.find((s) => s.id === profile.activeScene) ?? profile.scenes[0];
  const api = {
    enabled: true,
    listening: true,
    port: 28451,
    error: null,
    clients: [],
    pending: [],
    connections: 1,
    source: { enabled: true, listening: true, connections: 1 },
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
    systemLocale: async () => osLocale,
    playMotion: async () => true,
    virtualCameraState: async () => ({ state: "stopped", unavailable: null }),
    apiState: async () => api,
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
      onMessage({ kind: "profile", profile, path, dirty: true, live: active.id, recent: [path] });
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
          pose: ["ArmsDown", "Hat/On"],
        },
        parameters,
        usesLighting: false,
        // ポーズのパネル (T0842) を出すための偽のグループ。
        poseGroups: [
          { parts: ["ArmsDown", "ArmsUp", "Wave"], module: null },
          { parts: ["Hat/On", "Hat/Off"], module: "Hat" },
        ],
      });
      onMessage({
        kind: "animation",
        motions,
        expressions,
        current: null,
        activeExpressions: [],
        idleMotion: null,
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
              renderMs: 1.4,
              renderMaxMs: 3.1,
              dropped: 0,
              stale: false,
              targetFps: null,
            },
          ],
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
