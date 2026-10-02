<!--
  操作ウィンドウ (docs/studio.md#操作ウィンドウ)。モデルは描かない (描画は出力ウィンドウに任せる)。
  様子はパラメータの値・刻みの番号・負荷の数値で示す (負荷と刻みの速さは LoadPanel.svelte)。左にシーンの一覧、
  右のパネルにシーンの編集 (T0802)。入力 (T0803) は後のタスク。
  プロファイルはバックエンドが持ち、変わるたびに `profile` の通知で届く。保存先があれば、変えた後に自動で保存する。
-->
<script lang="ts">
import {
  formatNumber,
  getLocalePreference,
  LANGUAGES,
  type LocalePreference,
  setLocale,
} from "@ugoe/i18n";
import {
  AlertDialog,
  Button,
  Menu,
  type MenuEntry,
  Notice,
  Section,
  Select,
  Slider,
  Toggle,
  Toolbar,
} from "@ugoe/ui";
import { onMount } from "svelte";
import LoadPanel from "./LoadPanel.svelte";
import { describeError } from "./lib/errors";
import type {
  FileKind,
  MonitorMessage,
  OutputState,
  ParameterInfo,
  StatusMessage,
  StudioHost,
  StudioScene,
} from "./lib/host";
import { studioHost } from "./lib/host";
import { t } from "./lib/i18n";
import { sceneLabel } from "./lib/labels";
import { fileName, isStudioError, type ProfileView, type SceneSettings } from "./lib/profile";
import {
  loadTickRate,
  OUTPUT_SIZE_PRESETS,
  type OutputSizePreset,
  parseOutputSize,
  presetOf,
  presetSize,
  saveLocalePreference,
  saveTickRate,
  type TickRate,
} from "./lib/settings";
import SceneEditor from "./SceneEditor.svelte";
import ScenesPanel from "./ScenesPanel.svelte";

/** スライダーを動かした後、指揮役からの値で表示を上書きしない時間 (ミリ秒)。 */
const EDIT_HOLD_MS = 500;
/** プロファイルを変えてから自動で保存するまでの時間 (ミリ秒。続けて変えている間は待つ)。 */
const AUTOSAVE_MS = 800;

let host = $state<StudioHost | null>(null);
let scene = $state<StudioScene | null>(null);
/** 配信しているモデルがライティングを受けるか。 */
let usesLighting = $state(false);
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
let seq = $state<number | null>(null);
let tickMs = $state<number | null>(null);
let receivers = $state(0);
let output = $state<OutputState | null>(null);
let tickRate = $state<TickRate>(loadTickRate());
/** 最新の値と負荷 (負荷の表示に使う)。 */
let status = $state<StatusMessage | null>(null);
/** 出力ウィンドウに当てたプリセット (プロファイルの値と比べて、変わったら当て直す)。 */
let appliedSize: OutputSizePreset | null = null;
let localePreference = $state<LocalePreference>(getLocalePreference());
let showInternal = $state(false);
let error = $state<string | null>(null);
/** 手で動かしたパラメータと、動かした時刻。 */
const edited = new Map<string, number>();

const shownParameters = $derived(parameters.filter((p) => showInternal || !p.internal));
const outputSize = $derived(parseOutputSize(profile?.profile.output.size));
const scenes = $derived(profile?.profile.scenes ?? []);
const selectedScene = $derived.by(() => {
  const found = scenes.find((s) => s.id === selected) ?? null;
  return editing !== null && editing.id === found?.id ? editing : found;
});
const liveName = $derived.by(() => {
  const index = scenes.findIndex((s) => s.id === profile?.live);
  const live = scenes[index];
  return live === undefined ? "—" : sceneLabel(live, index);
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
    entries: (profile?.recent ?? []).map((path) => ({
      label: path,
      onselect: () => confirmDiscard(() => void openProfileAt(path)),
    })),
  },
]);
const languageOptions = $derived([
  { value: "system" as LocalePreference, label: t("language.system") },
  ...LANGUAGES.map((l) => ({ value: l.tag as LocalePreference, label: l.name })),
]);
const sizeOptions = $derived(
  OUTPUT_SIZE_PRESETS.map((preset) => {
    const size = presetSize(preset);
    return {
      value: preset,
      label:
        size === null
          ? t("output.sizeFree")
          : t(size[1] > size[0] ? "output.sizePortrait" : "output.sizeValue", {
              width: size[0],
              height: size[1],
            }),
    };
  }),
);

function onMonitor(message: MonitorMessage) {
  switch (message.kind) {
    case "scene":
      scene = message.scene;
      usesLighting = message.usesLighting;
      parameters = message.parameters;
      values = Object.fromEntries(message.parameters.map((p) => [p.id, p.default]));
      edited.clear();
      break;
    case "status": {
      status = message;
      seq = message.seq;
      tickMs = message.tickMs;
      receivers = message.outputs;
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
  }
}

function onProfile(view: ProfileView) {
  profile = view;
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
    await host?.openProfile(path);
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
  await run((h) => h.saveProfile(path));
}

/** シーンを出せないエラー (stage.rs の `SceneError`)。ほかはプロファイルのエラー。 */
const SCENE_CODES = new Set(["unknownScene", "missing", "unreadable", "wrongType", "dressUp"]);

/**
 * エラーを出す。シーンを出せないエラーは、そのシーンを選んで知らせる (モデルが見つからなければ、選び直すボタンを出す)。
 * プロファイルのエラーは上の帯に出す。
 */
function reportError(e: unknown, id: string | null) {
  const message = describeError(e);
  const sceneScoped = isStudioError(e) && (SCENE_CODES.has(e.code) || "asset" in e);
  if (isStudioError(e) && sceneScoped && id !== null) {
    sceneError = {
      id,
      message,
      missingModel: e.code === "missing" && e.asset === "model",
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
  void sceneOp(id, (h) => h.activateScene(id));
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
    await sceneOp(scene.id, (h) => h.updateScene(scene));
  }
  sending = false;
  editing = null;
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
  await sceneOp(id, async (h) => {
    await h.updateScene(next);
    if (profile?.live !== id) await h.activateScene(id);
  });
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
    error = e instanceof Error ? e.message : String(e);
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

async function changeTickRate(rate: TickRate) {
  tickRate = rate;
  saveTickRate(rate);
  await run((h) => h.setTickRate(rate));
}

function changeLocale(value: LocalePreference) {
  localePreference = value;
  setLocale(value);
  saveLocalePreference(value);
}

/** パラメータのスライダーの刻み。 */
function parameterStep(p: ParameterInfo): number {
  return (p.max - p.min) / 200;
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

onMount(() => {
  void (async () => {
    try {
      host = await studioHost();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      return;
    }
    output = (await run((h) => h.outputState())) ?? null;
    await run((h) => h.setTickRate(tickRate));
    await run((h) => h.subscribeStatus(onMonitor));
    // 最後に使ったプロファイルを開く (起動して最初の 1 回だけ。開けなければ新しいプロファイルのまま)。
    try {
      await host.restoreProfile();
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
  };
});
</script>

<div class="studio">
  <header class="studio__toolbar">
    <Menu label={t("toolbar.profile")} entries={profileEntries} disabled={host === null} />
    <span class="studio__profile">
      <span class="studio__profile-name">{profileName}</span>
      {#if profile !== null}
        <span class="studio__saved" class:studio__saved--dirty={profile.dirty}>
          {profile.dirty ? t("profile.unsaved") : t("profile.saved")}
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
    {#if status !== null}
      <span class="studio__load">
        {t("load.summary", {
          tick: t("load.ms", { value: formatNumber(status.tickMs, { maximumFractionDigits: 2 }) }),
          rate: t("load.rateValue", { value: status.tickRate }),
        })}
      </span>
    {/if}
    <div class="studio__language">
      <Select
        label={t("language.label")}
        value={localePreference}
        options={languageOptions}
        onchange={changeLocale}
      />
    </div>
  </header>

  {#if error !== null}
    <p class="studio__error" role="alert">{error}</p>
  {/if}
  {#if profile !== null && profile.path === null && profile.dirty}
    <Notice tone="warning">
      {t("profile.unsavedHint")}
      {#snippet actions()}
        <Button small onclick={() => saveProfile(true)}>{t("profile.saveAs")}</Button>
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

  <main class="studio__main">
    <ScenesPanel
      {scenes}
      live={profile?.live ?? null}
      {selected}
      disabled={host === null}
      onactivate={activateScene}
      onadd={addScene}
      onduplicate={duplicateScene}
      onmove={(id, index) => void run((h) => h.moveScene(id, index))}
      onremove={(id) => void run((h) => h.removeScene(id))}
    />

    <section class="studio__state" aria-labelledby="state-heading">
      <h2 id="state-heading">{t("state.heading")}</h2>
      {#if scene === null || scene.modelRevision === 0}
        <p class="studio__empty">{t("state.noModel")}</p>
      {:else}
        <dl class="studio__facts">
          <dt>{t("state.scene")}</dt>
          <dd>{liveName}</dd>
          <dt>{t("state.model")}</dt>
          <dd>{scene.modelName}</dd>
          <dt>{t("state.frame")}</dt>
          <dd class="studio__number">{seq === null ? "—" : formatNumber(seq)}</dd>
          <dt>{t("state.tick")}</dt>
          <dd class="studio__number">
            {tickMs === null
              ? "—"
              : t("state.tickValue", { value: formatNumber(tickMs, { maximumFractionDigits: 2 }) })}
          </dd>
          <dt>{t("state.receivers")}</dt>
          <dd class="studio__number">{formatNumber(receivers)}</dd>
        </dl>
      {/if}
    </section>

    <aside class="studio__panels">
      {#if selectedScene !== null}
        <Section title={t("editor.heading")}>
          <SceneEditor
            scene={selectedScene}
            live={profile?.live === selectedScene.id}
            usesLighting={profile?.live === selectedScene.id ? usesLighting : null}
            disabled={host === null}
            onchange={updateScene}
            onactivate={() => selectedScene && activateScene(selectedScene.id)}
            onpick={pickFile}
          />
        </Section>
      {/if}
      <Section title={t("output.heading")}>
        <div class="studio__stack">
          <Toggle
            label={t("output.show")}
            checked={output?.visible ?? false}
            disabled={output === null}
            onchange={setOutputVisible}
          />
          <Toggle
            label={t("output.onTop")}
            checked={output?.onTop ?? false}
            disabled={output === null}
            onchange={setOutputOnTop}
          />
          <Select
            label={t("output.size")}
            value={outputSize}
            options={sizeOptions}
            disabled={output === null}
            onchange={applyOutputSize}
          />
          {#if output !== null}
            <p class="studio__hint">
              {t("output.current", { width: output.width, height: output.height })}
              {#if presetOf(output.width, output.height) !== outputSize && outputSize !== "free"}
                <br />{t("output.scaled")}
              {/if}
            </p>
          {/if}
          <p class="studio__hint">{t("output.moveHint")}</p>
        </div>
      </Section>

      <Section title={t("load.heading")}>
        <LoadPanel {tickRate} {status} disabled={host === null} onTickRate={changeTickRate} />
      </Section>

      <Section title={t("parameters.heading")}>
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
    align-items: center;
    gap: var(--ugoe-space-3);
    padding: var(--ugoe-space-2) var(--ugoe-space-4);
    background: var(--ugoe-color-surface);
    border-bottom: 1px solid var(--ugoe-color-border);
  }

  .studio__spacer {
    flex: 1;
  }

  .studio__load {
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
    font-variant-numeric: tabular-nums;
  }

  .studio__language {
    min-width: 12rem;
  }

  .studio__error {
    margin: 0;
    padding: var(--ugoe-space-2) var(--ugoe-space-4);
    background: var(--ugoe-color-danger-soft);
    color: var(--ugoe-color-text);
    border-bottom: 1px solid var(--ugoe-color-border);
  }

  .studio__main {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr) minmax(18rem, 26rem);
  }

  @media (max-width: 960px) {
    .studio__main {
      grid-template-columns: minmax(0, 1fr);
      overflow: auto;
    }
  }

  .studio__profile {
    display: flex;
    align-items: baseline;
    gap: var(--ugoe-space-2);
    min-width: 0;
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

  .studio__state {
    padding: var(--ugoe-space-4) var(--ugoe-space-5);
    overflow: auto;
  }

  .studio__state h2 {
    margin: 0 0 var(--ugoe-space-4);
    font-size: var(--ugoe-font-size-xl);
    font-weight: var(--ugoe-font-weight-strong);
  }

  .studio__facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--ugoe-space-2) var(--ugoe-space-5);
    margin: 0;
  }

  .studio__facts dt {
    color: var(--ugoe-color-text-muted);
  }

  .studio__facts dd {
    margin: 0;
  }

  .studio__number {
    font-variant-numeric: tabular-nums;
  }

  .studio__panels {
    overflow: auto;
    padding: var(--ugoe-space-3);
    background: var(--ugoe-color-surface);
    border-left: 1px solid var(--ugoe-color-border);
  }

  .studio__stack {
    display: flex;
    flex-direction: column;
    gap: var(--ugoe-space-3);
  }

  .studio__empty,
  .studio__hint {
    margin: 0;
    color: var(--ugoe-color-text-muted);
    font-size: var(--ugoe-font-size-sm);
  }
</style>
