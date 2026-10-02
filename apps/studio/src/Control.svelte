<!--
  操作ウィンドウ (docs/studio.md#操作ウィンドウ)。モデルは描かない (描画は出力ウィンドウに任せる)。
  様子はパラメータの値・刻みの番号・負荷の数値で示す。シーン (T0802)・入力 (T0803)・出力の詳細 (T0804) は後のタスク。
-->
<script lang="ts">
import {
  formatNumber,
  getLocalePreference,
  LANGUAGES,
  type LocalePreference,
  setLocale,
} from "@ugoe/i18n";
import type { SceneMessage } from "@ugoe/player";
import { Button, Section, Select, Slider, Toggle, Toolbar } from "@ugoe/ui";
import { onMount } from "svelte";
import type { MonitorMessage, OutputState, ParameterInfo, StudioHost } from "./lib/host";
import { studioHost } from "./lib/host";
import { t } from "./lib/i18n";
import {
  loadOutputSize,
  OUTPUT_SIZE_PRESETS,
  type OutputSizePreset,
  presetOf,
  presetSize,
  saveLocalePreference,
  saveOutputSize,
} from "./lib/settings";

/** スライダーを動かした後、指揮役からの値で表示を上書きしない時間 (ミリ秒)。 */
const EDIT_HOLD_MS = 500;

let host = $state<StudioHost | null>(null);
let scene = $state<SceneMessage | null>(null);
let parameters = $state<ParameterInfo[]>([]);
let values = $state<Record<string, number>>({});
let seq = $state<number | null>(null);
let tickMs = $state<number | null>(null);
let receivers = $state(0);
let output = $state<OutputState | null>(null);
let outputSize = $state<OutputSizePreset>(loadOutputSize());
let localePreference = $state<LocalePreference>(getLocalePreference());
let showInternal = $state(false);
let error = $state<string | null>(null);
/** 手で動かしたパラメータと、動かした時刻。 */
const edited = new Map<string, number>();

const shownParameters = $derived(parameters.filter((p) => showInternal || !p.internal));
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
      parameters = message.parameters;
      values = Object.fromEntries(message.parameters.map((p) => [p.id, p.default]));
      edited.clear();
      break;
    case "status": {
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
  }
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

async function openModel() {
  const path = await run((h) => h.pickModelPath(t("file.filter")));
  if (path === null || path === undefined) return;
  error = null;
  const name = path.split(/[\\/]/).pop() ?? path;
  try {
    await host?.openModel(path);
  } catch (e) {
    error = t("errors.open", { name, message: e instanceof Error ? e.message : String(e) });
    return;
  }
  // 開いたら出力ウィンドウを出す (配信ソフトに取り込ませるため)。
  if (output !== null && !output.visible) await setOutputVisible(true);
}

async function setOutputVisible(visible: boolean) {
  output = (await run((h) => h.setOutputVisible(visible))) ?? output;
}

async function setOutputOnTop(onTop: boolean) {
  output = (await run((h) => h.setOutputOnTop(onTop))) ?? output;
}

async function applyOutputSize(preset: OutputSizePreset) {
  outputSize = preset;
  saveOutputSize(preset);
  const size = presetSize(preset);
  if (size !== null) output = (await run((h) => h.setOutputSize(size[0], size[1]))) ?? output;
}

function changeLocale(value: LocalePreference) {
  localePreference = value;
  setLocale(value);
  saveLocalePreference(value);
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
    await run((h) => h.subscribeStatus(onMonitor));
    output = (await run((h) => h.outputState())) ?? null;
    // 覚えた大きさにする (自由なら今の大きさのまま)。
    const size = presetSize(outputSize);
    if (
      output !== null &&
      size !== null &&
      (output.width !== size[0] || output.height !== size[1])
    ) {
      output = (await run((h) => h.setOutputSize(size[0], size[1]))) ?? output;
    }
  })();
});
</script>

<div class="studio">
  <header class="studio__toolbar">
    <Toolbar label={t("toolbar.label")}>
      <Button variant="primary" onclick={openModel} disabled={host === null}>{t("toolbar.open")}</Button>
      <Button
        pressed={output?.visible ?? false}
        disabled={output === null}
        onclick={() => setOutputVisible(!(output?.visible ?? false))}
      >
        {t("toolbar.output")}
      </Button>
    </Toolbar>
    <div class="studio__spacer"></div>
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

  <main class="studio__main">
    <section class="studio__state" aria-labelledby="state-heading">
      <h2 id="state-heading">{t("state.heading")}</h2>
      {#if scene === null}
        <p class="studio__empty">{t("state.noModel")}</p>
      {:else}
        <dl class="studio__facts">
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

      <Section title={t("parameters.heading")}>
        {#snippet actions()}
          <Button small disabled={scene === null} onclick={resetParameters}>{t("parameters.reset")}</Button>
        {/snippet}
        {#if scene === null}
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
                step={(p.max - p.min) / 200}
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
    grid-template-columns: minmax(0, 1fr) minmax(18rem, 26rem);
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
