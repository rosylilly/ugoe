<!--
  操作ウィンドウ (docs/studio.md#操作ウィンドウ)。モデルは描かない (描画は出力ウィンドウに任せる)。
  左にシーンの一覧、中央に配信中に触るもの (いまの状態の印・配信しているシーン・表情とモーション・ポーズ (T0842))、
  右のパネルに設定。並びは初めての配信の準備の順 (シーンの編集 (T0802)・入力 (カメラ・マイク)・出力 (出力ウィンドウ・ブラウザソース・仮想カメラ)) の後に、
  一度決めれば触らないもの (ホットキー (T0803)・モーションの設定・操作 API (T0805)・負荷 (LoadPanel.svelte)・パラメータ) を置く (T0818)。
  幅が 960px 以下では列を積んで画面の全体を送る (T0817)。
  操作 API に初めて接続したクライアントの承認は、どのパネルを開いていても上の知らせ (ApiApproval.svelte) で行う。
  カメラとマイクはこのウィンドウの WebView で取り、バックエンドの刻み (`subscribe_pulse`) で処理する (最小化しても止まらない)。
  プロファイルはバックエンドが持ち、変わるたびに `profile` の通知で届く。保存先があれば、変えた後に自動で保存する。
-->
<script lang="ts">
import { FaceTracker, type TrackingFrame } from "@ugoe/face-tracking";
import type { MotionPriority, PoseGroupInfo } from "@ugoe/player";
import {
  AlertDialog,
  announce,
  Button,
  Icon,
  IconButton,
  Menu,
  type MenuEntry,
  Notice,
  Section,
  Slider,
  Toggle,
  Toolbar,
} from "@ugoe/ui";
import { onMount, tick, untrack } from "svelte";
import ApiApproval from "./ApiApproval.svelte";
import ApiPanel from "./ApiPanel.svelte";
import HotkeysPanel from "./HotkeysPanel.svelte";
import InputPanel from "./InputPanel.svelte";
import LoadPanel from "./LoadPanel.svelte";
import {
  type ApiPendingClient,
  type ApiPermission,
  type ApiView,
  EMPTY_API,
  pendingToShow,
} from "./lib/api";
import { describeError, isSceneError } from "./lib/errors";
import type {
  FileKind,
  MonitorMessage,
  OutputState,
  ParameterInfo,
  ReceiverFailure,
  StatusMessage,
  StudioHost,
  StudioScene,
} from "./lib/host";
import { studioHost } from "./lib/host";
import { t } from "./lib/i18n";
import {
  type AnimationView,
  DEFAULT_INPUT,
  DEFAULT_MOTION,
  EMPTY_ANIMATION,
  type Hotkey,
  type HotkeyProblem,
  type InputSettings,
  type MotionSettings,
} from "./lib/inputs";
import { sceneLabel } from "./lib/labels";
import { latestOnly } from "./lib/latest";
import { LiveSceneWatch } from "./lib/liveScene";
import { Microphone } from "./lib/microphone.svelte";
import { PendingEdit } from "./lib/pendingEdit.svelte";
import { fileName, type ProfileView, type SceneSettings, withBackgroundKind } from "./lib/profile";
import { Pulse } from "./lib/pulse";
import { SceneLoading } from "./lib/sceneLoading.svelte";
import {
  type OutputSizePreset,
  parseOutputSize,
  parseTickRate,
  presetOf,
  presetSize,
  type TickRate,
} from "./lib/settings";
import { type PanelId, revealPanel, statusBadges } from "./lib/status";
import { DEFAULT_CAMERA_PROFILE, type VirtualCameraStatus } from "./lib/virtualCamera";
import MotionPanel from "./MotionPanel.svelte";
import MotionSettingsPanel from "./MotionSettingsPanel.svelte";
import OutputPanel from "./OutputPanel.svelte";
import PosePanel from "./PosePanel.svelte";
import ReceiverFailuresNotices from "./ReceiverFailuresNotices.svelte";
import SceneEditor from "./SceneEditor.svelte";
import ScenesPanel from "./ScenesPanel.svelte";
import SettingsDialog from "./SettingsDialog.svelte";
import SourcePanel from "./SourcePanel.svelte";
import StatusBadge from "./StatusBadge.svelte";
import VirtualCameraPanel from "./VirtualCameraPanel.svelte";

/** スライダーを動かした後、指揮役からの値で表示を上書きしない時間 (ミリ秒)。 */
const EDIT_HOLD_MS = 500;
/** プロファイルを変えてから自動で保存するまでの時間 (ミリ秒。続けて変えている間は待つ)。 */
const AUTOSAVE_MS = 800;

let host = $state<StudioHost | null>(null);
let scene = $state<StudioScene | null>(null);
/** 配信しているモデルがライティングを受けるか。 */
let usesLighting = $state(false);
/** 配信しているモデル (と付けているモジュール) のポーズのグループ (T0842)。 */
let poseGroups = $state<PoseGroupInfo[]>([]);
/** グループごとに指揮役が今表示している要素 (モーションが切り替えた要素を含む。T0848)。 */
let poseShown = $state<string[]>([]);
let profile = $state<ProfileView | null>(null);
/** 編集しているシーンの ID。 */
let selected = $state<string | null>(null);
/** 送っている途中のシーンの編集 (届いたプロファイルより先に画面に出す)。 */
let editing = $state<SceneSettings | null>(null);
/** 保存していないプロファイルを捨てる前の確かめ (確かめた後にすること)。 */
let discarding = $state<(() => void) | null>(null);
/** 直近のシーンのエラー (モデルが見つからないなど。選び直すボタンを出す)。 */
let sceneError = $state<{ id: string; message: string; missingModel: boolean } | null>(null);
let parameters = $state<ParameterInfo[]>([]);
let values = $state<Record<string, number>>({});
let output = $state<OutputState | null>(null);
/** 送っている途中の刻みの速さ (プロファイルに持つ。T0827)。 */
/** シーンの切り替えの読み込み中 (T0837。ホットキー・外部 API が始めたものはバックエンドの通知で、T0841)。 */
const sceneLoading = new SceneLoading();
const tickRateEdit = new PendingEdit<TickRate>((rate) => run((h) => h.setTickRate(rate)));
const tickRate = $derived(tickRateEdit.current(parseTickRate(profile?.profile.output.tickRate)));
/** 最新の値と負荷 (負荷の表示に使う)。 */
let status = $state<StatusMessage | null>(null);
/** 受け手が今のシーンで読めなかったこと (T0825。シーンが変わると空になって届く)。 */
let loadFailures = $state<ReceiverFailure[]>([]);
/** 出力ウィンドウに当てたプリセット (プロファイルの値と比べて、変わったら当て直す)。 */
let appliedSize: OutputSizePreset | null = null;
let showInternal = $state(false);
/** 仮想カメラの状態 (VirtualCameraPanel.svelte)。 */
let camera = $state<VirtualCameraStatus | null>(null);
let error = $state<string | null>(null);
let settingsOpen = $state(false);
/** 手で動かしたパラメータと、動かした時刻。 */
const edited = new Map<string, number>();

// --- 入力 (T0803) ---

let animation = $state<AnimationView>(EMPTY_ANIMATION);
let hotkeyProblems = $state<[number, HotkeyProblem][]>([]);
let wayland = $state(false);
/** ホットキーの操作を行えなかった理由。 */
let hotkeyError = $state<string | null>(null);

// --- 操作 API (T0805) ---

let api = $state<ApiView>(EMPTY_API);
/** 承認したばかりのクライアント (取り消せる知らせを出す。誤って承認したときのため)。 */
let approved = $state<{ id: string; name: string; permission: ApiPermission } | null>(null);
/** 承認の知らせで「後で」を押したクライアント。 */
let postponed = $state<ReadonlySet<string>>(new Set());
const approvals = $derived(pendingToShow(api, postponed));
/** 承認の取り消しの知らせ (承認した一覧に載っている間だけ)。 */
const approvedClient = $derived(
  approved !== null && api.clients.some((c) => c.id === approved?.id) ? approved : null,
);
/** カメラとマイクの処理を回す刻み (バックエンドの刻みのスレッドから届く)。 */
const pulse = new Pulse();
const sendTracking = latestOnly((frame: TrackingFrame | null) =>
  host === null ? Promise.resolve() : host.setTracking(frame),
);
const tracker = new FaceTracker(
  {
    setTracking: sendTracking,
    setAutoEyeBlink: (enabled) => void run((h) => h.setAutoEyeBlink(enabled)),
  },
  { schedule: pulse.schedule },
);
window.__UGOE_TRACKER__ = tracker;
const microphone = new Microphone(
  latestOnly((level: number | null) =>
    host === null ? Promise.resolve() : host.setLipSyncLevel(level),
  ),
  () => inputSettings,
);
/** 道具に当てた入力の設定 (プロファイルの値が変わったときだけ当て直す)。 */
let appliedInput = "";

const inputSettings = $derived<InputSettings>(profile?.profile.input ?? DEFAULT_INPUT);
const motionSettings = $derived<MotionSettings>(profile?.profile.motion ?? DEFAULT_MOTION);
const hotkeys = $derived<Hotkey[]>(profile?.profile.hotkeys ?? []);
/** シーンの ID → 切り替えのホットキー (シーンの一覧に並べて示す。同じシーンに複数あれば先のもの)。 */
const sceneHotkeys = $derived.by(() => {
  const out = new Map<string, string>();
  for (const hotkey of hotkeys) {
    if (hotkey.action.kind === "scene" && hotkey.keys !== "" && !out.has(hotkey.action.scene))
      out.set(hotkey.action.scene, hotkey.keys);
  }
  return out;
});
const outputSize = $derived(parseOutputSize(profile?.profile.output.size));
/** 出力ウィンドウに出している背景 (配信しているシーンのもの。配信していなければ `null`)。 */
const liveBackground = $derived(profile?.live == null ? null : (scene?.background ?? null));
/** 右の列の設定のパネルが開いているか (「いまの状態」の印から開いて送る。T0845)。 */
const panelsOpen = $state<Record<PanelId, boolean>>({
  inputs: true,
  output: true,
  source: true,
  hotkeys: true,
  api: true,
  load: true,
});

/** 印を押したとき: そのパネルを開いて送り、見出しにフォーカスを移す。 */
function openPanel(panel: PanelId) {
  void revealPanel(document, panel, async (p) => {
    panelsOpen[p] = true;
    await tick();
  });
}

/** 「いまの状態」の印 (lib/status.ts)。 */
const badges = $derived(
  statusBadges({
    tracking: {
      status: tracker.status,
      faceFound: tracker.faceFound,
      fps: tracker.fps,
      paused: animation.trackingPaused,
    },
    lipSync: { running: microphone.running, paused: animation.lipSyncPaused },
    hotkeys: {
      count: hotkeys.filter((h) => h.keys !== "").length,
      problems: hotkeyProblems.length,
      wayland,
    },
    output:
      output === null
        ? null
        : {
            window: output,
            scaled: outputSize !== "free" && presetOf(output.width, output.height) !== outputSize,
            background: liveBackground,
          },
    load: status,
    api,
  }),
);

const shownParameters = $derived(parameters.filter((p) => showInternal || !p.internal));
/** ブラウザソースの手順で案内する大きさ (出力ウィンドウの大きさのプリセット)。 */
const sourceSize = $derived.by(() => {
  const size = presetSize(outputSize);
  return size === null ? null : { width: size[0], height: size[1] };
});
const scenes = $derived(profile?.profile.scenes ?? []);
const sceneChoices = $derived(scenes.map((s, i) => ({ id: s.id, label: sceneLabel(s, i) })));
/** 編集しているシーンの、名前が空のときの表示名 (「シーン 1」など)。 */
const selectedDefaultName = $derived.by(() => {
  const index = scenes.findIndex((s) => s.id === selected);
  return index < 0 ? "" : t("scenes.defaultName", { n: index + 1 });
});
const selectedScene = $derived.by(() => {
  const found = scenes.find((s) => s.id === selected) ?? null;
  return editing !== null && editing.id === found?.id ? editing : found;
});
const liveName = $derived.by(() => {
  const index = scenes.findIndex((s) => s.id === profile?.live);
  const live = scenes[index];
  return live === undefined ? "—" : sceneLabel(live, index);
});
// 出しているシーンが変わったら読み上げる (クリック・ホットキー・API のどれでも。最初に開いたときは読まない。
// docs/ui.md#状態の変化の知らせ)。名前だけ変えたときは読まない
const liveWatch = new LiveSceneWatch();
$effect(() => {
  const live = profile?.live ?? null;
  const name = liveName;
  untrack(() => {
    if (liveWatch.changed(live)) announce(t("scenes.liveAnnounce", { name }));
  });
});
const profileName = $derived(
  profile?.path == null ? t("profile.untitled") : fileName(profile.path),
);
const profileEntries = $derived<MenuEntry[]>([
  { label: t("profile.new"), onselect: () => confirmDiscard(newProfile) },
  { label: t("profile.open"), onselect: () => confirmDiscard(openProfile) },
  { kind: "separator" },
  { label: t("profile.save"), shortcut: "Mod+S", onselect: () => void saveProfile(false) },
  { label: t("profile.saveAs"), onselect: () => void saveProfile(true) },
  { kind: "separator" },
  {
    kind: "submenu",
    label: t("profile.recent"),
    disabled: (profile?.recent.length ?? 0) === 0,
    // 名前はファイル名、説明の行に場所の全体を出す (同じ名前のファイルを見分ける)
    entries: (profile?.recent ?? []).map((path) => ({
      label: fileName(path),
      description: path,
      onselect: () => confirmDiscard(() => void openProfileAt(path)),
    })),
  },
]);
function onMonitor(message: MonitorMessage) {
  switch (message.kind) {
    case "scene":
      scene = message.scene;
      usesLighting = message.usesLighting;
      poseGroups = message.poseGroups;
      poseShown = message.poseShown;
      parameters = message.parameters;
      values = Object.fromEntries(message.parameters.map((p) => [p.id, p.default]));
      edited.clear();
      break;
    case "pose":
      poseShown = message.shown;
      break;
    case "status": {
      status = message;
      const ids = scene?.parameterIds ?? [];
      const now = performance.now();
      ids.forEach((id, i) => {
        const v = message.values[i];
        const at = edited.get(id);
        if (v === undefined || (at !== undefined && now - at < EDIT_HOLD_MS)) return;
        if (values[id] !== v) values[id] = v;
      });
      break;
    }
    case "output": {
      const { kind: _, ...state } = message;
      output = state;
      break;
    }
    case "profile": {
      const { kind: _, ...view } = message;
      onProfile(view);
      break;
    }
    case "animation": {
      const { kind: _, ...view } = message;
      animation = view;
      break;
    }
    case "hotkeys":
      hotkeyProblems = message.problems;
      wayland = message.wayland;
      break;
    case "hotkey":
      if (message.action.kind === "trackingCalibrate") tracker.calibrate();
      if (message.error !== null) hotkeyError = describeError(message.error);
      break;
    case "virtualCamera": {
      const { kind: _, ...state } = message;
      camera = state;
      break;
    }
    case "api": {
      const { kind: _, ...view } = message;
      api = view;
      break;
    }
    case "loadFailures":
      loadFailures = message.failures;
      break;
    case "sceneLoading":
      sceneLoading.notice(message.scene);
      break;
  }
}

/** プロファイルの入力の設定を、トラッカーとマイクに当てる (変わったときだけ)。 */
function applyInput(input: InputSettings) {
  const key = JSON.stringify(input);
  if (key === appliedInput) return;
  appliedInput = key;
  tracker.smoothing = input.smoothing;
  tracker.swapSides = input.swapSides;
  if (tracker.trackBody !== input.trackBody) tracker.setTrackBody(input.trackBody);
  if (tracker.stopEyeBlink !== input.stopEyeBlink) tracker.setStopEyeBlink(input.stopEyeBlink);
  const camera = input.camera;
  if (camera !== null && !tracker.running) {
    void tracker.refreshCameras().then(() => {
      const found =
        tracker.cameras.find((c) => c.id === camera.id) ??
        tracker.cameras.find((c) => c.label === camera.label);
      if (found !== undefined && !tracker.running) tracker.cameraId = found.id;
    });
  }
  if (!microphone.running) microphone.choose(input.microphone);
}

function onExpression(op: "set" | "add" | "remove" | "clear", name?: string) {
  void run((h) => {
    if (op === "clear") return h.clearExpressions();
    if (name === undefined) return Promise.resolve();
    if (op === "set") return h.setExpression(name);
    if (op === "add") return h.addExpression(name);
    return h.removeExpression(name);
  });
}

async function playMotion(name: string, priority: MotionPriority) {
  const started = await run((h) => h.playMotion(name, priority));
  if (started === false) hotkeyError = t("errors.animation.motion_rejected");
}

function onProfile(view: ProfileView) {
  profile = view;
  applyInput(view.profile.input);
  const ids = view.profile.scenes.map((s) => s.id);
  if (selected === null || !ids.includes(selected)) selected = view.live ?? ids[0] ?? null;
  if (sceneError !== null && !ids.includes(sceneError.id)) sceneError = null;
  // 別のプロファイルを開いたら、出力ウィンドウの大きさを合わせる。
  const preset = parseOutputSize(view.profile.output.size);
  if (preset !== appliedSize) void applyWindowSize(preset);
  scheduleAutosave();
}

// --- プロファイル ---

let autosaveTimer: ReturnType<typeof setTimeout> | undefined;

/** 保存先があり、変えていれば、少し待ってから保存する。 */
function scheduleAutosave() {
  clearTimeout(autosaveTimer);
  if (profile === null || profile.path === null || !profile.dirty) return;
  autosaveTimer = setTimeout(() => void run((h) => h.saveProfile()), AUTOSAVE_MS);
}

/** 保存先の無いプロファイルを変えていれば、捨ててよいか確かめてから `action` を行う。 */
function confirmDiscard(action: () => void) {
  if (profile !== null && profile.path === null && profile.dirty) discarding = action;
  else action();
}

async function newProfile() {
  sceneError = null;
  await run((h) => h.newProfile());
}

async function openProfile() {
  const path = await run((h) => h.pickFile("profile", t("file.profileFilter")));
  if (path !== null && path !== undefined) await openProfileAt(path);
}

async function openProfileAt(path: string) {
  error = null;
  sceneError = null;
  try {
    const h = host;
    if (h !== null) await sceneLoading.track(null, () => h.openProfile(path));
  } catch (e) {
    reportError(e, profile?.profile.activeScene ?? null);
  }
}

/** 保存する。`saveAs` か保存先が無ければ、保存先を選ばせる。 */
async function saveProfile(saveAs: boolean) {
  let path: string | undefined;
  if (saveAs || profile?.path == null) {
    const picked = await run((h) => h.pickSavePath(t("file.profileFilter"), t("file.profileName")));
    if (picked === null || picked === undefined) return;
    path = picked;
  }
  const saved = await run(async (h) => {
    await h.saveProfile(path);
    return true;
  });
  // 保存の印 (「保存済み」) は文が変わるだけなので、自分で保存したときは読み上げでも知らせる (自動の保存は知らせない)
  if (saved) announce(t("profile.savedAnnounce"));
}

/**
 * エラーを出す。シーンを出せないエラー (コードが `scene.` で始まる) は、そのシーンを選んで知らせる
 * (モデルが見つからなければ、選び直すボタンを出す)。プロファイルのエラーなどは上の帯に出す。
 */
function reportError(e: unknown, id: string | null) {
  const message = describeError(e);
  if (isSceneError(e) && id !== null) {
    sceneError = {
      id,
      message,
      missingModel: e.code === "scene.model_missing",
    };
    selected = id;
  } else {
    error = message;
  }
}

// --- シーン ---

async function sceneOp(id: string | null, action: (host: StudioHost) => Promise<unknown>) {
  if (host === null) return;
  try {
    await action(host);
    if (sceneError !== null && sceneError.id === id) sceneError = null;
  } catch (e) {
    reportError(e, id);
  }
}

function activateScene(id: string) {
  selected = id;
  void sceneOp(id, (h) => sceneLoading.track(id, () => h.activateScene(id)));
}

async function addScene() {
  const id = await run((h) => h.addScene(t("scenes.defaultName", { n: scenes.length + 1 })));
  if (id !== undefined) selected = id;
}

async function duplicateScene(id: string) {
  const index = scenes.findIndex((s) => s.id === id);
  const source = scenes[index];
  const name = t("scenes.copyName", {
    name: source === undefined ? "" : sceneLabel(source, index),
  });
  const copy = await run((h) => h.duplicateScene(id, name));
  if (copy !== undefined) selected = copy;
}

/** 送っていないシーンの編集 (送っている間に来た最後の 1 つ)。 */
let queued: SceneSettings | null = null;
let sending = false;

/** シーンの編集を送る。送っている間の編集はまとめて、最後のものだけを次に送る (スライダーを動かしている間)。 */
async function updateScene(next: SceneSettings) {
  editing = next;
  queued = next;
  if (sending) return;
  sending = true;
  while (queued !== null) {
    const scene = queued;
    queued = null;
    // 出しているシーンの編集は、読み直すことがある (モデル・モジュール・背景の画像を替えた)。
    // どのシーンも出していない (新しいプロファイル) ときにモデルを選んだら、そのシーンを出す
    // (「シーンの編集」から選んでも、出力ウィンドウにモデルが出るように。T0818)。
    const start = profile?.live == null && scene.model !== null;
    await sceneOp(scene.id, (h) =>
      profile?.live === scene.id || start
        ? sceneLoading.track(scene.id, async () => {
            await h.updateScene(scene);
            if (start) await h.activateScene(scene.id);
          })
        : h.updateScene(scene),
    );
  }
  sending = false;
  editing = null;
}

/**
 * 配信しているシーンの背景をクロマキーにする (透過が効かない注意のボタン、T0849)。シーンの編集の背景を替えるのと同じに送る
 * (保存され、戻すときは「シーンの編集」の背景で透過を選ぶ)。
 */
function chromaLiveBackground() {
  const index = scenes.findIndex((s) => s.id === profile?.live);
  const found = scenes[index];
  if (found === undefined) return;
  const current = editing !== null && editing.id === found.id ? editing : found;
  const next = structuredClone($state.snapshot(current)) as SceneSettings;
  next.background = withBackgroundKind(next.background, "chroma");
  void updateScene(next);
}

async function pickFile(kind: FileKind): Promise<string | null> {
  const filters = {
    model: "file.filter",
    module: "file.moduleFilter",
    image: "file.imageFilter",
    profile: "file.profileFilter",
  } as const;
  return (await run((h) => h.pickFile(kind, t(filters[kind])))) ?? null;
}

/** ツールバーの「モデルを選ぶ」: 編集しているシーン (無ければ新しいシーン) のモデルにして、そのシーンに切り替える。 */
async function chooseModel() {
  const path = await pickFile("model");
  if (path === null) return;
  error = null;
  let target = selectedScene;
  if (target === null) {
    const id = await run((h) => h.addScene(t("scenes.defaultName", { n: scenes.length + 1 })));
    if (id === undefined) return;
    selected = id;
    target = await nextScene(id);
  }
  if (target === null) return;
  const next = structuredClone($state.snapshot(target)) as SceneSettings;
  if (next.model !== path) next.modules = [];
  next.model = path;
  const id = next.id;
  await sceneOp(id, (h) =>
    sceneLoading.track(id, async () => {
      await h.updateScene(next);
      if (profile?.live !== id) await h.activateScene(id);
    }),
  );
  // 開いたら出力ウィンドウを出す (配信ソフトに取り込ませるため)。
  if (sceneError === null && output !== null && !output.visible) await setOutputVisible(true);
}

/** 加えたばかりのシーン (プロファイルの通知が届くのを待つ)。 */
async function nextScene(id: string): Promise<SceneSettings | null> {
  for (let i = 0; i < 20; i++) {
    const found = profile?.profile.scenes.find((s) => s.id === id);
    if (found !== undefined) return $state.snapshot(found) as SceneSettings;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  return null;
}

async function chooseModelAgain() {
  const id = sceneError?.id;
  if (id === undefined) return;
  selected = id;
  await chooseModel();
}

async function run<T>(action: (host: StudioHost) => Promise<T>): Promise<T | undefined> {
  if (host === null) return undefined;
  try {
    return await action(host);
  } catch (e) {
    error = describeError(e);
    return undefined;
  }
}

async function setOutputVisible(visible: boolean) {
  output = (await run((h) => h.setOutputVisible(visible))) ?? output;
}

async function setOutputOnTop(onTop: boolean) {
  output = (await run((h) => h.setOutputOnTop(onTop))) ?? output;
}

async function applyOutputSize(preset: OutputSizePreset) {
  await run((h) => h.setOutputPreset(preset));
  await applyWindowSize(preset);
}

/** 出力ウィンドウをプリセットの大きさにする (自由なら今の大きさのまま)。 */
async function applyWindowSize(preset: OutputSizePreset) {
  appliedSize = preset;
  const size = presetSize(preset);
  if (output === null || size === null || (output.width === size[0] && output.height === size[1]))
    return;
  output = (await run((h) => h.setOutputSize(size[0], size[1]))) ?? output;
}

function changeTickRate(rate: TickRate) {
  void tickRateEdit.submit(rate);
}

/** パラメータのスライダーの刻み。 */
function parameterStep(p: ParameterInfo): number {
  return (p.max - p.min) / 200;
}

async function approveClient(client: ApiPendingClient, permission: ApiPermission) {
  if (host === null) return;
  try {
    // 承認したクライアントは新しい ID を持つ (承認を待つ間の ID では一覧に無く、知らせが出なかった。T0919)
    const id = await host.approveApiClient(client.id, permission);
    approved = { id, name: client.name, permission };
  } catch (e) {
    error = describeError(e);
  }
}

/** 承認したばかりのクライアントの承認を取り消す (知らせの「取り消す」)。 */
function undoApproval() {
  const client = approved;
  approved = null;
  if (client !== null) void run((h) => h.revokeApiClient(client.id));
}

function setParameter(id: string, value: number) {
  edited.set(id, performance.now());
  values[id] = value;
  void run((h) => h.setParameter(id, value));
}

async function resetParameters() {
  edited.clear();
  await run((h) => h.resetParameters());
}

/** マイクの処理を刻みから外す。 */
let stopMicrophoneSteps: (() => void) | null = null;

onMount(() => {
  void (async () => {
    try {
      host = await studioHost();
    } catch (e) {
      error = describeError(e);
      return;
    }
    output = (await run((h) => h.outputState())) ?? null;
    camera = (await run((h) => h.virtualCameraState())) ?? null;
    await run((h) => h.subscribeStatus(onMonitor));
    // 入力の処理はバックエンドの刻みで回す (受け取れなければ requestAnimationFrame のまま)。
    try {
      await host.subscribePulse(() => pulse.beat(performance.now()));
      pulse.connect();
    } catch {
      // 刻みを受け取れない口 (古いブリッジなど)。
    }
    stopMicrophoneSteps = pulse.schedule((now) => microphone.step(now));
    // 最後に使ったプロファイルを開く (起動して最初の 1 回だけ。開けなければ新しいプロファイルのまま)。
    try {
      const h = host;
      await sceneLoading.track(null, () => h.restoreProfile());
    } catch (e) {
      reportError(e, profile?.profile.activeScene ?? null);
    }
  })();
  const onKey = (event: KeyboardEvent) => {
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void saveProfile(false);
    }
  };
  window.addEventListener("keydown", onKey);
  return () => {
    window.removeEventListener("keydown", onKey);
    clearTimeout(autosaveTimer);
    stopMicrophoneSteps?.();
    microphone.stop();
    tracker.stop();
  };
});
</script>

<div class="studio">
  <header class="studio__toolbar">
    <!-- 画面の名前 (見出しの移動の起点。見た目にはツールバーがあるので隠す) -->
    <h1 class="ugoe-visually-hidden">{t("app.title")}</h1>
    <Menu label={t("toolbar.profile")} entries={profileEntries} disabled={host === null} />
    <span class="studio__profile">
      <!-- 狭い幅では省略するので、全体 (保存先の場所) を title に出す -->
      <span class="studio__profile-name" title={profile?.path ?? profileName}>{profileName}</span>
      {#if profile !== null}
        <!-- 一度も保存していないプロファイルは、変えていなくても「未保存」(保存済みに見せない。T0818) -->
        <span class="studio__saved" class:studio__saved--dirty={profile.dirty}>
          {profile.dirty || profile.path === null ? t("profile.unsaved") : t("profile.saved")}
        </span>
      {/if}
    </span>
    <Toolbar label={t("toolbar.label")}>
      <Button variant="primary" onclick={chooseModel} disabled={host === null}>{t("toolbar.open")}</Button>
      <Button
        pressed={output?.visible ?? false}
        disabled={output === null}
        onclick={() => setOutputVisible(!(output?.visible ?? false))}
      >
        {t("toolbar.output")}
      </Button>
    </Toolbar>
    <div class="studio__spacer"></div>
    <IconButton label={t("toolbar.settings")} onclick={() => (settingsOpen = true)}>
      <Icon name="settings" />
    </IconButton>
  </header>

  {#if error !== null}
    <Notice tone="danger" ondismiss={() => (error = null)}>{error}</Notice>
  {/if}
  {#if profile !== null && profile.path === null && profile.dirty}
    <Notice tone="warning">
      {t("profile.unsavedHint")}
      {#snippet actions()}
        <Button small onclick={() => saveProfile(true)}>{t("profile.saveAs")}</Button>
      {/snippet}
    </Notice>
  {/if}

  {#if hotkeyError !== null}
    <Notice tone="warning" ondismiss={() => (hotkeyError = null)}><span class="ugoe-selectable">{hotkeyError}</span></Notice>
  {/if}

  {#each approvals as client (client.id)}
    <ApiApproval
      {client}
      disabled={host === null}
      onapprove={(permission) => void approveClient(client, permission)}
      ondeny={() => void run((h) => h.denyApiClient(client.id))}
      onlater={() => (postponed = new Set([...postponed, client.id]))}
    />
  {/each}

  {#if approvedClient !== null}
    <Notice tone="success" ondismiss={() => (approved = null)}>
      {t("api.approved", {
        name: approvedClient.name,
        permission: t(approvedClient.permission === "control" ? "api.permissionControl" : "api.permissionRead"),
      })}
      {#snippet actions()}
        <Button small onclick={undoApproval}>{t("api.undoApproval")}</Button>
      {/snippet}
    </Notice>
  {/if}

  {#if sceneError !== null}
    <Notice tone="danger" ondismiss={() => (sceneError = null)}>
      {sceneError.message}
      {#snippet actions()}
        {#if sceneError?.missingModel}
          <Button small onclick={chooseModelAgain}>{t("errors.chooseAgain")}</Button>
        {/if}
      {/snippet}
    </Notice>
  {/if}

  <ReceiverFailuresNotices
    failures={loadFailures}
    {scene}
    ondismiss={(label) => (loadFailures = loadFailures.filter((f) => f.label !== label))}
  />

  <main class="studio__main">
    <ScenesPanel
      {scenes}
      live={profile?.live ?? null}
      {selected}
      loading={sceneLoading.active ? { scene: sceneLoading.scene } : null}
      disabled={host === null}
      hotkeys={sceneHotkeys}
      onactivate={activateScene}
      onadd={addScene}
      onduplicate={duplicateScene}
      onmove={(id, index) => void run((h) => h.moveScene(id, index))}
      onremove={(id) => void run((h) => h.removeScene(id))}
    />

    <div class="studio__center">
      <section class="studio__now" aria-labelledby="state-heading">
        <h2 id="state-heading">{t("state.heading")}</h2>
        <ul class="studio__status" aria-label={t("status.label")}>
          {#each badges as view (view.id)}
            <li>
              <StatusBadge {view} onopen={() => openPanel(view.panel)}>
                {#if view.id === "lipSync" && view.tone === "ok"}
                  <meter min="0" max="1" value={microphone.level} aria-label={t("status.meter")}></meter>
                {/if}
              </StatusBadge>
            </li>
          {/each}
        </ul>
        {#if scene === null || scene.modelRevision === 0}
          <p class="studio__empty">{t("state.noModel")}</p>
          <div>
            <Button onclick={chooseModel} disabled={host === null}>{t("toolbar.open")}</Button>
          </div>
        {:else}
          <dl class="studio__facts">
            <dt>{t("state.scene")}</dt>
            <dd class="studio__live-name">{liveName}</dd>
            <dt>{t("state.model")}</dt>
            <dd>{scene.modelName}</dd>
            <dt>{t("state.motion")}</dt>
            <dd>{animation.current?.name ?? "—"}</dd>
            <dt>{t("state.expressions")}</dt>
            <dd>{animation.activeExpressions.length === 0 ? "—" : animation.activeExpressions.join(", ")}</dd>
          </dl>
        {/if}
      </section>

      <Section level={2} title={t("motion.heading")}>
        <MotionPanel
          {animation}
          settings={motionSettings}
          disabled={host === null || scene === null || scene.modelRevision === 0}
          onplay={(name, priority) => void playMotion(name, priority)}
          onstop={() => void run((h) => h.stopMotions())}
          onexpression={onExpression}
        />
      </Section>
      {#if poseGroups.length > 0}
        <Section level={2} title={t("poses.heading")}>
          <PosePanel
            groups={poseGroups}
            shown={poseShown}
            disabled={host === null}
            onchange={(part) => run((h) => h.showPose(part))}
          />
        </Section>
      {/if}
    </div>

    <aside class="studio__panels">
      {#if selectedScene !== null}
        <Section level={2} title={t("editor.heading")}>
          <SceneEditor
            scene={selectedScene}
            defaultName={selectedDefaultName}
            live={profile?.live === selectedScene.id}
            usesLighting={profile?.live === selectedScene.id ? usesLighting : null}
            disabled={host === null}
            onchange={updateScene}
            onactivate={() => selectedScene && activateScene(selectedScene.id)}
            onpick={pickFile}
          />
        </Section>
      {/if}
      <div data-panel="inputs">
        <Section level={2} title={t("inputs.heading")} bind:open={panelsOpen.inputs}>
          <InputPanel
            {tracker}
            {microphone}
            settings={inputSettings}
            trackingParameters={animation.trackingParameters}
            {values}
            trackingPaused={animation.trackingPaused}
            lipSyncPaused={animation.lipSyncPaused}
            disabled={host === null}
            onsettings={(settings) => run((h) => h.setInputSettings(settings))}
            ontrackingpause={(paused) => void run((h) => h.setTrackingPaused(paused))}
            onlipsyncpause={(paused) => void run((h) => h.setLipSyncPaused(paused))}
          />
        </Section>
      </div>
      <div data-panel="output">
        <Section level={2} title={t("output.heading")} bind:open={panelsOpen.output}>
          <OutputPanel
            {output}
            size={outputSize}
            background={liveBackground}
            onvisible={setOutputVisible}
            ontop={setOutputOnTop}
            onsize={applyOutputSize}
            onchroma={chromaLiveBackground}
          />
        </Section>
      </div>

      <div data-panel="source">
        <Section level={2} title={t("source.heading")} bind:open={panelsOpen.source}>
          <SourcePanel
            {api}
            size={sourceSize}
            disabled={host === null}
            onenabled={(enabled) => void run((h) => h.setSourceEnabled(enabled))}
            url={async () => {
              // i18n-ignore: 起こらない (host が無いときは部品を無効にする)
              if (host === null) throw new Error("no host");
              return host.sourceUrl();
            }}
            onregenerate={async () => (await run((h) => h.regenerateSourceToken())) ?? null}
          />
        </Section>
      </div>

      <Section level={2} title={t("virtualCamera.heading")}>
        <VirtualCameraPanel
          {host}
          status={camera}
          settings={profile?.profile.output.virtualCamera ?? DEFAULT_CAMERA_PROFILE}
        />
      </Section>

      <div data-panel="hotkeys">
        <Section level={2} title={t("hotkeys.heading")} bind:open={panelsOpen.hotkeys}>
          <HotkeysPanel
            {hotkeys}
            problems={hotkeyProblems}
            {wayland}
            scenes={sceneChoices}
            expressions={animation.expressions}
            motions={animation.motions.map((m) => m.name)}
            slots={animation.slots}
            poses={poseGroups.flatMap((g) => g.parts)}
            disabled={host === null}
            onchange={(list) => run((h) => h.setHotkeys(list))}
            onsuspend={(suspended) => void run((h) => h.suspendHotkeys(suspended))}
            onpickmodule={() => pickFile("module")}
            moduleSlot={async (path) => {
              // i18n-ignore: 起こらない (host が無いときは部品を無効にする)
              if (host === null) throw new Error("no host");
              try {
                return await host.moduleSlot(path);
              } catch (e) {
                throw new Error(describeError(e));
              }
            }}
          />
        </Section>
      </div>
      <Section level={2} title={t("motion.settingsHeading")}>
        <MotionSettingsPanel
          {animation}
          settings={motionSettings}
          disabled={host === null}
          onsettings={(settings) => run((h) => h.setMotionSettings(settings))}
        />
      </Section>

      <div data-panel="api">
        <Section level={2} title={t("api.heading")} bind:open={panelsOpen.api}>
          <ApiPanel
            {api}
            disabled={host === null}
            onenabled={(enabled) => {
              postponed = new Set();
              void run((h) => h.setApiEnabled(enabled));
            }}
            onport={(port) => void run((h) => h.setApiPort(port))}
            token={async () => {
              // i18n-ignore: 起こらない (host が無いときは部品を無効にする)
              if (host === null) throw new Error("no host");
              return host.apiToken();
            }}
            onregenerate={() => void run((h) => h.regenerateApiToken())}
            ondisconnect={(id) => void run((h) => h.disconnectApiClient(id))}
            onrevoke={(id) => void run((h) => h.revokeApiClient(id))}
            onreview={(id) => (postponed = new Set([...postponed].filter((p) => p !== id)))}
          />
        </Section>
      </div>

      <div data-panel="load">
        <Section level={2} title={t("load.heading")} bind:open={panelsOpen.load}>
          <LoadPanel {tickRate} {status} disabled={host === null} onTickRate={changeTickRate} />
        </Section>
      </div>

      <Section level={2} title={t("parameters.heading")}>
        {#snippet actions()}
          <Button small disabled={scene === null || scene.modelRevision === 0} onclick={resetParameters}>{t("parameters.reset")}</Button>
        {/snippet}
        {#if scene === null || scene.modelRevision === 0}
          <p class="studio__empty">{t("parameters.empty")}</p>
        {:else}
          <div class="studio__stack">
            <Toggle label={t("parameters.showInternal")} bind:checked={showInternal} />
            {#each shownParameters as p (p.id)}
              <Slider
                label={p.name || p.id}
                value={values[p.id] ?? p.default}
                min={p.min}
                max={p.max}
                step={parameterStep(p)}
                decimals={2}
                defaultValue={p.default}
                onchange={(v) => setParameter(p.id, v)}
              />
            {/each}
          </div>
        {/if}
      </Section>
    </aside>
  </main>
</div>

<SettingsDialog bind:open={settingsOpen} />

<AlertDialog
  open={discarding !== null}
  title={t("profile.discardTitle")}
  description={t("profile.discardBody")}
  confirmLabel={t("profile.discardConfirm")}
  danger
  onconfirm={() => {
    const action = discarding;
    discarding = null;
    action?.();
  }}
  oncancel={() => (discarding = null)}
/>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    background: var(--ugoe-color-bg);
    color: var(--ugoe-color-text);
    font-family: var(--ugoe-font-sans);
    font-size: var(--ugoe-font-size-md);
    line-height: var(--ugoe-line-height);
  }

  :global(#app) {
    height: 100%;
  }

  .studio {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .studio__toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--ugoe-space-3);
    padding: var(--ugoe-space-2) var(--ugoe-space-4);
    background: var(--ugoe-color-surface);
    border-bottom: 1px solid var(--ugoe-color-border);
  }

  .studio__spacer {
    flex: 1;
  }

  /*
    列の幅 (T0817): シーンの一覧は名前とホットキーが 1 行に収まる幅 (右のスクロールバーの分の 0.5rem を含む。T0970)、中央 (配信中に触るもの) を最も広く、右の設定のパネルは
    スライダーと数値の欄が並ぶ幅で止める。960px 以下では、シーンと中央を 2 列にして設定のパネルをその下に、640px 以下では 1 列に積み、
    画面の全体を送る (列ごとの送りをやめる)。
  */
  .studio__main {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(13.5rem, 17.5rem) minmax(20rem, 1fr) minmax(20rem, 28rem);
  }

  @media (max-width: 960px) {
    .studio__main {
      grid-template-columns: minmax(12.5rem, 16.5rem) minmax(0, 1fr);
      /* 縦に積んだ列を中身の高さにして、画面の全体を送る (auto の行は中のスクロールに合わせて縮み、列が重なっていた。T0907) */
      grid-auto-rows: max-content;
      /* 行末の列 (中央・設定のパネル) が右の padding でスクロールバーの分を空ける */
      overflow: auto;
    }

    .studio__panels {
      grid-column: 1 / -1;
      border-left: none;
      border-top: 1px solid var(--ugoe-color-border);
    }
  }

  @media (max-width: 640px) {
    .studio__main {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* 狭い幅でも名前を次の行へ送らず、省略して「プロファイル」と同じ行に置く (ヘッダーを 2 行に収める。T0817) */
  .studio__profile {
    display: flex;
    flex: 1 1 8rem;
    align-items: baseline;
    gap: var(--ugoe-space-2);
    min-width: 0;
    max-width: max-content;
  }

  .studio__profile-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--ugoe-font-weight-strong);
  }

  .studio__saved {
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
    white-space: nowrap;
  }

  .studio__saved--dirty {
    color: var(--ugoe-color-warning);
  }

  .studio__center {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-4);
    min-width: 0;
    /* 右の padding (space-5) はスクロールバーの分 (--ugoe-scrollbar-gap) 以上 */
    padding: var(--ugoe-space-4) var(--ugoe-space-5);
    overflow: auto;
  }

  .studio__now {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .studio__now h2 {
    margin: 0;
    font-size: var(--ugoe-font-size-lg);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .studio__status {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ugoe-space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .studio__status meter {
    width: 4rem;
  }

  .studio__facts {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    align-items: baseline;
    gap: var(--ugoe-space-1) var(--ugoe-space-4);
    margin: 0;
  }

  .studio__facts dt {
    color: var(--ugoe-color-text-muted);
  }

  .studio__facts dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .studio__live-name {
    font-size: var(--ugoe-font-size-xl);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .studio__panels {
    overflow: auto;
    /* 右はスクロールバーの分を空ける (docs/ui.md#レイアウト の「送り」) */
    padding: var(--ugoe-space-3) var(--ugoe-scrollbar-gap) var(--ugoe-space-3) var(--ugoe-space-3);
    background: var(--ugoe-color-surface);
    border-left: 1px solid var(--ugoe-color-border);
  }

  .studio__stack {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .studio__empty {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
